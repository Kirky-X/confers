// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(
    // 注意：在实际生产环境中，请替换为真实的远程配置 URL
    remote = "https://example.com/config",
    // 注意：以下路径需要替换为实际证书文件的绝对路径
    remote_ca_cert = "/path/to/ca.crt",
    remote_client_cert = "/path/to/client.crt",
    remote_client_key = "/path/to/client.key"
)]
struct TlsConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub value: i32,
}

#[cfg(feature = "remote")]
fn demonstrate_watcher_tls() {
    println!("Testing TLS configuration with ConfigWatcher...");

    // 注意：以下路径需要替换为实际证书文件的绝对路径
    let tls_config = confers::watcher::TlsConfig {
        ca_cert_path: Some("/path/to/ca.crt".to_string()),
        client_cert_path: Some("/path/to/client.crt".to_string()),
        client_key_path: Some("/path/to/client.key".to_string()),
    };

    println!("TLS配置创建成功！");
    println!("CA证书路径: {:?}", tls_config.ca_cert_path);
    println!("客户端证书路径: {:?}", tls_config.client_cert_path);
    println!("客户端密钥路径: {:?}", tls_config.client_key_path);

    let _watcher = confers::watcher::ConfigWatcher::new_remote(
        "https://example.com/config",
        Duration::from_secs(60),
    )
    .with_tls_config(tls_config);

    println!("配置观察器与TLS配置集成成功！");
}

fn demonstrate_macro_tls() {
    println!("\nTesting macro-level TLS configuration...");

    match TlsConfig::load() {
        Ok(config) => {
            println!("Config loaded successfully: {:?}", config);
        }
        Err(e) => {
            println!("Expected error (no valid certs): {}", e);
        }
    }
}

#[cfg(feature = "remote")]
fn main() {
    tracing_subscriber::fmt::init();

    demonstrate_watcher_tls();
    demonstrate_macro_tls();

    println!("\n远程配置TLS支持实现完成！");
}

#[cfg(not(feature = "remote"))]
fn main() {
    println!("Please run with --features remote");
    println!("Testing macro-level TLS configuration...");

    demonstrate_macro_tls();
}
