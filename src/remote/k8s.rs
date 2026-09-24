// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Kubernetes configuration sources (`k8s` feature).
//!
//! Two complementary adapters:
//!
//! - [`K8sMountedSource`]: reads a ConfigMap/Secret **mount volume**
//!   (`/var/run/secrets/...` style directory). kubelet publishes updates with
//!   the atomic-writer pattern: a `..data` symlink is atomically swapped to a
//!   fresh timestamped directory, so the source polls the symlink target and
//!   reloads exactly once per swap (never observing a half-updated tree).
//! - [`K8sApiSource`]: fetches a ConfigMap/Secret through the **Kubernetes
//!   REST API** (in-cluster service-account config by default). MVP skeleton:
//!   single-object GET with bearer auth; kind `Secret` values are
//!   base64-decoded per the API contract.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;

use crate::error::{ConfigError, ConfigResult};
use crate::loader::Format;
use crate::remote::circuit_breaker::CircuitBreaker;
use crate::remote::common::try_parse_value_with_format;
use crate::types::{AnnotatedValue, ConfigValue, SourceId};

const SOURCE_NAME: &str = "k8s";

/// Default service-account token path inside a pod.
pub const DEFAULT_SERVICE_ACCOUNT_TOKEN: &str =
    "/var/run/secrets/kubernetes.io/serviceaccount/token";

/// Default service-account CA bundle path inside a pod.
///
/// The in-cluster API server serves a cluster-specific TLS certificate that
/// is only trusted via this CA bundle; without loading it every in-cluster
/// request fails TLS verification.
pub const DEFAULT_SERVICE_ACCOUNT_CA: &str = "/var/run/secrets/kubernetes.io/serviceaccount/ca.crt";

/// Default connect timeout for Kubernetes API requests (10 seconds).
pub const DEFAULT_K8S_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Default overall request timeout for Kubernetes API requests (30 seconds).
pub const DEFAULT_K8S_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

// =============================================================================
// Mounted-volume source (ConfigMap/Secret as files)
// =============================================================================

/// Configuration source reading a ConfigMap/Secret **mount volume**.
///
/// The kubelet atomic-writer layout is honoured: when `..data` exists, its
/// symlink target identifies the current generation and is used as the change
/// marker (atomic swap semantics); plain directories without `..data` fall
/// back to content-based change detection.
pub struct K8sMountedSource {
    mount_path: PathBuf,
    format: Option<Format>,
    interval: Duration,
    /// Last observed `..data` symlink target (change marker).
    observed_target: std::sync::Mutex<Option<Arc<str>>>,
    /// Last observed content hash for symlink-less directories.
    observed_stamp: std::sync::Mutex<u64>,
    cached: ArcSwap<Option<Arc<AnnotatedValue>>>,
    source_id: SourceId,
}

impl K8sMountedSource {
    /// Create a source for the mounted volume at `mount_path`.
    pub fn new(mount_path: impl Into<PathBuf>) -> Self {
        let mount_path = mount_path.into();
        let source_id = SourceId::new(format!("{SOURCE_NAME}:mount:{}", mount_path.display()));
        Self {
            mount_path,
            format: None,
            interval: Duration::from_secs(10),
            observed_target: std::sync::Mutex::new(None),
            observed_stamp: std::sync::Mutex::new(0),
            cached: ArcSwap::new(Arc::new(None)),
            source_id,
        }
    }

    /// Pin a value format (default: per-file content sniffing).
    pub fn with_format(mut self, format: Format) -> Self {
        self.format = Some(format);
        self
    }

    /// Set the poll interval.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// The mounted directory.
    pub fn mount_path(&self) -> &Path {
        &self.mount_path
    }

    /// Current `..data` symlink target (the generation marker), if any.
    pub fn data_target(&self) -> Option<String> {
        std::fs::read_link(self.mount_path.join("..data"))
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
    }

