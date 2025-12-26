#[cfg(feature = "remote")]
use confers::providers::remote::consul::ConsulProvider;
#[cfg(feature = "remote")]
use confers::providers::remote::etcd::EtcdProvider;
#[cfg(feature = "remote")]
use serde_json::Value;
#[cfg(feature = "remote")]
use std::process::Command;
#[cfg(feature = "remote")]
use std::thread;
#[cfg(feature = "remote")]
use std::time::Duration;

#[tokio::test]
#[cfg(feature = "remote")]
#[ignore = "Requires Docker"]
async fn test_etcd_provider() {
    let container_name = "etcd-test-confers";
    let _ = Command::new("docker")
        .args(["rm", "-f", container_name])
        .output();

    println!("Starting Etcd container...");
    let status = Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            container_name,
            "-p",
            "2379:2379",
            "-e",
            "ALLOW_NONE_AUTHENTICATION=yes",
            "bitnami/etcd:latest",
        ])
        .status()
        .expect("Failed to start docker");

    if !status.success() {
        panic!("Failed to start Etcd container. Ensure Docker is running.");
    }

    thread::sleep(Duration::from_secs(10));

    let endpoints = vec!["127.0.0.1:2379".to_string()];

    let mut attempts = 0;
    loop {
        match etcd_client::Client::connect(&endpoints, None).await {
            Ok(mut client) => {
                let json_config = r#"{"database": {"host": "etcd-host", "port": 5432}}"#;
                client
                    .put("config/app", json_config, None)
                    .await
                    .expect("Failed to put key");
                break;
            }
            Err(e) => {
                attempts += 1;
                if attempts > 5 {
                    let _ = Command::new("docker")
                        .args(["rm", "-f", container_name])
                        .output();
                    panic!("Failed to connect to Etcd after retries: {}", e);
                }
                thread::sleep(Duration::from_secs(2));
            }
        }
    }

    let provider = EtcdProvider::new(endpoints, "config/app");
    let figment = figment::Figment::from(provider);
    let val: Value = figment.extract().expect("Failed to extract config");

    assert_eq!(val["database"]["host"], "etcd-host");
    assert_eq!(val["database"]["port"], 5432);

    let _ = Command::new("docker")
        .args(["rm", "-f", container_name])
        .output();
}

#[tokio::test]
#[cfg(feature = "remote")]
async fn test_etcd_provider_builder() {
    let provider = EtcdProvider::new(vec!["http://localhost:2379".to_string()], "config/app")
        .with_auth("user", "password")
        .with_tls(
            Some("/path/to/ca.pem".to_string()),
            Some("/path/to/cert.pem".to_string()),
            Some("/path/to/key.pem".to_string()),
        );

    let _ = figment::Figment::from(provider);
}

#[tokio::test]
#[cfg(feature = "remote")]
#[ignore = "Requires Docker"]
async fn test_consul_provider() {
    let container_name = "consul-test-confers";
    let _ = Command::new("docker")
        .args(["rm", "-f", container_name])
        .output();

    println!("Starting Consul container...");
    let status = Command::new("docker")
        .args([
            "run",
            "-d",
            "--name",
            container_name,
            "-p",
            "8500:8500",
            "consul",
        ])
        .status()
        .expect("Failed to start docker");

    if !status.success() {
        panic!("Failed to start Consul container. Ensure Docker is running.");
    }

    thread::sleep(Duration::from_secs(10));

    let client = reqwest::Client::new();
    let json_config = r#"{"database": {"host": "consul-host", "port": 5432}}"#;

    let resp = client
        .put("http://127.0.0.1:8500/v1/kv/config/app")
        .body(json_config)
        .send()
        .await
        .expect("Failed to send request to Consul");

    if !resp.status().is_success() {
        panic!("Failed to put key to Consul: {}", resp.status());
    }

    let provider = ConsulProvider::new("http://127.0.0.1:8500", "config/app");
    let figment = figment::Figment::from(provider);
    let val: Value = figment.extract().expect("Failed to extract config");

    assert_eq!(val["database"]["host"], "consul-host");
    assert_eq!(val["database"]["port"], 5432);

    let _ = Command::new("docker")
        .args(["rm", "-f", container_name])
        .output();
}

#[tokio::test]
#[cfg(feature = "remote")]
async fn test_consul_provider_builder() {
    let provider =
        ConsulProvider::new("http://localhost:8500", "config/app").with_token("my-secret-token");

    let _ = figment::Figment::from(provider);
}
