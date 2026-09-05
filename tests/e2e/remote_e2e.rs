// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 远程源(etcd / Consul / HTTP 轮询)(tests/e2e/remote_e2e.rs)
//!
//! 依赖 docker compose 服务(confers/docker-compose.test.yml):
//! - etcd  127.0.0.1:2379
//! - Consul 127.0.0.1:8500
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.17–2.19):
//! - REM-09 轮询级熔断:不可达目标连续失败达 `circuit_breaker_threshold`
//!   → 快速失败(`CircuitBreakerOpen`),不再发起网络请求
//! - REM-10 熔断半开:冷却后允许一次真实探测(错误类型从 CircuitBreakerOpen
//!   变回真实网络错误),失败后重新打开
//! - ETC-09 端到端:写 KV → poll 读到 → 改 KV → 再读到新值(远程热更语义)
//! - ETC-10 并发写多个 key 后轮询读取:prefix 树完整
//! - CSL-07 端到端:Consul 写 KV → poll 读到新值 → 删除 KV → 处理空配置
//! - CSL-08 DoS 防护:`max_kv_entries` 超限拒绝
//!
//! 偏差说明(见报告):
//! - REM-11(轮询内容变化生效)由 ETC-09/CSL-07 以受支持的远程源承载:
//!   HttpPolledSource 仅允许 HTTPS 且 SSRF 默认阻断回环地址,
//!   无法以本地 Consul(127.0.0.1:8500,明文 HTTP)作轮询端点。
//! - REM-10 仅覆盖"半开探测 + 失败重开"语义;完整"恢复后重新可用"
//!   需要可恢复的真实 HTTPS 端点,沙箱内无法提供自签信任。

use confers::remote::{
    ConsulSourceBuilder, EtcdSourceBuilder, HttpPolledSourceBuilder, PolledSource,
};
use std::time::Duration;

fn unique(prefix: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;
    format!(
        "{prefix}-e2e-{}-{}",
        nanos,
        SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

async fn etcd_ready() -> bool {
    let resp = reqwest::get("http://127.0.0.1:2379/health").await;
    match resp {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    }
}

async fn consul_ready() -> bool {
    let resp = reqwest::get("http://127.0.0.1:8500/v1/status/leader").await;
    match resp {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    }
}

/// etcd v3 HTTP 网关写 KV(键/值均需 base64)。
async fn etcd_put(key: &str, value: &str) {
    use base64::Engine;
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "key": base64::engine::general_purpose::STANDARD.encode(key.as_bytes()),
        "value": base64::engine::general_purpose::STANDARD.encode(value.as_bytes()),
    });
    let resp = client
        .post("http://127.0.0.1:2379/v3/kv/put")
        .json(&body)
        .send()
        .await
        .expect("etcd put request");
    assert!(resp.status().is_success(), "etcd put must succeed");
}

async fn etcd_delete_prefix_keys(keys: &[String]) {
    use base64::Engine;
    let client = reqwest::Client::new();
    for key in keys {
        let body = serde_json::json!({
            "key": base64::engine::general_purpose::STANDARD.encode(key.as_bytes()),
        });
        let _ = client
            .post("http://127.0.0.1:2379/v3/kv/deleterange")
            .json(&body)
            .send()
            .await;
    }
}

/// REM-09:连续失败达阈值 → 熔断打开,后续 poll 快速失败。
#[tokio::test]
async fn rem09_circuit_breaker_opens_after_threshold_failures() {
    // localhost 在 allowed_domain 白名单内(跳过 SSRF IP 校验),
    // 端口 1 无监听 → 每次真实请求连接被拒。
    let source = HttpPolledSourceBuilder::new()
        .url("https://localhost:1/confers-e2e.toml")
        .allowed_domain("localhost")
        .timeout(Duration::from_secs(2))
        .circuit_breaker_threshold(3)
        .circuit_breaker_base_delay(Duration::from_millis(10_000))
        .circuit_breaker_max_delay(Duration::from_millis(10_000))
        .build()
        .expect("build passes validation (whitelisted domain)");

    for i in 0..3 {
        let err = source
            .poll()
            .await
            .expect_err("unreachable target must fail");
        assert!(
            matches!(err, confers::ConfigError::RemoteUnavailable { .. }),
            "poll {i} must be a real network error, got: {err:?}"
        );
    }

    // 阈值达到 → 快速失败,错误类型为 CircuitBreakerOpen(不再发起请求)。
    let err = source.poll().await.expect_err("circuit must be open");
    match err {
        confers::ConfigError::RemoteUnavailable {
            ref error_type,
            retryable: false,
        } => assert_eq!(error_type, "CircuitBreakerOpen"),
        other => panic!("expected CircuitBreakerOpen, got: {other:?}"),
    }
}