    /// Reload when the volume generation changed (symlink swap).
    ///
    /// Returns the freshly loaded snapshot on change, `None` when the
    /// generation is unchanged. The first call always loads.
    pub fn reload_if_swapped(&self) -> ConfigResult<Option<AnnotatedValue>> {
        let target = self.data_target();
        let changed = match &target {
            Some(t) => {
                let stored = self
                    .observed_target
                    .try_lock()
                    .map_err(|_| self.lock_error())?;
                stored.as_deref() != Some(t.as_str())
            }
            None => {
                // No `..data` symlink: fall back to a cheap content stamp.
                let stamp = self.directory_stamp()?;
                let mut stored = self
                    .observed_stamp
                    .try_lock()
                    .map_err(|_| self.lock_error())?;
                if *stored == stamp && self.cached.load().is_some() {
                    return Ok(None);
                }
                *stored = stamp;
                true
            }
        };
        if !changed {
            return Ok(None);
        }
        if let Some(t) = target
            && let Ok(mut guard) = self.observed_target.try_lock()
        {
            *guard = Some(Arc::from(t.as_str()));
        }
        let value = self.read_volume()?;
        self.cached.store(Arc::new(Some(Arc::new(value.clone()))));
        Ok(Some(value))
    }

    /// Read every file in the mounted directory into a config map.
    ///
    /// `..`-prefixed entries (the atomic-writer internals) are skipped; each
    /// remaining file becomes a key named after the file, with the content
    /// parsed through the configured format (or content sniffing) and raw
    /// text as fallback.
    pub fn read_volume(&self) -> ConfigResult<AnnotatedValue> {
        let entries = std::fs::read_dir(&self.mount_path).map_err(io_err(
            self.mount_path.display().to_string(),
            "cannot read k8s mount volume".to_string(),
        ))?;

        let mut map = indexmap::IndexMap::new();
        for entry in entries {
            let entry = entry.map_err(|e| {
                io_err(
                    self.mount_path.display().to_string(),
                    "cannot read mount volume entry".to_string(),
                )(e)
            })?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("..") {
                // Atomic-writer internals (`..data`, `..2024_01_01_...`).
                continue;
            }
            // Volume entries are symlinks (each key points into the `..data`
            // timestamped directory), so links must be followed — but only
            // within the mount: a symlink escaping `mount_path` is not a
            // config key and is skipped.
            let Some(resolved) = self.entry_path_within_mount(entry.path()) else {
                continue;
            };
            let content = std::fs::read_to_string(&resolved).map_err(|e| {
                io_err(
                    entry.path().display().to_string(),
                    format!("cannot read mounted key '{name}'"),
                )(e)
            })?;
            let parsed = try_parse_value_with_format(&content, self.format, SOURCE_NAME)
                .unwrap_or_else(|| {
                    AnnotatedValue::new(
                        ConfigValue::String(content),
                        SourceId::new(SOURCE_NAME),
                        name.as_ref(),
                    )
                });
            map.insert(Arc::from(name.as_ref()), parsed);
        }

        let value = if map.is_empty() {
            ConfigValue::Null
        } else {
            ConfigValue::map(map.into_iter().collect())
        };
        Ok(AnnotatedValue::new(value, SourceId::new(SOURCE_NAME), ""))
    }

    /// Resolve `path` (following symlinks) and require it to stay inside the
    /// mount directory; `None` when it escapes or cannot be resolved.
    ///
    /// Mounted ConfigMap/Secret volumes are symlink farms managed by the
    /// kubelet (`key -> ..data/key`), so links must be followed — canonical
    /// paths of legitimate entries still live under the mount's canonical
    /// path. A link pointing elsewhere (e.g. `/etc/shadow`) is refused.
    fn entry_path_within_mount(&self, path: std::path::PathBuf) -> Option<std::path::PathBuf> {
        let mount = self.mount_path.canonicalize().ok()?;
        let resolved = path.canonicalize().ok()?;
        if resolved.starts_with(&mount) {
            Some(resolved)
        } else {
            None
        }
    }

    /// FNV-1a stamp over file names + contents for symlink-less directories.
    ///
    /// Mounted ConfigMaps/Secrets are size-bounded by kubelet (1 MiB), so
    /// hashing content is cheap and immune to coarse filesystem timestamps.
    fn directory_stamp(&self) -> ConfigResult<u64> {
        let entries = std::fs::read_dir(&self.mount_path).map_err(io_err(
            self.mount_path.display().to_string(),
            "cannot read k8s mount volume".to_string(),
        ))?;
        let mut hash: u64 = 0xcbf29ce484222325;
        let mut collected: Vec<(String, Vec<u8>)> = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with("..") {
                continue;
            }
            // Same containment rule as `read_volume`: entries resolving
            // outside the mount are not part of the volume.
            let Some(resolved) = self.entry_path_within_mount(entry.path()) else {
                continue;
            };
            let content = std::fs::read(&resolved).unwrap_or_default();
            collected.push((name, content));
        }
        collected.sort();
        for (name, content) in collected {
            for byte in name.as_bytes().iter().chain(content.iter()) {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        Ok(hash)
    }

    fn lock_error(&self) -> ConfigError {
        ConfigError::InvalidValue {
            key: SOURCE_NAME.to_string(),
            expected_type: "mount source state".to_string(),
            message: "k8s mounted source state lock poisoned".to_string(),
        }
    }
}

/// Build an `InvalidValue` error from an IO failure at `path`.
fn io_err(path: String, what: String) -> impl Fn(std::io::Error) -> ConfigError {
    move |e| ConfigError::InvalidValue {
        key: SOURCE_NAME.to_string(),
        expected_type: "readable path".to_string(),
        message: format!("{what} ({path}): {e}"),
    }
}

impl std::fmt::Debug for K8sMountedSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("K8sMountedSource")
            .field("mount_path", &self.mount_path)
            .finish()
    }
}

