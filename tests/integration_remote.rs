//! Integration tests for remote configuration sources.
//!
//! These tests verify the PolledSource trait and HttpPolledSource implementations.

#![cfg(feature = "remote")]

use std::time::Duration;

use confers::remote::{HttpPolledSourceBuilder, PolledSource};

/// Test that HttpPolledSourceBuilder requires a URL.
#[test]
fn test_builder_requires_url() {
    let result = HttpPolledSourceBuilder::new().build();
    assert!(result.is_err());

    let err = result.err().unwrap();
    assert!(matches!(err, confers::ConfigError::InvalidValue { .. }));
}

/// Test that HttpPolledSourceBuilder accepts valid URL.
#[test]
fn test_builder_accepts_url() {
    let source = HttpPolledSourceBuilder::new()
        .url("https://example.com/config.json")
        .build()
        .unwrap();

    assert_eq!(
        source.source_id().as_str(),
        "http:https://example.com/config.json"
    );
}

/// Test that custom poll interval is set correctly.
#[test]
fn test_builder_custom_interval() {
    let source = HttpPolledSourceBuilder::new()
        .url("https://example.com/config.json")
        .interval(Duration::from_secs(45))
        .build()
        .unwrap();

    assert_eq!(source.poll_interval(), Some(Duration::from_secs(45)));
}

/// Test that default poll interval is 60 seconds.
#[test]
fn test_default_poll_interval() {
    let source = HttpPolledSourceBuilder::new()
        .url("https://example.com/config.json")
        .build()
        .unwrap();

    // DEFAULT_POLL_INTERVAL is 60 seconds
    assert_eq!(source.poll_interval(), Some(Duration::from_secs(60)));
}

/// Test that format can be specified.
#[test]
fn test_builder_with_format() {
    use confers::loader::Format;

    let source = HttpPolledSourceBuilder::new()
        .url("https://example.com/config.toml")
        .format(Format::Toml)
        .build()
        .unwrap();

    // Just verify it builds successfully
    assert!(source.source_id().as_str().contains("example.com"));
}

/// Test that timeout can be specified.
#[test]
fn test_builder_with_timeout() {
    let source = HttpPolledSourceBuilder::new()
        .url("https://example.com/config.json")
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    // Just verify it builds successfully
    assert!(source.source_id().as_str().contains("example.com"));
}

/// Test URL formats.
#[test]
fn test_various_url_formats() {
    // HTTPS URL (HTTP is not allowed for security reasons)
    let source = HttpPolledSourceBuilder::new()
        .url("https://localhost:8080/config.json")
        .build()
        .unwrap();
    // source_id format is "http:https://..."
    assert_eq!(
        source.source_id().as_str(),
        "http:https://localhost:8080/config.json"
    );

    // HTTPS URL
    let source = HttpPolledSourceBuilder::new()
        .url("https://api.example.com/v1/config")
        .build()
        .unwrap();
    assert!(source.source_id().as_str().contains("api.example.com"));

    // URL with query string
    let source = HttpPolledSourceBuilder::new()
        .url("https://example.com/config?env=prod")
        .build()
        .unwrap();
    assert!(source.source_id().as_str().contains("env=prod"));
}

/// Test builder pattern chaining.
#[test]
fn test_builder_pattern() {
    use confers::loader::Format;

    let source = HttpPolledSourceBuilder::new()
        .url("https://example.com/config.yaml")
        .interval(Duration::from_secs(30))
        .format(Format::Yaml)
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();

    assert_eq!(source.poll_interval(), Some(Duration::from_secs(30)));
}

/// Test PolledSource trait object creation.
#[test]
fn test_trait_object() {
    let source: Box<dyn PolledSource> = Box::new(
        HttpPolledSourceBuilder::new()
            .url("https://example.com/config.json")
            .build()
            .unwrap(),
    );

    // Can call trait methods on trait object
    let _source_id = source.source_id();
    let _interval = source.poll_interval();
}
