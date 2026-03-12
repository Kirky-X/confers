//! Common test utilities for confers integration tests.
//!
//! This module provides shared test fixtures and helper functions to reduce
//! code duplication across test files.

use std::collections::HashMap;
use std::io::Write;

use confers::traits::ConfigProvider;
use confers::value::{AnnotatedValue, ConfigValue, SourceId};
use tempfile::NamedTempFile;

#[cfg(feature = "remote")]
use std::time::Duration;

#[cfg(feature = "remote")]
use reqwest;

/// A test configuration struct with common fields used across multiple tests.
///
/// This struct implements `ConfigProvider` and can be used in tests that need
/// a realistic configuration object with proper trait implementations.
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub timeout_ms: u32,
    pub max_connections: usize,
    pub database_host: String,
    pub database_port: u16,
    pub values: HashMap<String, AnnotatedValue>,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl TestConfig {
    /// Creates a new TestConfig with the specified timeout and max connections.
    pub fn new(timeout_ms: u32, max_connections: usize) -> Self {
        Self::with_all(timeout_ms, max_connections, "localhost".to_string(), 5432)
    }

    /// Creates a TestConfig with only timeout specified.
    pub fn with_timeout(timeout_ms: u32) -> Self {
        Self::new(timeout_ms, 0)
    }

    /// Creates a TestConfig with only max_connections specified.
    pub fn with_connections(max_connections: usize) -> Self {
        Self::new(0, max_connections)
    }

    /// Creates a TestConfig with random values for fuzzing tests.
    pub fn random() -> Self {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};

        let state = RandomState::new();
        let mut hasher = state.build_hasher();
        hasher.write_u32(42);
        let random_u32 = hasher.finish() as u32;

        Self::new(random_u32 % 10000, (random_u32 as usize) % 1000)
    }

    /// Creates a new TestConfig with all fields specified.
    pub fn with_all(
        timeout_ms: u32,
        max_connections: usize,
        database_host: String,
        database_port: u16,
    ) -> Self {
        let mut values = HashMap::new();

        values.insert(
            "timeout_ms".to_string(),
            AnnotatedValue::new(
                ConfigValue::from(timeout_ms),
                SourceId::new("test-config"),
                "timeout_ms",
            ),
        );
        values.insert(
            "max_connections".to_string(),
            AnnotatedValue::new(
                ConfigValue::from(max_connections as i64),
                SourceId::new("test-config"),
                "max_connections",
            ),
        );
        values.insert(
            "database_host".to_string(),
            AnnotatedValue::new(
                ConfigValue::from(database_host.clone()),
                SourceId::new("test-config"),
                "database_host",
            ),
        );
        values.insert(
            "database_port".to_string(),
            AnnotatedValue::new(
                ConfigValue::from(database_port as i64),
                SourceId::new("test-config"),
                "database_port",
            ),
        );

        Self {
            timeout_ms,
            max_connections,
            database_host,
            database_port,
            values,
        }
    }
}

impl ConfigProvider for TestConfig {
    fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
        self.values.get(key)
    }

    fn keys(&self) -> Vec<String> {
        self.values.keys().cloned().collect()
    }
}

/// Checks if a remote service is available at the given URL.
///
/// This is used by integration tests that require external services like
/// Etcd or Consul to be running.
#[cfg(feature = "remote")]
pub async fn is_service_available(url: &str, timeout: Duration) -> bool {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .build()
        .unwrap();

    match client.get(url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Runs a closure with a temporary environment variable set.
///
/// The environment variable is automatically removed after the closure completes,
/// regardless of whether it succeeded or panicked.
pub fn with_env_var<F, R>(key: &str, value: &str, f: F) -> R
where
    F: FnOnce() -> R,
{
    std::env::set_var(key, value);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f()));
    std::env::remove_var(key);

    match result {
        Ok(value) => value,
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

/// Creates a temporary configuration file with the given content and extension.
///
/// The file is automatically deleted when the returned `NamedTempFile` is dropped.
pub fn create_temp_config(content: &str, extension: &str) -> NamedTempFile {
    let mut file = NamedTempFile::with_suffix(extension).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_config_new() {
        let config = TestConfig::new(100, 10);
        assert_eq!(config.timeout_ms, 100);
        assert_eq!(config.max_connections, 10);
    }

    #[test]
    fn test_test_config_default() {
        let config = TestConfig::default();
        assert_eq!(config.timeout_ms, 0);
        assert_eq!(config.max_connections, 0);
    }

    #[test]
    fn test_test_config_provider() {
        let config = TestConfig::new(500, 100);

        assert!(config.get_raw("timeout_ms").is_some());
        assert!(config.get_raw("max_connections").is_some());
        assert!(config.get_raw("nonexistent").is_none());

        let keys = config.keys();
        assert_eq!(keys.len(), 4);
    }

    #[test]
    fn test_with_env_var() {
        with_env_var("TEST_VAR_COMMON", "test_value", || {
            assert_eq!(std::env::var("TEST_VAR_COMMON").unwrap(), "test_value");
        });

        assert!(std::env::var("TEST_VAR_COMMON").is_err());
    }

    #[test]
    fn test_create_temp_config() {
        let file = create_temp_config("key = \"value\"", ".toml");
        assert!(file.path().exists());
        assert!(file.path().to_string_lossy().ends_with(".toml"));
    }
}
