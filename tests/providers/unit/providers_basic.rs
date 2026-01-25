// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试：提供者基础功能
//!
//! 测试配置提供者的基础功能

use confers::providers::HttpConfigProvider;
use std::time::Duration;

#[test]
fn test_provider_creation() {
    // 测试提供者创建
    let provider = HttpConfigProvider::new("https://example.com/config");
    assert_eq!(provider.name(), "http");
    assert!(provider.is_available());
}

#[test]
fn test_provider_priority() {
    // 测试提供者优先级
    let provider1 = HttpConfigProvider::new("https://example.com/config").with_priority(10);
    let provider2 = HttpConfigProvider::new("https://example.com/config").with_priority(20);

    assert_eq!(provider1.priority(), 10);
    assert_eq!(provider2.priority(), 20);
}

#[test]
fn test_provider_metadata() {
    // 测试提供者元数据
    let provider = HttpConfigProvider::new("https://example.com/config");
    let metadata = provider.metadata();

    assert!(!metadata.supports_watch());
    assert!(metadata.is_remote());
}

#[test]
fn test_provider_with_url() {
    // 测试带 URL 的提供者
    let provider = HttpConfigProvider::new("https://config.example.com/app.yaml");
    assert!(provider.priority() > 0);
}

#[test]
fn test_provider_with_timeout() {
    // 测试带超时的提供者
    let provider =
        HttpConfigProvider::new("https://example.com/config").with_timeout(Duration::from_secs(30));
    assert!(provider.is_available());
}

#[test]
fn test_provider_with_auth() {
    // 测试带认证的提供者
    let provider =
        HttpConfigProvider::new("https://example.com/config").with_auth("user", "password");
    assert!(provider.is_available());
}

#[test]
fn test_provider_with_bearer_token() {
    // 测试带 bearer token 的提供者
    let provider =
        HttpConfigProvider::new("https://example.com/config").with_bearer_token("test-token-12345");
    assert!(provider.is_available());
}

#[test]
fn test_provider_invalid_url() {
    // 测试无效 URL 的提供者
    let provider = HttpConfigProvider::new("not-a-valid-url");
    assert_eq!(provider.name(), "http");
}

#[test]
fn test_provider_type() {
    // 测试提供者类型
    let provider = HttpConfigProvider::new("https://example.com/config");
    assert_eq!(
        provider.provider_type(),
        confers::providers::provider::ProviderType::Http
    );
}
