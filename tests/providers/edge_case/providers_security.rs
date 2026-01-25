// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 边界测试：提供者安全功能
//!
//! 测试提供者的安全功能，如 SSRF 保护

#[cfg(test)]
mod tests {
    use confers::providers::HttpConfigProvider;

    /// Test HTTP provider with localhost URL (should be blocked by SSRF protection)
    #[test]
    fn test_http_provider_localhost_blocked() {
        // Localhost URLs should be blocked by SSRF protection in load()
        let provider = HttpConfigProvider::new("http://127.0.0.1:8080/config");
        let result = provider.load();
        assert!(result.is_err());
    }

    /// Test HTTP provider with private IP blocked
    #[test]
    fn test_http_provider_private_ip_blocked() {
        // Private IPs should be blocked by SSRF protection
        let provider = HttpConfigProvider::new("http://192.168.1.1/config");
        let result = provider.load();
        assert!(result.is_err());
    }
}