#[async_trait::async_trait]
impl crate::remote::PolledSource for K8sMountedSource {
    async fn poll(&self) -> ConfigResult<AnnotatedValue> {
        if let Some(fresh) = self.reload_if_swapped()? {
            return Ok(fresh);
        }
        let cached = self.cached.load();
        if let Some(ref value) = **cached {
            return Ok((**value).clone());
        }
        // First poll without a successful load yet.
        self.read_volume()
    }

    fn poll_interval(&self) -> Option<Duration> {
        Some(self.interval)
    }

    fn source_id(&self) -> SourceId {
        self.source_id.clone()
    }
}

// =============================================================================
// REST API source (skeleton)
// =============================================================================

/// The object kind served by [`K8sApiSource`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum K8sObjectKind {
    /// ConfigMap: `data` values are plain strings.
    ConfigMap,
    /// Secret: `data` values are base64-encoded.
    Secret,
}

impl K8sObjectKind {
    /// URL path segment for the kind.
    pub fn segment(&self) -> &'static str {
        match self {
            Self::ConfigMap => "configmaps",
            Self::Secret => "secrets",
        }
    }
}

/// Builder for [`K8sApiSource`].
pub struct K8sApiSourceBuilder {
    namespace: String,
    name: String,
    kind: K8sObjectKind,
    /// API server base URL; defaults to the in-cluster service environment.
    api_host: Option<String>,
    /// Bearer token (defaults to the service-account token file).
    token: Option<String>,
    /// CA bundle for the API server TLS certificate (defaults to the
    /// service-account CA file when it exists).
    ca_file: Option<PathBuf>,
    connect_timeout: Duration,
    request_timeout: Duration,
    cb_threshold: u32,
    interval: Duration,
}

impl K8sApiSourceBuilder {
    /// Watch `<namespace>/<kind>/<name>`.
    pub fn new(namespace: impl Into<String>, name: impl Into<String>, kind: K8sObjectKind) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            kind,
            api_host: None,
            token: None,
            ca_file: None,
            connect_timeout: DEFAULT_K8S_CONNECT_TIMEOUT,
            request_timeout: DEFAULT_K8S_REQUEST_TIMEOUT,
            cb_threshold: 5,
            interval: Duration::from_secs(30),
        }
    }

    /// Override the API server (e.g. `https://kubernetes.default.svc` or a
    /// mock endpoint for tests).
    pub fn api_host(mut self, host: impl Into<String>) -> Self {
        self.api_host = Some(host.into());
        self
    }

    /// Provide the bearer token directly (defaults to the in-cluster
    /// service-account token file).
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Override the CA bundle used to verify the API server certificate
    /// (defaults to the in-cluster service-account `ca.crt` when present).
    pub fn ca_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.ca_file = Some(path.into());
        self
    }

    /// Set the connect timeout. Default: 10 seconds.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the overall request timeout. Default: 30 seconds.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// Set the circuit breaker failure threshold.
    ///
    /// After this many consecutive failed polls, the circuit opens and
    /// subsequent polls fail fast with `CircuitBreakerOpen`. Default: 5.
    pub fn circuit_breaker_threshold(mut self, failures: u32) -> Self {
        self.cb_threshold = failures;
        self
    }

    /// Poll interval for change detection.
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Build the source. Fails when neither an explicit API host nor the
    /// in-cluster environment is available, and fails loudly when a
    /// configured CA file cannot be read or parsed (Rule 12).
    pub fn build(self) -> ConfigResult<K8sApiSource> {
        let api_host = match self.api_host {
            Some(host) => host.trim_end_matches('/').to_string(),
            None => in_cluster_api_host().ok_or_else(|| ConfigError::InvalidValue {
                key: "k8s".to_string(),
                expected_type: "in-cluster environment".to_string(),
                message: format!(
                    "no api_host configured and KUBERNETES_SERVICE_HOST is not set \
                     (namespace '{}', object '{}')",
                    self.namespace, self.name
                ),
            })?,
        };
        let token = match self.token {
            Some(t) => Some(t),
            None => std::fs::read_to_string(DEFAULT_SERVICE_ACCOUNT_TOKEN)
                .ok()
                .map(|t| t.trim().to_string()),
        };
        let url = format!(
            "{}/api/v1/namespaces/{}/{}/{}",
            api_host,
            self.namespace,
            self.kind.segment(),
            self.name
        );
        let client = build_api_client(
            self.ca_file.as_deref(),
            self.connect_timeout,
            self.request_timeout,
        )?;
        Ok(K8sApiSource {
            url,
            kind: self.kind,
            token,
            interval: self.interval,
            client,
            cached: ArcSwap::new(Arc::new(None)),
            source_id: SourceId::new(format!(
                "{SOURCE_NAME}:api:{}:{}/{}",
                self.namespace,
                self.kind.segment(),
                self.name
            )),
            circuit_breaker: std::sync::Mutex::new(
                CircuitBreaker::new().with_threshold(self.cb_threshold),
            ),
        })
    }
}