/// REM-10:冷却后熔断进入半开,放行一次真实探测;失败后重新打开。
#[tokio::test]
async fn rem10_half_open_probes_then_reopens_on_failure() {
    let source = HttpPolledSourceBuilder::new()
        .url("https://localhost:1/confers-e2e.toml")
        .allowed_domain("localhost")
        .timeout(Duration::from_secs(2))
        .circuit_breaker_threshold(2)
        .circuit_breaker_base_delay(Duration::from_millis(300))
        .circuit_breaker_max_delay(Duration::from_millis(300))
        .build()
        .unwrap();

    // 两次失败 → 熔断打开。
    for _ in 0..2 {
        let _ = source.poll().await.unwrap_err();
    }
    let err = source.poll().await.unwrap_err();
    assert!(
        matches!(&err, confers::ConfigError::RemoteUnavailable { error_type, .. }
            if error_type == "CircuitBreakerOpen"),
        "circuit must open right after threshold: {err:?}"
    );

    // 冷却期过后 → 半开放行一次真实探测(连接再次被拒,但错误是真实网络错误)。
    tokio::time::sleep(Duration::from_millis(350)).await;
    let err = source.poll().await.unwrap_err();
    assert!(
        matches!(&err, confers::ConfigError::RemoteUnavailable { error_type, .. }
            if error_type != "CircuitBreakerOpen"),
        "half-open must attempt the real request, got: {err:?}"
    );

    // 探测失败 → 熔断重新打开。
    let err = source.poll().await.unwrap_err();
    assert!(
        matches!(&err, confers::ConfigError::RemoteUnavailable { error_type, .. }
            if error_type == "CircuitBreakerOpen"),
        "failed probe must reopen the circuit: {err:?}"
    );
}

/// ETC-09:写 KV → poll 读到 → 改 KV → poll 读到新值。
#[tokio::test]
async fn etc09_etcd_kv_changes_reflected_across_polls() {
    assert!(etcd_ready().await, "etcd must be up (compose)");

    let prefix = unique("confers-etc09");
    // 值本身是 TOML:kv 组装时按内容解析为子树。
    let key = format!("{prefix}/config");
    etcd_put(&key, "host = \"first-host\"\n").await;

    let source = EtcdSourceBuilder::new()
        .endpoint("127.0.0.1:2379")
        .prefix(&prefix)
        .interval(Duration::from_secs(60))
        .build()
        .await
        .expect("etcd source builds");

    let first = source.poll().await.expect("first poll must read config");
    assert_eq!(first.to_json()["config"]["host"], "first-host");

    etcd_put(&key, "host = \"second-host\"\n").await;
    let second = source.poll().await.expect("second poll");
    assert_eq!(
        second.to_json()["config"]["host"],
        "second-host",
        "KV change must be visible on the next poll"
    );

    etcd_delete_prefix_keys(&[key]).await;
}

