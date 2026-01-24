// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

#[cfg(any(feature = "watch", feature = "remote"))]
use crate::error::ConfigError;

#[cfg(feature = "watch")]
use notify::{RecursiveMode, Watcher};
#[cfg(feature = "watch")]
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[cfg(feature = "remote")]
use crate::utils::ssrf::validate_remote_url;
#[cfg(feature = "remote")]
use reqwest;
#[cfg(feature = "remote")]
use tokio::time::interval;

#[cfg(feature = "remote")]
use std::fs;

#[cfg(all(feature = "remote", feature = "encryption"))]
use crate::security::{SecureString, SensitivityLevel};

#[cfg(all(feature = "remote", not(feature = "encryption")))]
use std::sync::Arc;

#[cfg(feature = "watch")]
use std::sync::mpsc::{channel, Receiver};

#[cfg(feature = "watch")]
use crate::core::loader::is_editor_temp_file;

/// Type alias for the debounced watcher result
#[cfg(all(feature = "remote", feature = "watch"))]
type DebouncedWatcherResult = Result<
    (
        Debouncer<notify::RecommendedWatcher, FileIdMap>,
        Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
    ),
    ConfigError,
>;

pub struct ReloadLatencyMetrics {
    change_detected_at: Instant,
    reload_completed_at: Option<Instant>,
}

impl ReloadLatencyMetrics {
    pub fn new() -> Self {
        Self {
            change_detected_at: Instant::now(),
            reload_completed_at: None,
        }
    }

    pub fn with_change_detected_at(at: Instant) -> Self {
        Self {
            change_detected_at: at,
            reload_completed_at: None,
        }
    }

    pub fn mark_completed(&mut self) {
        self.reload_completed_at = Some(Instant::now());
    }

    pub fn latency(&self) -> Option<Duration> {
        self.reload_completed_at
            .map(|completed| completed.duration_since(self.change_detected_at))
    }

    pub fn latency_ms(&self) -> Option<u128> {
        self.latency().map(|d| d.as_millis())
    }

    /// Get the change detection time (for testing/debugging)
    pub fn change_detected_at(&self) -> &Instant {
        &self.change_detected_at
    }
}

impl Default for ReloadLatencyMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::large_enum_variant)]
pub enum WatchTarget {
    Files(Vec<PathBuf>),
    #[cfg(feature = "remote")]
    Remote {
        url: String,
        poll_interval: Duration,
        auth: Option<RemoteAuth>,
        tls: Option<TlsConfig>,
    },
}

#[cfg(feature = "remote")]
#[derive(Clone)]
pub struct TlsConfig {
    ca_cert_path: Option<String>,
    client_cert_path: Option<String>,
    client_key_path: Option<String>,
}

#[cfg(feature = "remote")]
impl TlsConfig {
    /// Create a new TlsConfig with builder pattern
    pub fn new() -> Self {
        Self {
            ca_cert_path: None,
            client_cert_path: None,
            client_key_path: None,
        }
    }

    /// Set CA certificate path
    pub fn with_ca_cert(mut self, path: impl Into<PathBuf>) -> Self {
        self.ca_cert_path = Some(path.into().to_string_lossy().into_owned());
        self
    }

    /// Set client certificate path
    pub fn with_client_cert(mut self, path: impl Into<PathBuf>) -> Self {
        self.client_cert_path = Some(path.into().to_string_lossy().into_owned());
        self
    }

    /// Set client key path
    pub fn with_client_key(mut self, path: impl Into<PathBuf>) -> Self {
        self.client_key_path = Some(path.into().to_string_lossy().into_owned());
        self
    }

    /// Get CA certificate path reference
    pub fn ca_cert_path(&self) -> Option<&String> {
        self.ca_cert_path.as_ref()
    }

    /// Get client certificate path reference
    pub fn client_cert_path(&self) -> Option<&String> {
        self.client_cert_path.as_ref()
    }

    /// Get client key path reference
    pub fn client_key_path(&self) -> Option<&String> {
        self.client_key_path.as_ref()
    }

