// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 集成测试：HTTP 提供者功能
//!
//! 测试 HTTP 配置提供者的功能

#[cfg(test)]
mod tests {
    use confers::providers::HttpConfigProvider;
    use std::time::Duration;

    /// Test basic HTTP provider creation
    #[test]
    fn test_http_provider_creation() {
        let provider = HttpConfigProvider::new("https://example.com/config");
        assert_eq!(provider.name(), "http");
        assert!(provider.is_available());
    }

    /// Test HTTP provider with URL
    #[test]
    fn test_http_provider_with_url() {
        let provider = HttpConfigProvider::new("https://config.example.com/app.yaml");
        assert!(provider.priority() > 0);
    }

    /// Test HTTP provider with timeout
    #[test]
    fn test_http_provider_with_timeout() {
        let provider = HttpConfigProvider::new("https://example.com/config")
            .with_timeout(Duration::from_secs(30));
        assert!(provider.is_available());
    }

    /// Test HTTP provider with auth
    #[test]
    fn test_http_provider_with_auth() {
        let provider =
            HttpConfigProvider::new("https://example.com/config").with_auth("user", "password");
        assert!(provider.is_available());
    }

    /// Test HTTP provider with bearer token
    #[test]
    fn test_http_provider_with_bearer_token() {
        let provider = HttpConfigProvider::new("https://example.com/config")
            .with_bearer_token("test-token-12345");
        assert!(provider.is_available());
    }

    /// Test HTTP provider priority setting
    #[test]
    fn test_http_provider_priority() {
        let provider1 = HttpConfigProvider::new("https://example.com/config").with_priority(10);
        let provider2 = HttpConfigProvider::new("https://example.com/config").with_priority(20);

        assert_eq!(provider1.priority(), 10);
        assert_eq!(provider2.priority(), 20);
    }

    /// Test HTTP provider metadata
    #[test]
    fn test_http_provider_metadata() {
        let provider = HttpConfigProvider::new("https://example.com/config");
        let metadata = provider.metadata();

        assert!(!metadata.supports_watch());
        assert!(metadata.is_remote());
    }

    /// Test HTTP provider with invalid URL (should still be created but fail at load)
    #[test]
    fn test_http_provider_invalid_url() {
        // Provider can be created with invalid URL, but will fail when loading
        let provider = HttpConfigProvider::new("not-a-valid-url");
        assert_eq!(provider.name(), "http");
    }
}