/// ETC-10:并发写多个 key 后轮询读取,prefix 树完整。
#[tokio::test]
async fn etc10_concurrent_kv_writes_yield_complete_prefix_tree() {
    assert!(etcd_ready().await, "etcd must be up (compose)");

    let prefix = unique("confers-etc10");
    let keys: Vec<String> = (0..10).map(|i| format!("{prefix}/svc/key{i}")).collect();

    let writes = keys.iter().enumerate().map(|(i, key)| {
        let key = key.clone();
        async move { etcd_put(&key, &format!("value-{i}")).await }
    });
    futures_util::future::join_all(writes).await;

    let source = EtcdSourceBuilder::new()
        .endpoint("127.0.0.1:2379")
        .prefix(&prefix)
        .build()
        .await
        .unwrap();

    let value = source.poll().await.expect("poll after concurrent writes");
    let json = value.to_json();
    for i in 0..10 {
        // 纯文本值的键保留斜杠扁平键语义(src/remote/etcd.rs poll_internal)。
        assert_eq!(
            json[&format!("svc/key{i}")],
            format!("value-{i}"),
            "key{i} must be present and correct"
        );
    }

    etcd_delete_prefix_keys(&keys).await;
}

/// CSL-07:Consul 写 KV → poll 读到 → 删除 KV → 空配置处理。
#[tokio::test]
async fn csl07_consul_kv_write_poll_delete() {
    assert!(consul_ready().await, "consul must be up (compose)");

    let prefix = unique("confers/csl07");
    let client = reqwest::Client::new();
    let put = |key: String, value: String| {
        let client = client.clone();
        async move {
            let resp = client
                .put(format!("http://127.0.0.1:8500/v1/kv/{key}"))
                .body(value)
                .send()
                .await
                .expect("consul put");
            assert!(resp.status().is_success());
        }
    };
    put(format!("{prefix}/app/host"), "consul-host".into()).await;
    put(format!("{prefix}/app/port"), "8500".into()).await;

    let source = ConsulSourceBuilder::new()
        .address("127.0.0.1:8500")
        .prefix(&prefix)
        .interval(Duration::from_secs(60))
        .build()
        .expect("consul source builds");

    let value = source.poll().await.expect("poll must read KV tree");
    assert_eq!(value.to_json()["app/host"], "consul-host");
    assert_eq!(value.to_json()["app/port"], "8500");

    // 删除 KV → 后续 poll 反映空配置(报错或空树,不得 panic 或返回旧值)。
    for key in [format!("{prefix}/app/host"), format!("{prefix}/app/port")] {
        let resp = client
            .delete(format!("http://127.0.0.1:8500/v1/kv/{key}"))
            .send()
            .await
            .expect("consul delete");
        assert!(resp.status().is_success());
    }

    let after = source.poll().await;
    match after {
        Ok(v) => assert!(
            v.to_json().get("app/host").is_none(),
            "deleted keys must not resurface: {:?}",
            v.to_json()
        ),
        Err(e) => {
            assert!(
                !e.to_string().to_lowercase().contains("consul-host"),
                "error must not leak stale values: {e}"
            );
        }
    }
}

/// CSL-08:DoS 防护 —— max_kv_entries 超限拒绝。
#[tokio::test]
async fn csl08_max_kv_entries_rejects_oversized_response() {
    assert!(consul_ready().await, "consul must be up (compose)");

    let prefix = unique("confers/csl08");
    let client = reqwest::Client::new();
    for i in 0..4 {
        let resp = client
            .put(format!("http://127.0.0.1:8500/v1/kv/{prefix}/app/k{i}"))
            .body(format!("v{i}"))
            .send()
            .await
            .expect("consul put");
        assert!(resp.status().is_success());
    }

    let source = ConsulSourceBuilder::new()
        .address("127.0.0.1:8500")
        .prefix(&prefix)
        .max_kv_entries(2)
        .build()
        .expect("source builds");

    let err = source
        .poll()
        .await
        .expect_err("4 keys over max_kv_entries=2 must be rejected");
    assert!(
        matches!(
            err,
            confers::ConfigError::SizeLimitExceeded { .. }
                | confers::ConfigError::InvalidValue { .. }
                | confers::ConfigError::RemoteUnavailable { .. }
        ),
        "expected a limit-rejection error, got: {err:?}"
    );

    for i in 0..4 {
        let _ = client
            .delete(format!("http://127.0.0.1:8500/v1/kv/{prefix}/app/k{i}"))
            .send()
            .await;
    }
}