/// Build the reqwest client used by [`K8sApiSource`].
///
/// - `ca_path` (explicit or the in-cluster service-account `ca.crt` when it
///   exists) is loaded and installed via `add_root_certificate`; unreadable
///   or unparsable PEM fails loudly.
/// - The client always carries the connect/total timeouts so a hung API
///   server cannot stall the poll loop forever.
fn build_api_client(
    ca_path: Option<&Path>,
    connect_timeout: Duration,
    request_timeout: Duration,
) -> ConfigResult<reqwest::Client> {
    let ca_path = match ca_path {
        Some(p) => Some(p.to_path_buf()),
        // In-cluster default: only used when the file actually exists, so
        // out-of-cluster use (tests, local development) keeps working.
        None => {
            let default_path = Path::new(DEFAULT_SERVICE_ACCOUNT_CA);
            default_path.exists().then(|| default_path.to_path_buf())
        }
    };

    let mut builder = reqwest::Client::builder()
        .connect_timeout(connect_timeout)
        .timeout(request_timeout);

    if let Some(ca_path) = ca_path {
        let ca_pem = std::fs::read(&ca_path).map_err(|e| ConfigError::InvalidValue {
            key: "k8s.tls.ca_file".to_string(),
            expected_type: "readable PEM file".to_string(),
            message: format!(
                "Failed to read Kubernetes CA file '{}': {e}",
                ca_path.display()
            ),
        })?;
        // reqwest/rustls silently accept PEM-less bytes, so the certificate
        // check is done here: a CA file without a certificate block fails
        // the build loudly instead of producing a client that cannot verify
        // the API server (Rule 12).
        if !ca_pem
            .windows(27)
            .any(|w| w == b"-----BEGIN CERTIFICATE-----")
        {
            return Err(ConfigError::InvalidValue {
                key: "k8s.tls.ca_file".to_string(),
                expected_type: "valid PEM certificate".to_string(),
                message: format!(
                    "Invalid Kubernetes CA certificate in '{}': no PEM certificate block found",
                    ca_path.display()
                ),
            });
        }
        let cert =
            reqwest::Certificate::from_pem(&ca_pem).map_err(|e| ConfigError::InvalidValue {
                key: "k8s.tls.ca_file".to_string(),
                expected_type: "valid PEM certificate".to_string(),
                message: format!(
                    "Invalid Kubernetes CA certificate in '{}': {e}",
                    ca_path.display()
                ),
            })?;
        builder = builder.add_root_certificate(cert);
    }

    builder.build().map_err(|e| ConfigError::InvalidValue {
        key: SOURCE_NAME.to_string(),
        expected_type: "HTTP client".to_string(),
        message: format!("failed to build k8s HTTP client: {e}"),
    })
}

/// Detect the in-cluster API server from the service environment.
pub fn in_cluster_api_host() -> Option<String> {
    let host = std::env::var("KUBERNETES_SERVICE_HOST").ok()?;
    let port = std::env::var("KUBERNETES_SERVICE_PORT_HTTPS")
        .or_else(|_| std::env::var("KUBERNETES_SERVICE_PORT"))
        .unwrap_or_else(|_| "443".to_string());
    Some(format!("https://{host}:{port}"))
}

