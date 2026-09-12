// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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
use crate::remote::common::try_parse_value_with_format;
use crate::types::{AnnotatedValue, ConfigValue, SourceId};

const SOURCE_NAME: &str = "k8s";

/// Default service-account token path inside a pod.
pub const DEFAULT_SERVICE_ACCOUNT_TOKEN: &str =
    "/var/run/secrets/kubernetes.io/serviceaccount/token";

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
        if let Some(t) = target {
            if let Ok(mut guard) = self.observed_target.try_lock() {
                *guard = Some(Arc::from(t.as_str()));
            }
        }
        let value = self.read_volume()?;
        self.cached
            .store(Arc::new(Some(Arc::new(value.clone()))));
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
    interval: Duration,
}

impl K8sApiSourceBuilder {
    /// Watch `<namespace>/<kind>/<name>`.
    pub fn new(
        namespace: impl Into<String>,
        name: impl Into<String>,
        kind: K8sObjectKind,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            name: name.into(),
            kind,
            api_host: None,
            token: None,
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

    /// Poll interval for change detection.
    pub fn interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Build the source. Fails when neither an explicit API host nor the
    /// in-cluster environment is available.
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
        let url = format!("{}/api/v1/namespaces/{}/{}/{}", api_host, self.namespace, self.kind.segment(), self.name);
        Ok(K8sApiSource {
            url,
            kind: self.kind,
            token,
            interval: self.interval,
            client: reqwest::Client::new(),
            cached: ArcSwap::new(Arc::new(None)),
            source_id: SourceId::new(format!("{SOURCE_NAME}:api:{}:{}/{}", self.namespace, self.kind.segment(), self.name)),
        })
    }
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
        let response = request.send().await.map_err(|e| ConfigError::InvalidValue {
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
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "k8s API JSON body".to_string(),
                message: format!("k8s API returned invalid JSON: {e}"),
            })?;

        let data = body.get("data").and_then(|d| d.as_object()).ok_or_else(|| {
            ConfigError::InvalidValue {
                key: SOURCE_NAME.to_string(),
                expected_type: "object with a data map".to_string(),
                message: format!("k8s API response for {} has no data map", self.url),
            }
        })?;

        let mut map = indexmap::IndexMap::new();
        for (key, value) in data {
            let raw = value.as_str().unwrap_or_default();
            let content = match self.kind {
                K8sObjectKind::ConfigMap => raw.to_string(),
                K8sObjectKind::Secret => decode_base64(raw).ok_or_else(|| {
                    ConfigError::InvalidValue {
                        key: SOURCE_NAME.to_string(),
                        expected_type: "base64 secret value".to_string(),
                        message: format!("secret key '{key}' is not valid base64"),
                    }
                })?,
            };
            let parsed = try_parse_value_with_format(&content, None, SOURCE_NAME)
                .unwrap_or_else(|| {
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
        let fresh = crate::remote::record_fetch_metrics(&self.source_id, self.fetch()).await?;
        self.cached
            .store(Arc::new(Some(Arc::new(fresh.clone()))));
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
        assert_eq!(host.and_then(|h| h.as_str()), Some("v2"), "new generation wins");
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
        assert!(get_path(&value, "key", "").is_some(), "regular file is a key");
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
        assert_eq!(get_path(&polled, "a", "").and_then(|v| v.as_str()), Some("1"));
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
        let log_level = get_path(&polled, "config.json", "log_level")
            .and_then(|v| v.as_str());
        assert_eq!(log_level, Some("debug"), "configmap data projected as config keys");
    }

    #[test]
    fn base64_decoding_covers_padded_and_whitespace_input() {
        assert_eq!(decode_base64("aGVsbG8="), Some("hello".to_string()));
        assert_eq!(decode_base64("aGVs\nbG8="), Some("hello".to_string()));
        assert_eq!(decode_base64("!!!"), None);
    }
}