    /// Convert to unified TlsConfig with PathBuf
    pub fn to_unified_config(&self) -> TlsConfig {
        use std::path::PathBuf;
        let mut config = TlsConfig::new();
        if let Some(path) = &self.ca_cert_path {
            config = config.with_ca_cert(PathBuf::from(path));
        }
        if let Some(path) = &self.client_cert_path {
            config = config.with_client_cert(PathBuf::from(path));
        }
        if let Some(path) = &self.client_key_path {
            config = config.with_client_key(PathBuf::from(path));
        }
        config
    }
}

#[cfg(feature = "remote")]
impl Default for TlsConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(feature = "remote", feature = "encryption"))]
#[derive(Clone)]
pub struct RemoteAuth {
    username: Option<String>,
    password: Option<SecureString>,
    bearer_token: Option<SecureString>,
}

#[cfg(all(feature = "remote", feature = "encryption"))]
impl RemoteAuth {
    /// Create a new RemoteAuth with builder pattern
    pub fn new() -> Self {
        Self {
            username: None,
            password: None,
            bearer_token: None,
        }
    }

    /// Set username
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set password securely
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(SecureString::new(
            password.into(),
            SensitivityLevel::Critical,
        ));
        self
    }

    /// Set bearer token securely
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(SecureString::new(token.into(), SensitivityLevel::Critical));
        self
    }

    /// Get username reference
    pub fn username(&self) -> Option<&String> {
        self.username.as_ref()
    }

    /// Get password reference (for internal use only)
    #[doc(hidden)]
    pub fn password(&self) -> Option<&SecureString> {
        self.password.as_ref()
    }

    /// Get bearer token reference (for internal use only)
    #[doc(hidden)]
    pub fn bearer_token(&self) -> Option<&SecureString> {
        self.bearer_token.as_ref()
    }
}

#[cfg(all(feature = "remote", feature = "encryption"))]
impl Default for RemoteAuth {
    fn default() -> Self {
        Self::new()
    }
}

/// RemoteAuth without encryption feature
///
/// # Security Warning
///
/// When the `encryption` feature is disabled, credentials are stored in plain memory.
/// This is intended for development and testing environments only.
///
/// For production use, enable the `encryption` feature to use `SecureString` which provides:
/// - Constant-time comparison to prevent timing attacks
/// - Automatic memory zeroization on drop
/// - Sensitivity level tracking
///
/// To enable encryption, build with: `cargo build --features encryption`
#[cfg(all(feature = "remote", not(feature = "encryption")))]
#[derive(Clone)]
pub struct RemoteAuth {
    username: Option<String>,
    password: Option<String>,
    bearer_token: Option<String>,
}

#[cfg(all(feature = "remote", not(feature = "encryption")))]
impl RemoteAuth {
    /// Create a new RemoteAuth with builder pattern
    pub fn new() -> Self {
        Self {
            username: None,
            password: None,
            bearer_token: None,
        }
    }

    /// Set username
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set password (plain string when encryption is disabled)
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set bearer token (plain string when encryption is disabled)
    pub fn with_bearer_token(mut self, token: impl Into<String>) -> Self {
        self.bearer_token = Some(token.into());
        self
    }

    /// Get username reference
    pub fn username(&self) -> Option<&String> {
        self.username.as_ref()
    }

    /// Get password reference
    #[doc(hidden)]
    pub fn password(&self) -> Option<&String> {
        self.password.as_ref()
    }

    /// Get bearer token reference
    #[doc(hidden)]
    pub fn bearer_token(&self) -> Option<&String> {
        self.bearer_token.as_ref()
    }

    /// Validate that at least one authentication method is provided
    pub fn validate(&self) -> Result<(), ConfigError> {
        let has_basic = self.username.is_some() && self.password.is_some();
        let has_bearer = self.bearer_token.is_some();

        if !has_basic && !has_bearer {
            return Err(ConfigError::ValidationError(
                "RemoteAuth requires either username/password or bearer token".to_string(),
            ));
        }

        if has_basic && has_bearer {
            eprintln!(
                "Warning: Both Basic Auth and Bearer Token are set. Bearer Token will be used."
            );
        }

        Ok(())
    }

    /// Check if using Basic Authentication
    pub fn is_basic_auth(&self) -> bool {
        self.username.is_some() && self.password.is_some()
    }

    /// Check if using Bearer Token Authentication
    pub fn is_bearer_auth(&self) -> bool {
        self.bearer_token.is_some()
    }