/// Configuration source fetching a ConfigMap/Secret through the Kubernetes
/// REST API (MVP skeleton: single-object GET with bearer auth).
pub struct K8sApiSource {
    url: String,
    kind: K8sObjectKind,
    token: Option<String>,
    interval: Duration,
    client: reqwest::Client,
    cached: ArcSwap<Option<Arc<AnnotatedValue>>>,
    source_id: SourceId,
    /// Poll circuit breaker repeated failures open the circuit and
    /// polls fail fast without touching the network.
    circuit_breaker: std::sync::Mutex<CircuitBreaker>,
}

impl K8sApiSource {
    /// The fetched object URL.
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Fetch the object and project its `data` map into config values.
    async fn fetch(&self) -> ConfigResult<AnnotatedValue> {
        let mut request = self.client.get(&self.url);
        if let Some(ref token) = self.token {
            request = request.bearer_auth(token);
        }
        let response = request
            .send()
            .await
            .map_err(|e| ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "k8s API response".to_string(),
                message: format!("k8s API request failed: {e}"),
            })?;
        let status = response.status();
        if !status.is_success() {
            return Err(ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "2xx status".to_string(),
                message: format!("k8s API returned {status} for {}", self.url),
            });
        }
        let body: serde_json::Value =
            response
                .json()
                .await
                .map_err(|e| ConfigError::InvalidValue {
                    key: SOURCE_NAME.to_string(),
                    expected_type: "k8s API JSON body".to_string(),
                    message: format!("k8s API returned invalid JSON: {e}"),
                })?;

        let data = body
            .get("data")
            .and_then(|d| d.as_object())
            .ok_or_else(|| ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "object with a data map".to_string(),
                message: format!("k8s API response for {} has no data map", self.url),
            })?;

        let mut map = indexmap::IndexMap::new();
        for (key, value) in data {
            let raw = value.as_str().unwrap_or_default();
            let content = match self.kind {
                K8sObjectKind::ConfigMap => raw.to_string(),
                K8sObjectKind::Secret => {
                    decode_base64(raw).ok_or_else(|| ConfigError::InvalidValue {
                        key: SOURCE_NAME.to_string(),
                        expected_type: "base64 secret value".to_string(),
                        message: format!("secret key '{key}' is not valid base64"),
                    })?
                }
            };
            let parsed =
                try_parse_value_with_format(&content, None, SOURCE_NAME).unwrap_or_else(|| {
                    AnnotatedValue::new(
                        ConfigValue::String(content),
                        SourceId::new(SOURCE_NAME),
                        key.as_str(),
                    )
                });
            map.insert(Arc::from(key.as_str()), parsed);
        }

        let value = if map.is_empty() {
            ConfigValue::Null
        } else {
            ConfigValue::map(map.into_iter().collect())
        };
        Ok(AnnotatedValue::new(value, SourceId::new(SOURCE_NAME), ""))
    }
}

/// Decode standard base64 (with padding tolerance) into UTF-8 text.
fn decode_base64(input: &str) -> Option<String> {
    use base64::Engine;
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let engine = base64::engine::general_purpose::STANDARD;
    let bytes = engine.decode(cleaned.as_bytes()).ok()?;
    String::from_utf8(bytes).ok()
}

#[async_trait::async_trait]
impl crate::remote::PolledSource for K8sApiSource {
    async fn poll(&self) -> ConfigResult<AnnotatedValue> {
        // circuit breaker around the fetch — consecutive failures open
        // the circuit and subsequent polls fail fast without a request.
        let allowed = {
            let mut cb = self
                .circuit_breaker
                .lock()
                .map_err(|_| ConfigError::InvalidValue {
                    key: SOURCE_NAME.to_string(),
                    expected_type: "k8s circuit breaker state".to_string(),
                    message: "k8s circuit breaker lock poisoned".to_string(),
                })?;
            cb.can_execute()
        };
        if !allowed {
            return Err(ConfigError::RemoteUnavailable {
                error_type: "CircuitBreakerOpen".to_string(),
                retryable: false,
            });
        }
        let fresh = crate::remote::record_fetch_metrics(&self.source_id, self.fetch()).await;
        {
            let mut cb = self
                .circuit_breaker
                .lock()
                .map_err(|_| ConfigError::InvalidValue {
                    key: SOURCE_NAME.to_string(),
                    expected_type: "k8s circuit breaker state".to_string(),
                    message: "k8s circuit breaker lock poisoned".to_string(),
                })?;
            match &fresh {
                Ok(_) => cb.record_success(),
                Err(_) => cb.record_failure(),
            }
        }
        let fresh = fresh?;
        self.cached.store(Arc::new(Some(Arc::new(fresh.clone()))));
        Ok(fresh)
    }

