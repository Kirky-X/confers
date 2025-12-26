use std::net::SocketAddr;

use serde_json::json;
use tokio::time::{timeout, Duration};
use warp::Filter;

#[cfg(feature = "remote")]
use confers::providers::remote::http::HttpProvider;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TestConfig {
    server_port: u16,
    server_host: String,
    db_password: String,
    db_url: String,
}

#[cfg(feature = "remote")]
async fn start_json_server(port: u16, config: serde_json::Value) -> SocketAddr {
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let config_clone1 = config.clone();
    let config_clone2 = config.clone();

    let routes = warp::path("etcd-config.json")
        .map(move || warp::reply::json(&config))
        .or(warp::path("consul-config.json").map(move || warp::reply::json(&config_clone1)))
        .or(warp::path("config.json").map(move || warp::reply::json(&config_clone2)));

    tokio::spawn(warp::serve(routes).run(addr));

    addr
}

#[cfg(feature = "remote")]
async fn start_invalid_server(port: u16) -> SocketAddr {
    let invalid_config = json!({
        "server_port": 70000,
        "server_host": "test-server.example",
        "db_password": "test_db_password",
        "db_url": "postgres://test-db:5432/app"
    });

    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let routes = warp::path("invalid-config.json").map(move || warp::reply::json(&invalid_config));

    tokio::spawn(warp::serve(routes).run(addr));

    addr
}

#[cfg(feature = "remote")]
#[tokio::test]
async fn test_etcd_like_real_server() {
    let port = 18701;
    let config_response = json!({
        "server_port": 9000,
        "server_host": "etcd-server.example",
        "db_password": "etcd_db_password",
        "db_url": "postgres://etcd-db:5432/production"
    });

    let addr = start_json_server(port, config_response).await;
    let url = format!("http://{}/etcd-config.json", addr);

    let figment = tokio::task::spawn_blocking(move || {
        let p = HttpProvider::new(url);
        p.load_sync()
    })
    .await
    .unwrap()
    .unwrap();

    let config: TestConfig = figment
        .extract()
        .expect("Failed to extract config from real etcd-like server");

    assert_eq!(config.server_port, 9000);
    assert_eq!(config.server_host, "etcd-server.example");
    assert_eq!(config.db_password, "etcd_db_password");
    assert_eq!(config.db_url, "postgres://etcd-db:5432/production");
}

#[cfg(feature = "remote")]
#[tokio::test]
async fn test_consul_like_real_server() {
    let port = 18702;
    let config_response = json!({
        "server_port": 9000,
        "server_host": "consul-server.example",
        "db_password": "consul_db_password",
        "db_url": "postgres://consul-db:5432/production"
    });

    let addr = start_json_server(port, config_response).await;
    let url = format!("http://{}/consul-config.json", addr);

    let figment = tokio::task::spawn_blocking(move || {
        let p = HttpProvider::new(url);
        p.load_sync()
    })
    .await
    .unwrap()
    .unwrap();

    let config: TestConfig = figment
        .extract()
        .expect("Failed to extract config from real consul-like server");

    assert_eq!(config.server_port, 9000);
    assert_eq!(config.server_host, "consul-server.example");
    assert_eq!(config.db_password, "consul_db_password");
    assert_eq!(config.db_url, "postgres://consul-db:5432/production");
}

#[cfg(feature = "remote")]
#[tokio::test]
async fn test_remote_config_with_validation_error() {
    let port = 18703;
    let _addr = start_invalid_server(port).await;
    let url = format!("http://127.0.0.1:{}/invalid-config.json", port);

    let figment = tokio::task::spawn_blocking(move || {
        let p = HttpProvider::new(url);
        p.load_sync()
    })
    .await
    .unwrap()
    .unwrap();

    let result = figment.extract::<TestConfig>();
    assert!(
        result.is_err(),
        "Should fail due to invalid port validation (port 70000 > 65535)"
    );
}

#[cfg(feature = "remote")]
#[tokio::test]
async fn test_remote_config_connection_failure() {
    let provider = HttpProvider::new("http://10.255.255.1:9999/config.json");

    let result = timeout(Duration::from_secs(2), async {
        tokio::task::spawn_blocking(move || provider.load_sync()).await
    })
    .await;

    match result {
        Ok(Ok(_)) => panic!("Should fail due to connection error"),
        Ok(Err(_)) => (),
        Err(_) => (),
    }
}

#[cfg(feature = "remote")]
#[tokio::test]
async fn test_http_provider_real_server() {
    let port = 18704;
    let config_response = json!({
        "server_port": 8080,
        "server_host": "real-server.example.com",
        "db_password": "real_password",
        "db_url": "postgres://real-server:5432/real_db"
    });

    let addr = start_json_server(port, config_response).await;
    let url = format!("http://{}/config.json", addr);

    let figment = tokio::task::spawn_blocking(move || {
        let p = HttpProvider::new(url);
        p.load_sync()
    })
    .await
    .unwrap()
    .unwrap();

    let config: TestConfig = figment
        .extract()
        .expect("Failed to extract config from real HTTP server");

    assert_eq!(config.server_port, 8080);
    assert_eq!(config.server_host, "real-server.example.com");
    assert_eq!(config.db_password, "real_password");
    assert_eq!(config.db_url, "postgres://real-server:5432/real_db");
}

#[cfg(feature = "remote")]
#[tokio::test]
async fn test_http_provider_invalid_json() {
    use tokio::io::AsyncWriteExt;
    use tokio::net::TcpListener;

    let port = 18705;
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .await
        .unwrap();

    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            let (_, mut writer) = tokio::io::split(stream);
            let invalid_json = b"not valid json at all {";
            writer.write_all(invalid_json).await.unwrap();
            writer.shutdown().await.unwrap();
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let provider = HttpProvider::new(format!("http://127.0.0.1:{}/invalid.json", port));
    let result = tokio::task::spawn_blocking(move || provider.load_sync())
        .await
        .unwrap();

    assert!(result.is_err(), "Should fail due to invalid JSON response");
}

#[cfg(feature = "remote")]
#[tokio::test]
async fn test_error_server_response() {
    use warp::http::StatusCode;
    use warp::reply;

    let port = 18706;
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let routes = warp::path("error-config.json")
        .map(|| reply::with_status("Internal Server Error", StatusCode::INTERNAL_SERVER_ERROR));

    tokio::spawn(warp::serve(routes).run(addr));

    tokio::time::sleep(Duration::from_millis(100)).await;

    let provider = HttpProvider::new(format!("http://{}/error-config.json", addr));
    let result = tokio::task::spawn_blocking(move || provider.load_sync())
        .await
        .unwrap();

    assert!(result.is_err(), "Should fail due to server error");
}
