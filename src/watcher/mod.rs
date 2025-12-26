// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::core::loader::is_editor_temp_file;
use crate::error::ConfigError;
use notify::{RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, DebouncedEvent, Debouncer, FileIdMap};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

#[cfg(feature = "remote")]
use reqwest;
#[cfg(feature = "remote")]
use tokio::time::interval;

#[cfg(feature = "remote")]
use std::fs;

/// Type alias for the debounced watcher result
#[cfg(feature = "remote")]
type DebouncedWatcherResult = Result<
    (
        Debouncer<notify::RecommendedWatcher, FileIdMap>,
        Receiver<Result<Vec<DebouncedEvent>, Vec<notify::Error>>>,
    ),
    ConfigError,
>;

pub struct ReloadLatencyMetrics {
    pub change_detected_at: Instant,
    pub reload_completed_at: Option<Instant>,
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
}

impl Default for ReloadLatencyMetrics {
    fn default() -> Self {
        Self::new()
    }
}

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
    pub ca_cert_path: Option<String>,
    pub client_cert_path: Option<String>,
    pub client_key_path: Option<String>,
    pub skip_verify: bool,
}

#[cfg(feature = "remote")]
#[derive(Clone)]
pub struct RemoteAuth {
    pub username: Option<String>,
    pub password: Option<String>,
    pub bearer_token: Option<String>,
}

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
    pub fn new_remote(url: impl Into<String>, poll_interval: Duration) -> Self {
        Self {
            target: WatchTarget::Remote {
                url: url.into(),
                poll_interval,
                auth: None,
                tls: None,
            },
        }
    }

    #[cfg(feature = "remote")]
    pub fn with_remote_auth(
        mut self,
        username: Option<String>,
        password: Option<String>,
        bearer_token: Option<String>,
    ) -> Self {
        if let WatchTarget::Remote { ref mut auth, .. } = self.target {
            *auth = Some(RemoteAuth {
                username,
                password,
                bearer_token,
            });
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

            // Add to cache to ensure events are tracked (notify-debouncer-full specifics)
            debouncer
                .cache()
                .add_root(path, RecursiveMode::NonRecursive);
        }

        Ok((debouncer, rx))
    }

    #[cfg(feature = "remote")]
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

                if tls_config.skip_verify {
                    builder = builder.danger_accept_invalid_certs(true);
                }

                if let Some(ref ca_cert_path) = tls_config.ca_cert_path {
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

                if let (Some(ref client_cert_path), Some(ref client_key_path)) =
                    (&tls_config.client_cert_path, &tls_config.client_key_path)
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
                if let Some(ref auth_config) = auth {
                    if let Some(ref token) = auth_config.bearer_token {
                        request = request.bearer_auth(token);
                    } else if let (Some(ref username), password) =
                        (&auth_config.username, &auth_config.password)
                    {
                        request =
                            request.basic_auth(username, password.as_ref().map(|s| s.as_str()));
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

        // Create a dummy debouncer that doesn't actually watch anything
        // This is a workaround since we can't easily create a debouncer without a real watcher
        let debouncer = new_debouncer(Duration::from_secs(3600), None, |_res| {})
            .map_err(|e| ConfigError::FormatDetectionFailed(e.to_string()))?;

        Ok((debouncer, rx))
    }
}

#[cfg(test)]
mod tests {
    use super::{ConfigWatcher, Duration, ReloadLatencyMetrics};
    use std::fs;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use std::thread;
    use std::time::Instant;
    use tempfile::TempDir;

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
        let tls_config = TlsConfig {
            ca_cert_path: Some("/path/to/ca.crt".to_string()),
            client_cert_path: Some("/path/to/client.crt".to_string()),
            client_key_path: Some("/path/to/client.key".to_string()),
            skip_verify: false,
        };

        assert_eq!(tls_config.ca_cert_path, Some("/path/to/ca.crt".to_string()));
        assert_eq!(
            tls_config.client_cert_path,
            Some("/path/to/client.crt".to_string())
        );
        assert_eq!(
            tls_config.client_key_path,
            Some("/path/to/client.key".to_string())
        );
        assert!(!tls_config.skip_verify);
    }

    #[cfg(feature = "remote")]
    #[test]
    fn test_watcher_with_tls_config() {
        let watcher =
            ConfigWatcher::new_remote("https://example.com/config", Duration::from_secs(60))
                .with_tls_config(TlsConfig {
                    ca_cert_path: Some("/path/to/ca.crt".to_string()),
                    client_cert_path: None,
                    client_key_path: None,
                    skip_verify: true,
                });

        match watcher.target {
            WatchTarget::Remote { tls, .. } => {
                assert!(tls.is_some());
                if let Some(tls_config) = tls {
                    assert_eq!(tls_config.ca_cert_path, Some("/path/to/ca.crt".to_string()));
                    assert!(tls_config.skip_verify);
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