    fn poll_interval(&self) -> Option<Duration> {
        Some(self.interval)
    }

    fn source_id(&self) -> SourceId {
        self.source_id.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Map-lookup helper on a snapshot: `("", "")` addresses the root.
    fn get_path<'a>(
        value: &'a AnnotatedValue,
        key: &str,
        nested: &str,
    ) -> Option<&'a AnnotatedValue> {
        let entry = value.inner.as_map()?.get(key)?;
        if nested.is_empty() {
            Some(entry)
        } else {
            entry.inner.as_map()?.get(nested)
        }
    }

    /// Atomic swap: `os::rename` of a temp symlink over `..data` — exactly
    /// how kubelet's atomic writer publishes a new ConfigMap generation.
    fn atomic_swap_link(dir: &Path, from: &str) {
        let tmp_link = dir.join("..data.swap");
        let _ = fs::remove_file(&tmp_link);
        #[cfg(unix)]
        std::os::unix::fs::symlink(from, &tmp_link).expect("create swap symlink");
        fs::rename(&tmp_link, dir.join("..data")).expect("atomic symlink swap");
    }

    #[test]
    fn symlink_swap_is_detected_and_reloaded_atomically() {
        let dir = TempDir::new().expect("tempdir");
        let mount = dir.path();

        // Generation 1 (kubelet atomic-writer layout: per-key symlinks go
        // through the `..data` indirection).
        let gen1 = mount.join("..2024_01_01_00_00_00.000");
        fs::create_dir(&gen1).unwrap();
        fs::write(gen1.join("config.toml"), "host = \"v1\"\nport = 1\n").unwrap();
        atomic_swap_link(mount, "..2024_01_01_00_00_00.000");
        #[cfg(unix)]
        std::os::unix::fs::symlink("..data/config.toml", mount.join("config.toml"))
            .expect("key symlink");

        let source = K8sMountedSource::new(mount);

        // First poll: loads generation 1.
        let first = source.reload_if_swapped().expect("load gen1");
        assert!(first.is_some(), "first load must produce a snapshot");
        let v = first.unwrap();
        assert!(v.is_map());
        let host = get_path(&v, "config.toml", "host");
        assert_eq!(host.and_then(|h| h.as_str()), Some("v1"));

        // No swap: no reload.
        let second = source.reload_if_swapped().expect("no-op poll");
        assert!(second.is_none(), "unchanged generation must not reload");

        // Generation 2 via atomic symlink swap.
        let gen2 = mount.join("..2024_01_02_00_00_00.000");
        fs::create_dir(&gen2).unwrap();
        fs::write(gen2.join("config.toml"), "host = \"v2\"\nport = 2\n").unwrap();
        atomic_swap_link(mount, "..2024_01_02_00_00_00.000");

        let third = source.reload_if_swapped().expect("load gen2");
        assert!(third.is_some(), "swapped generation must reload");
        let v = third.unwrap();
        let host = get_path(&v, "config.toml", "host");
        assert_eq!(
            host.and_then(|h| h.as_str()),
            Some("v2"),
            "new generation wins"
        );
    }

    #[test]
    fn plain_directory_without_data_link_falls_back_to_content_stamp() {
        let dir = TempDir::new().expect("tempdir");
        let mount = dir.path();
        fs::write(mount.join("plain.txt"), "hello\n").unwrap();

        let source = K8sMountedSource::new(mount);
        let first = source.reload_if_swapped().expect("first load");
        assert!(first.is_some());
        let second = source.reload_if_swapped().expect("second poll");
        assert!(second.is_none(), "unchanged plain dir must not reload");

        fs::write(mount.join("plain.txt"), "changed\n").unwrap();
        let third = source.reload_if_swapped().expect("third poll");
        assert!(third.is_some(), "content change must reload");
    }

    #[test]
    fn unparsable_files_are_kept_as_raw_strings() {
        let dir = TempDir::new().expect("tempdir");
        let mount = dir.path();
        fs::write(mount.join("mode"), "0755\n").unwrap();

        let source = K8sMountedSource::new(mount);
        let value = source.read_volume().expect("read volume");
        let mode = get_path(&value, "mode", "").and_then(|m| m.as_str());
        assert_eq!(mode, Some("0755\n"), "raw text stays a string");
    }

    #[test]
    fn internal_atomic_writer_entries_are_skipped() {
        let dir = TempDir::new().expect("tempdir");
        let mount = dir.path();
        let generation = mount.join("..2024_01_01_00_00_00.000");
        fs::create_dir(&generation).unwrap();
        fs::write(generation.join("secret"), "s").unwrap();
        fs::write(mount.join("key"), "v").unwrap();
        atomic_swap_link(mount, "..2024_01_01_00_00_00.000");

        let source = K8sMountedSource::new(mount);
        let value = source.read_volume().expect("read volume");
        assert!(
            get_path(&value, "key", "").is_some(),
            "regular file is a key"
        );
        assert!(
            get_path(&value, "..data", "").is_none()
                && get_path(&value, "..2024_01_01_00_00_00.000", "").is_none(),
            "atomic-writer internals must be skipped"
        );
    }

    #[tokio::test]
    async fn mounted_source_poll_matches_reload() {
        let dir = TempDir::new().expect("tempdir");
        let mount = dir.path();
        fs::write(mount.join("a"), "1").unwrap();
        let source = K8sMountedSource::new(mount);
        let polled = crate::remote::PolledSource::poll(&source)
            .await
            .expect("poll");
        assert_eq!(
            get_path(&polled, "a", "").and_then(|v| v.as_str()),
            Some("1")
        );
    }

    #[test]
    fn api_builder_requires_host_or_in_cluster_env() {
        // Explicit host always builds.
        let source = K8sApiSourceBuilder::new("default", "app-config", K8sObjectKind::ConfigMap)
            .api_host("http://127.0.0.1:8080")
            .build();
        assert!(source.is_ok(), "explicit api_host must build");
        assert_eq!(
            source.unwrap().url(),
            "http://127.0.0.1:8080/api/v1/namespaces/default/configmaps/app-config"
        );
    }

    #[tokio::test]
    async fn api_source_fetches_configmap_data_against_mock_server() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind ephemeral port");
        let addr = listener.local_addr().expect("local addr");

        let body = serde_json::json!({
            "kind": "ConfigMap",
            "metadata": {"name": "app-config"},
            "data": {"config.json": "{\"log_level\": \"debug\"}"}
        })
        .to_string();

        let server = tokio::spawn(async move {
            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let (mut stream, _) = listener.accept().await.expect("accept");
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf).await.expect("read request");
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).await.expect("write");
        });

        let source = K8sApiSourceBuilder::new("default", "app-config", K8sObjectKind::ConfigMap)
            .api_host(format!("http://{addr}"))
            .token("test-token")
            .build()
            .expect("build");

        let polled = crate::remote::PolledSource::poll(&source)
            .await
            .expect("poll mock api");
        server.abort();

        // The nested `config.json` key parses as JSON and yields a map.
        let log_level = get_path(&polled, "config.json", "log_level").and_then(|v| v.as_str());
        assert_eq!(
            log_level,
            Some("debug"),
            "configmap data projected as config keys"
        );
    }

    #[test]
    fn base64_decoding_covers_padded_and_whitespace_input() {
        assert_eq!(decode_base64("aGVsbG8="), Some("hello".to_string()));
        assert_eq!(decode_base64("aGVs\nbG8="), Some("hello".to_string()));
        assert_eq!(decode_base64("!!!"), None);
    }

    // ==================== in-cluster client hardening ====================

    /// A valid self-signed PEM used as a stand-in service-account CA bundle
    /// (fixture file: base64 trips spell-checkers/secret scanners; excluded
    /// via typos.toml and covered by .secrets.baseline).
    const TEST_CA_PEM: &str = include_str!("testdata/test-ca.pem");

    /// the client is built with the SA CA bundle installed and the
    /// default timeouts applied (connect 10s / total 30s). A readable,
    /// parsable PEM must be accepted by the client builder — proving the
    /// `add_root_certificate` path executes.
    #[test]
    fn api_client_loads_ca_and_applies_default_timeouts() {
        let dir = TempDir::new().expect("tempdir");
        let ca_path = dir.path().join("ca.crt");
        fs::write(&ca_path, TEST_CA_PEM).expect("write CA pem");

        // Explicit CA file: must build (CA parsed and installed).
        let client = build_api_client(
            Some(&ca_path),
            DEFAULT_K8S_CONNECT_TIMEOUT,
            DEFAULT_K8S_REQUEST_TIMEOUT,
        );
        assert!(
            client.is_ok(),
            "client with a valid CA must build: {:?}",
            client.err()
        );

        // The in-cluster default path resolution: pointing the builder at a
        // CA-less environment (no explicit ca_file, default path absent in
        // the test environment) still builds — the default CA is optional
        // off-cluster.
        let source = K8sApiSourceBuilder::new("default", "app-config", K8sObjectKind::ConfigMap)
            .api_host("https://kubernetes.default.svc")
            .build();
        assert!(source.is_ok(), "build without CA must succeed off-cluster");
    }

    /// an unreadable or unparsable CA file fails loudly instead of
    /// silently building a client that cannot verify the API server.
    #[test]
    fn api_client_fails_loud_on_bad_ca_file() {
        // Missing file.
        let missing = build_api_client(
            Some(Path::new("/nonexistent/confers/ca.crt")),
            DEFAULT_K8S_CONNECT_TIMEOUT,
            DEFAULT_K8S_REQUEST_TIMEOUT,
        );
        assert!(missing.is_err(), "missing CA file must fail the build");
        let err = missing.unwrap_err().to_string();
        assert!(
            err.contains("Failed to read Kubernetes CA file"),
            "error must mention the CA file read failure: {err}"
        );

        // Unparsable PEM.
        let dir = TempDir::new().expect("tempdir");
        let ca_path = dir.path().join("ca.crt");
        fs::write(&ca_path, "not a pem").expect("write junk pem");
        let junk = build_api_client(
            Some(&ca_path),
            DEFAULT_K8S_CONNECT_TIMEOUT,
            DEFAULT_K8S_REQUEST_TIMEOUT,
        );
        assert!(junk.is_err(), "unparsable PEM must fail the build");
        let err = junk.unwrap_err().to_string();
        assert!(
            err.contains("Invalid Kubernetes CA certificate"),
            "error must mention the PEM parse failure: {err}"
        );
    }

    /// the default timeouts are connect 10s / total 30s and the
    /// builder applies them unless overridden.
    #[test]
    fn api_client_default_timeout_constants() {
        assert_eq!(DEFAULT_K8S_CONNECT_TIMEOUT, Duration::from_secs(10));
        assert_eq!(DEFAULT_K8S_REQUEST_TIMEOUT, Duration::from_secs(30));

        // Builder-level override path: the values are plain fields consumed
        // by build_api_client; assert they round-trip through the builder.
        let builder = K8sApiSourceBuilder::new("default", "app", K8sObjectKind::ConfigMap)
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(7));
        assert_eq!(builder.connect_timeout, Duration::from_secs(3));
        assert_eq!(builder.request_timeout, Duration::from_secs(7));

        // Defaults match the constants.
        let builder = K8sApiSourceBuilder::new("default", "app", K8sObjectKind::ConfigMap);
        assert_eq!(builder.connect_timeout, DEFAULT_K8S_CONNECT_TIMEOUT);
        assert_eq!(builder.request_timeout, DEFAULT_K8S_REQUEST_TIMEOUT);
    }

    /// after the failure threshold is reached, polls fail fast with
    /// `CircuitBreakerOpen` without issuing an API request.
    #[tokio::test]
    async fn api_source_circuit_breaker_opens_after_failures() {
        // A guaranteed-closed port: the first poll fails on connect.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        drop(listener);

        let source = K8sApiSourceBuilder::new("default", "app-config", K8sObjectKind::ConfigMap)
            .api_host(format!("http://{addr}"))
            .token("test-token")
            .circuit_breaker_threshold(1)
            .build()
            .expect("build");

        use crate::remote::PolledSource;
        // 1. First poll: the connection failure surfaces its own error and
        //    opens the circuit (threshold 1).
        let first = source.poll().await;
        assert!(first.is_err(), "closed port must fail the poll");
        assert!(
            !matches!(first, Err(ConfigError::RemoteUnavailable { ref error_type, .. }) if error_type == "CircuitBreakerOpen"),
            "the first failure must be the fetch error itself: {first:?}"
        );

        // 2. While open (1s default base delay): fail fast, no request.
        let second = source.poll().await;
        match second {
            Err(ConfigError::RemoteUnavailable {
                error_type,
                retryable: false,
            }) if error_type == "CircuitBreakerOpen" => {}
            other => panic!("open circuit must fail fast, got: {other:?}"),
        }
    }
}