    /// Get the authentication header value
    pub fn auth_header(&self) -> Option<String> {
        if let Some(token) = &self.bearer_token {
            return Some(format!("Bearer {}", token));
        }

        if let (Some(username), Some(password)) = (&self.username, &self.password) {
            let credentials = format!("{}:{}", username, password);
            let encoded = base64::engine::general_purpose::STANDARD.encode(credentials);
            return Some(format!("Basic {}", encoded));
        }

        None
    }

    /// Clear sensitive data from memory
    pub fn clear(&mut self) {
        if let Some(mut password) = self.password.take() {
            unsafe {
                let ptr = password.as_mut_vec();
                for byte in ptr.iter_mut() {
                    *byte = 0;
                }
            }
        }

        if let Some(mut token) = self.bearer_token.take() {
            unsafe {
                let ptr = token.as_mut_vec();
                for byte in ptr.iter_mut() {
                    *byte = 0;
                }
            }
        }
    }
}

#[cfg(all(feature = "remote", not(feature = "encryption")))]
impl Default for RemoteAuth {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
pub struct ConfigWatcher {
    target: WatchTarget,
}

impl ConfigWatcher {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            target: WatchTarget::Files(paths),
        }
    }

    #[cfg(feature = "remote")]
    pub fn new_remote(
        url: impl Into<String>,
        poll_interval: Duration,
    ) -> Result<Self, ConfigError> {
        let url_str = url.into();
        // Validate URL to prevent SSRF attacks
        validate_remote_url(&url_str)?;

        Ok(Self {
            target: WatchTarget::Remote {
                url: url_str,
                poll_interval,
                auth: None,
                tls: None,
            },
        })
    }

    #[cfg(feature = "remote")]
    pub fn with_remote_auth(
        mut self,
        username: Option<String>,
        password: Option<String>,
        bearer_token: Option<String>,
    ) -> Self {
        if let WatchTarget::Remote { ref mut auth, .. } = self.target {
            let mut remote_auth = RemoteAuth::new();

            if let Some(username) = username {
                remote_auth = remote_auth.with_username(username);
            }
            if let Some(password) = password {
                remote_auth = remote_auth.with_password(password);
            }
            if let Some(bearer_token) = bearer_token {
                remote_auth = remote_auth.with_bearer_token(bearer_token);
            }

            *auth = Some(remote_auth);
        }
        self
    }

    #[cfg(feature = "remote")]
    pub fn with_tls_config(mut self, tls_config: TlsConfig) -> Self {
        if let WatchTarget::Remote { ref mut tls, .. } = self.target {
            *tls = Some(tls_config);
        }
        self
    }

    #[cfg(feature = "watch")]
    #[allow(clippy::type_complexity)]
    pub fn watch(
        &self,
    ) -> Result<
        (
            Debouncer<notify::RecommendedWatcher, FileIdMap>,
            Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
        ),
        ConfigError,
    > {
        match &self.target {
            WatchTarget::Files(paths) => self.watch_files(paths),
            #[cfg(feature = "remote")]
            WatchTarget::Remote {
                url,
                poll_interval,
                auth,
                tls,
            } => self.watch_remote(url, *poll_interval, auth.clone(), tls.clone()),
        }
    }

    #[cfg(feature = "watch")]
    #[allow(clippy::type_complexity)]
    fn watch_files(
        &self,
        paths: &[PathBuf],
    ) -> Result<
        (
            Debouncer<notify::RecommendedWatcher, FileIdMap>,
            Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
        ),
        ConfigError,
    > {
        let (tx, rx) = channel();

        let mut debouncer = new_debouncer(
            Duration::from_millis(500),
            None,
            move |res: Result<Vec<DebouncedEvent>, Vec<notify::Error>>| match res {
                Ok(events) => {
                    let filtered_events: Vec<_> = events
                        .into_iter()
                        .filter_map(|mut e| {
                            e.paths.retain(|p| !is_editor_temp_file(p));
                            if e.paths.is_empty() {
                                None
                            } else {
                                Some(e)
                            }
                        })
                        .collect();
                    if !filtered_events.is_empty() {
                        let _ = tx.send(Ok(filtered_events));
                    }
                }
                Err(errors) => {
                    let _ = tx.send(Err(errors));
                }
            },
        )
        .map_err(|e| ConfigError::FormatDetectionFailed(e.to_string()))?;

        for path in paths {
            debouncer
                .watcher()
                .watch(path, RecursiveMode::NonRecursive)
                .map_err(|_e| ConfigError::FileNotFound { path: path.clone() })?;
        }

        Ok((debouncer, rx))
    }

    #[cfg(all(feature = "remote", feature = "watch"))]
    fn watch_remote(
        &self,
        url: &str,
        poll_interval: Duration,
        auth: Option<RemoteAuth>,
        tls: Option<TlsConfig>,
    ) -> DebouncedWatcherResult {
        let (tx, rx) = channel();
        let url = url.to_string();

        // Spawn a background task to poll the remote endpoint
        tokio::spawn(async move {
            let mut interval = interval(poll_interval);

            // Create HTTP client with TLS configuration
            let client_builder =
                reqwest::Client::builder().timeout(std::time::Duration::from_secs(30));

            let client_builder = if let Some(ref tls_config) = tls {
                // Apply TLS configuration
                let mut builder = client_builder;

                if let Some(ref ca_cert_path) = tls_config.ca_cert_path() {
                    match std::fs::read(ca_cert_path) {
                        Ok(cert_data) => match reqwest::Certificate::from_pem(&cert_data) {
                            Ok(cert) => {
                                builder = builder.add_root_certificate(cert);
                            }
                            Err(e) => {
                                let _ = tx.send(Err(vec![notify::Error::generic(&format!(
                                    "Failed to parse CA certificate: {}",
                                    e
                                ))]));
                                return;
                            }
                        },
                        Err(e) => {
                            let _ = tx.send(Err(vec![notify::Error::generic(&format!(
                                "Failed to read CA certificate file: {}",
                                e
                            ))]));
                            return;
                        }
                    }
                }

                #[allow(clippy::needless_borrow)]
                if let (Some(ref client_cert_path), Some(ref client_key_path)) =
                    (tls_config.client_cert_path(), tls_config.client_key_path())
                {
                    // Load client certificate and key, then create Identity for reqwest
                    match load_client_identity(client_cert_path, client_key_path) {
                        Ok(identity) => {
                            builder = builder.identity(identity);
                        }
                        Err(e) => {
                            let _ = tx.send(Err(vec![notify::Error::generic(&format!(
                                "Failed to load client certificate: {}",
                                e
                            ))]));
                            return;
                        }
                    }
                }

                builder
            } else {
                client_builder
            };

            let client = match client_builder.build() {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Err(vec![notify::Error::generic(&format!(
                        "Failed to create HTTP client: {}",
                        e
                    ))]));
                    return;
                }
            };

            loop {
                interval.tick().await;

                let mut request = client.get(&url);

                // Apply authentication if configured
                #[allow(clippy::needless_borrow)]
                if let Some(ref auth_config) = auth {
                    if let Some(ref token) = auth_config.bearer_token() {
                        request = request.bearer_auth(token.as_str());
                    } else if let (Some(ref username), Some(ref password)) =
                        (auth_config.username(), auth_config.password())
                    {
                        request = request.basic_auth(username, Some(password.as_str()));
                    }
                }

                match request.send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            // Simulate a file change event when we successfully fetch remote config
                            // For simplicity, we'll send an empty vector to indicate a change occurred
                            // This is sufficient to trigger a config reload in most implementations
                            let _ = tx.send(Ok(vec![]));
                        } else {
                            let _ = tx.send(Err(vec![notify::Error::generic(&format!(
                                "HTTP request failed with status: {}",
                                response.status()
                            ))]));
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(Err(vec![notify::Error::generic(&format!(
                            "HTTP request failed: {}",
                            e
                        ))]));
                    }
                }
            }
        });

        // Create a minimal debouncer for API compatibility
        // For remote configuration, the actual polling happens in the tokio::spawn task above
        // This debouncer is kept only to maintain consistent return type with file watching
        // Note: For remote watching, the debouncer callback never fires since events are
        // sent directly via the channel. This is an intentional design choice for remote sources.
        let debouncer = new_debouncer(Duration::from_secs(1), None, |_res| {})
            .map_err(|e| ConfigError::FormatDetectionFailed(e.to_string()))?;

        Ok((debouncer, rx))
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::{ConfigWatcher, Duration, ReloadLatencyMetrics};
    use std::fs;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use std::thread;
    use std::time::Instant;
    use tempfile::TempDir;

    #[cfg(feature = "watch")]
    #[test]
    fn test_watcher_single_file_change() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");
        fs::write(&file_path, "key = \"initial\"").unwrap();

        let watcher = ConfigWatcher::new(vec![file_path.clone()]);
        let (_debouncer, rx) = watcher.watch().unwrap();

        thread::spawn(move || {
            thread::sleep(Duration::from_millis(100));
            fs::write(&file_path, "key = \"updated\"").unwrap();
        });

        let result = rx.recv_timeout(Duration::from_secs(5));
        assert!(result.is_ok(), "Should receive file change event");
    }

    #[cfg(feature = "watch")]
    #[test]
    fn test_concurrent_file_modifications() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");
        fs::write(&file_path, "value = 0").unwrap();

        let watcher = ConfigWatcher::new(vec![file_path.clone()]);
        let (_debouncer, rx) = watcher.watch().unwrap();

        let event_count = Arc::new(AtomicUsize::new(0));
        let _count_clone = Arc::clone(&event_count);

        let rx_thread = thread::spawn(move || {
            let mut count = 0;
            for _ in 1..=10 {
                match rx.recv_timeout(Duration::from_secs(2)) {
                    Ok(Ok(events)) => {
                        if !events.is_empty() {
                            count += 1;
                        }
                    }
                    Err(_) => break,
                    Ok(Err(_)) => break,
                }
            }
            count
        });

        thread::sleep(Duration::from_millis(100));

        let handles: Vec<_> = (0..3)
            .map(|i| {
                let path = file_path.clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_millis(50 * i));
                    let _ = fs::write(&path, format!("value = {}", i + 1));
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        let final_count = rx_thread.join().unwrap();
        assert!(
            final_count > 0,
            "Should receive at least one file change event, got: {}",
            final_count
        );
    }

    #[cfg(feature = "watch")]
    #[test]
    fn test_multiple_files_watching() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("config1.toml");
        let file2 = temp_dir.path().join("config2.json");
        fs::write(&file1, "[section]\nkey = \"value\"").unwrap();
        fs::write(&file2, r#"{"key": "value"}"#).unwrap();

        let watcher = ConfigWatcher::new(vec![file1.clone(), file2.clone()]);
        let (_debouncer, rx) = watcher.watch().unwrap();

        let file1_handle = {
            let path = file1.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(100));
                fs::write(&path, "[section]\nkey = \"updated\"").unwrap();
            })
        };

        file1_handle.join().unwrap();

        let result = rx.recv_timeout(Duration::from_secs(5));
        assert!(result.is_ok(), "Should receive file change event");
    }

    #[cfg(feature = "watch")]
    #[test]
    fn test_rapid_successive_changes() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.yaml");
        fs::write(&file_path, "key: value1").unwrap();

        let watcher = ConfigWatcher::new(vec![file_path.clone()]);
        let (_debouncer, rx) = watcher.watch().unwrap();

        for i in 0..10 {
            let path = file_path.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(i * 10));
                let _ = fs::write(&path, format!("key: value{}", i + 2));
            });
        }

        let mut events_received = 0;
        let start = std::time::Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
            if let Ok(Ok(events)) = rx.recv_timeout(Duration::from_secs(1)) {
                if !events.is_empty() {
                    events_received += 1;
                }
            }
        }
        assert!(events_received >= 1);
    }

    #[cfg(feature = "watch")]
    #[test]
    fn test_debounce_behavior() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");
        fs::write(&file_path, "count = 0").unwrap();

        let watcher = ConfigWatcher::new(vec![file_path.clone()]);
        let (_debouncer, rx) = watcher.watch().unwrap();

        for i in 0..5 {
            let path = file_path.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_millis(50));
                let _ = fs::write(&path, format!("count = {}", i + 1));
            });
        }

        let start = std::time::Instant::now();
        let mut event_count = 0;
        while start.elapsed() < Duration::from_secs(3) {
            match rx.recv_timeout(Duration::from_secs(1)) {
                Ok(Ok(events)) => {
                    if !events.is_empty() {
                        event_count += 1;
                    }
                }
                Err(_) => break,
                Ok(Err(_)) => break,
            }
        }
        assert!(event_count >= 1);
    }

    #[test]
    fn test_reload_latency_metrics_basic() {
        let metrics = ReloadLatencyMetrics::new();
        let latency_ms = metrics.latency_ms();
        assert!(
            latency_ms.is_none(),
            "Latency should be None when not completed"
        );
    }

    #[cfg(feature = "watch")]
    #[test]
    fn test_reload_latency_measurement() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("config.toml");
        fs::write(&file_path, "key = \"initial\"").unwrap();

        let watcher = ConfigWatcher::new(vec![file_path.clone()]);
        let (_debouncer, rx) = watcher.watch().unwrap();

        let mut metrics = ReloadLatencyMetrics::new();

        let handle = thread::spawn(move || {
            thread::sleep(Duration::from_millis(50));
            fs::write(&file_path, "key = \"updated\"").unwrap();
        });

        let result = rx.recv_timeout(Duration::from_secs(5));
        assert!(result.is_ok(), "Should receive file change event");

        metrics.mark_completed();

        handle.join().unwrap();

        let latency_ms = metrics
            .latency_ms()
            .expect("Latency should be Some after completion");
        assert!(
            latency_ms >= 40,
            "Latency should be at least 40ms, got {}ms",
            latency_ms
        );
    }

    #[test]
    fn test_reload_latency_with_fixed_detection_time() {
        let detection_time = Instant::now() - Duration::from_millis(100);
        let mut metrics = ReloadLatencyMetrics::with_change_detected_at(detection_time);

        assert!(
            metrics.latency_ms().is_none(),
            "Latency should be None before completion"
        );

        metrics.mark_completed();

        let latency_ms = metrics
            .latency_ms()
            .expect("Latency should be Some after completion");
        assert!(
            latency_ms >= 90,
            "Latency should be at least 90ms, got {}ms",
            latency_ms
        );
    }

    #[cfg(feature = "remote")]
    use super::*;

    #[cfg(feature = "remote")]
    #[test]
    fn test_tls_config_creation() {
        let tls_config = TlsConfig::new()
            .with_ca_cert("/path/to/ca.crt")
            .with_client_cert("/path/to/client.crt")
            .with_client_key("/path/to/client.key");

        assert_eq!(
            tls_config.ca_cert_path(),
            Some(&"/path/to/ca.crt".to_string())
        );
        assert_eq!(
            tls_config.client_cert_path(),
            Some(&"/path/to/client.crt".to_string())
        );
        assert_eq!(
            tls_config.client_key_path(),
            Some(&"/path/to/client.key".to_string())
        );
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_watcher_with_tls_config() {
        let watcher =
            ConfigWatcher::new_remote("https://example.com/config", Duration::from_secs(60))
                .unwrap()
                .with_tls_config(TlsConfig::new().with_ca_cert("/path/to/ca.crt"));

        match watcher.target {
            WatchTarget::Remote { tls, .. } => {
                assert!(
                    tls.is_some(),
                    "TLS config should be present for remote watcher"
                );
                if let Some(tls_config) = tls {
                    assert_eq!(
                        tls_config.ca_cert_path(),
                        Some(&"/path/to/ca.crt".to_string()),
                        "CA cert path should match expected value"
                    );
                }
            }
            _ => panic!("Expected remote target"),
        }
    }
}

#[cfg(feature = "remote")]
fn load_client_identity(
    cert_path: &str,
    key_path: &str,
) -> Result<reqwest::Identity, Box<dyn std::error::Error + Send + Sync>> {
    // Read certificate file
    let cert_data = fs::read(cert_path)?;

    // Read private key file
    let key_data = fs::read(key_path)?;

    // Combine certificate and key into PKCS12 format for reqwest Identity
    // reqwest expects PEM format containing both certificate and private key
    let mut combined = Vec::new();
    combined.extend_from_slice(&cert_data);
    combined.extend_from_slice(b"\n");
    combined.extend_from_slice(&key_data);

    // Create identity from combined PEM data
    match reqwest::Identity::from_pem(&combined) {
        Ok(identity) => Ok(identity),
        Err(e) => Err(format!("Failed to create identity from certificate and key: {}", e).into()),
    }
}
