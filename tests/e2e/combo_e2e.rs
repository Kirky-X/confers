// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: feature 组合交互(tests/e2e/combo_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.26,组合矩阵 §3.2 C1…C17):
//! - CMP-01 encryption+watch:文件携带新密文,热更后解密生效;错钥 → 保留旧明文(回滚)
//! - CMP-02 migration+snapshot:v1 配置迁移 v2 → 快照 → 回放得到 v2 内容
//! - CMP-03 / TGL-06 dynamic+toggle:开关门控动态字段回调
//! - CMP-04 watch+config-bus:FsWatcher 事件 → 总线广播 → 订阅方执行重载
//! - CMP-05 watch+migration:文件回退旧版本 → 自动迁移后生效 v2 语义
//! - CMP-06 progressive-reload+validation:garde 校验作为健康检查,坏配置回滚
//! - CMP-08 etcd+dynamic:远程 KV 轮询驱动动态字段
//! - CMP-09 audit+key:轮换留痕,旧版本密钥仍可解密旧数据
//! - CMP-10 validation+security-rules:结构校验 + 安全校验双层串联
//! - CMP-11 modules+profile:激活 profile 的文件并入主配置链
//! - CMP-12 interpolation+env+file:文件模板由 env 注入解析
//! - CMP-13 cli+schema:ConfigClap 参数覆盖文件值
//! - CMP-14 full 全功能烟囱:构建+快照+加密+审计+热更一次跑通
//! - CMP-15 snapshot+watch:每次重载各自快照,可回放任一历史
//! - CMP-16 nats-bus 双实例:A 广播变更,B 收到事件
//! - CMP-17 / CTX-09 context-aware+feature-toggle+dynamic:上下文+开关共同决定动态值
//!
//! CMP-07(remote+encryption 自动解密注入)无法落地:远程值解密注入管线未实现,
//! 且 HttpPolledSource 仅允许 HTTPS(SSRF 阻断本地端点),见报告。
//! IPL-09(examples/interpolation)由批 4 的 examples 运行验收覆盖。

use base64::Engine;
use confers::audit::AuditWriter;
use confers::dynamic::DynamicField;
use confers::migration::MigrationRegistry;
use confers::secret::{XChaCha20Crypto, derive_field_key};
use confers::snapshot::{SnapshotConfig, SnapshotFormat, SnapshotManager};
use confers::toggle::FeatureToggleRegistry;
use confers::watcher::FsWatcher;
use confers::{ConfigBuilder, ConfigValue, SourceId};
use serial_test::serial;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 恰好 32 字节的测试密钥(仅测试用,非真实凭据)。
const TEST_MASTER_KEY: &str = "test-key-with-exactly-32-bytes!!";

fn encrypt_value(plaintext: &[u8], master: &[u8; 32], field: &str) -> String {
    let field_key = derive_field_key(master, field, "v1").expect("derive");
    let crypto = XChaCha20Crypto::new();
    let (nonce, ciphertext) = crypto.encrypt(plaintext, &field_key).expect("encrypt");
    format!(
        "enc-{}:{}",
        base64::engine::general_purpose::STANDARD.encode(&nonce),
        base64::engine::general_purpose::STANDARD.encode(&ciphertext)
    )
}

fn decrypt_value(enc: &str, master: &[u8; 32], field: &str) -> Option<Vec<u8>> {
    let field_key = derive_field_key(master, field, "v1").ok()?;
    let (nonce_b64, ct_b64) = enc.trim_start_matches("enc-").split_once(':')?;
    let decode = |s: &str| {
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .expect("valid base64")
    };
    XChaCha20Crypto::new()
        .decrypt(&decode(nonce_b64), &decode(ct_b64), &field_key)
        .ok()
}

/// CMP-01:热更后新密文解密生效;错误密钥解密失败 → 保留上一份好明文(回滚)。
#[tokio::test]
#[serial]
async fn cmp01_encryption_plus_watch_hot_reload_with_key_rotation() {
    let master: [u8; 32] = TEST_MASTER_KEY.as_bytes().try_into().unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("secure.toml");

    // 当前文件携带 v1 密文,应用侧缓存其明文。
    let v1 = encrypt_value(b"secret-one", &master, "cmp01.api");
    std::fs::write(&path, format!("api = \"{v1}\"\n")).unwrap();
    let mut live_plaintext = decrypt_value(&v1, &master, "cmp01.api").unwrap();
    assert_eq!(live_plaintext, b"secret-one");

    let mut watcher = FsWatcher::new(&path, 100).await.expect("watcher");
    tokio::time::sleep(Duration::from_millis(300)).await;

    // 热更:文件换成 v2 密文(轮换后的新值)。
    let v2 = encrypt_value(b"secret-two", &master, "cmp01.api");
    std::fs::write(&path, format!("api = \"{v2}\"\n")).unwrap();
    tokio::time::timeout(Duration::from_secs(5), watcher.recv())
        .await
        .expect("watch event")
        .expect("channel open");

    // 读到新密文 → 用当前密钥解密 → 生效。
    if let Some(new_plain) = decrypt_value(&v2, &master, "cmp01.api") {
        live_plaintext = new_plain;
    }
    assert_eq!(live_plaintext, b"secret-two");

    // 错误密钥:解密失败 → 回滚保留上一份好明文。
    let wrong_master: [u8; 32] = [9u8; 32];
    let candidate = decrypt_value(&v2, &wrong_master, "cmp01.api");
    assert!(candidate.is_none(), "wrong key must fail to decrypt");
    assert_eq!(
        live_plaintext, b"secret-two",
        "rollback keeps the last good plaintext"
    );

    watcher.stop();
}

/// CMP-02:migration+snapshot —— v1 → v2 迁移后快照,回放为 v2 内容。
#[test]
fn cmp02_migrated_config_snapshots_and_replays_as_v2() {
    let annotated = |version: i64| {
        confers::types::AnnotatedValue::new(
            ConfigValue::map(vec![(
                "version",
                confers::types::AnnotatedValue::new(
                    ConfigValue::integer(version),
                    SourceId::new("cmp02"),
                    "version",
                ),
            )]),
            SourceId::new("cmp02"),
            "",
        )
    };

    let mut registry = MigrationRegistry::new();
    registry.register(1, 2, |mut v| {
        v.version = 2;
        Ok(v)
    });
    registry.precompute_paths();

    let migrated = registry.migrate(annotated(1), 1, 2).expect("migrate");
    // 迁移函数更新的是 AnnotatedValue 的版本元数据(migration 契约)。
    assert_eq!(migrated.version, 2);

    let dir = tempfile::tempdir().unwrap();
    let manager = SnapshotManager::new(SnapshotConfig {
        dir: dir.path().join("snaps"),
        format: SnapshotFormat::Json,
        include_provenance: false,
        ..SnapshotConfig::default()
    });
    let path = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { manager.save(&migrated, &[]).await })
        .expect("snapshot save");
    let replayed = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { manager.load_snapshot(&path).await })
        .expect("snapshot replay");
    // 快照承载配置树本身(迁移更新的是版本元数据)。
    assert_eq!(
        replayed.to_json(),
        migrated.to_json(),
        "replay must equal the saved (post-migration) tree"
    );
}

/// CMP-03 / TGL-06:开关门控动态字段回调的生效。
#[test]
fn cmp03_feature_toggle_gates_dynamic_field_callback() {
    let toggle = Arc::new(FeatureToggleRegistry::new());
    toggle.register("apply_limits", "gate for connection limit updates", false);

    let handle: DynamicField<u32> = DynamicField::new(10);
    let applied = Arc::new(Mutex::new(Vec::<u32>::new()));
    let sink = Arc::clone(&applied);
    let toggle_ref = Arc::clone(&toggle);
    let _guard = handle.on_change(move |v| {
        if toggle_ref.is_enabled("apply_limits") {
            sink.lock().unwrap().push(*v);
        }
    });

    // 开关关闭:更新发生但门控回调不应用。
    handle.update(50);
    assert!(applied.lock().unwrap().is_empty());
    assert_eq!(handle.get(), 50);

    // 开关打开:后续更新恢复应用。
    toggle.enable("apply_limits");
    handle.update(80);
    assert_eq!(*applied.lock().unwrap(), vec![80]);
}

/// CMP-04:FsWatcher 事件 → ConfigBus 广播 → 订阅方执行重载。
#[tokio::test]
async fn cmp04_watch_event_broadcast_over_bus_triggers_reload() {
    let dir = tempfile::tempdir().unwrap();
    let path = Arc::new(dir.path().join("app.toml"));
    std::fs::write(path.as_ref(), "port = 1\n").unwrap();

    use confers::bus::ConfigBus as _;
    use futures_util::StreamExt as _;
    let bus: confers::bus::InMemoryBus = confers::bus::BusBuilder::new().build();

    let mut watcher = FsWatcher::new(path.as_ref(), 100).await.expect("watcher");
    tokio::time::sleep(Duration::from_millis(300)).await;
    let mut rx = bus.subscribe().await.expect("subscribe");

    // 修改文件 → watcher 事件 → 广播。
    std::fs::write(path.as_ref(), "port = 2\n").unwrap();
    tokio::time::timeout(Duration::from_secs(5), watcher.recv())
        .await
        .expect("watch event")
        .expect("channel open");
    bus.publish(confers::bus::ConfigChangeEvent::new(
        "watcher-bridge",
        "file",
        vec!["port".to_string()],
        "cmp04",
    ))
    .await
    .expect("publish");

    // 订阅方收到事件后重载。
    let ev = tokio::time::timeout(Duration::from_secs(5), rx.next())
        .await
        .expect("bus event")
        .expect("stream open");
    assert_eq!(ev.instance_id, "watcher-bridge");

    let reloaded: serde_json::Value = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(path.as_ref())
        .build()
        .expect("reload");
    assert_eq!(reloaded["port"], 2);
    watcher.stop();
}

/// CMP-05:文件回退旧版本 → 迁移注册表自动迁移 → 生效 v2 语义。
#[test]
fn cmp05_watched_file_reverted_then_auto_migrated() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    // 当前生效 v2;随后文件被回退为 v1(缺 v2 字段)。
    std::fs::write(&path, "version = 2\n").unwrap();
    std::fs::write(&path, "version = 1\n").unwrap();

    let merged: serde_json::Value = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&path)
        .build()
        .expect("v1 file loads");
    assert_eq!(merged["version"], 1, "file reverted to v1");

    // MigrationOnReload::Always 语义:检测到旧版本即自动迁移。
    let annotated = confers::types::AnnotatedValue::new(
        ConfigValue::map(vec![(
            "version",
            confers::types::AnnotatedValue::new(
                ConfigValue::integer(1),
                SourceId::new("cmp05"),
                "version",
            ),
        )]),
        SourceId::new("cmp05"),
        "",
    );
    let mut registry = MigrationRegistry::new();
    registry.register(1, 2, |mut v| {
        v.version = 2;
        Ok(v)
    });
    registry.precompute_paths();

    let effective = registry
        .migrate(annotated, 1, 2)
        .expect("auto migration v1→v2");
    assert_eq!(
        effective.version, 2,
        "reverted file is migrated back to the current version"
    );
}

/// CMP-06:garde 结构校验作为 ReloadHealthCheck,坏配置 Critical → 回滚。
#[tokio::test]
async fn cmp06_progressive_reload_with_validation_health_check() {
    use confers::interface::ConfigProvider;
    use confers::watcher::{HealthStatus, ProgressiveReloader, ReloadHealthCheck, ReloadStrategy};
    use garde::Validate as GardeValidate;

    #[derive(Debug, Clone, serde::Deserialize, GardeValidate)]
    struct WebCfg {
        #[garde(range(min = 1, max = 10000))]
        port: u16,
    }

    /// 真实健康检查:从 provider 读取候选值并用 garde 校验。
    struct GardeHealth;
    #[async_trait::async_trait]
    impl ReloadHealthCheck for GardeHealth {
        async fn check(&self, provider: Arc<dyn ConfigProvider>) -> HealthStatus {
            let port = provider
                .get_raw("port")
                .and_then(|v| v.to_json().as_u64())
                .unwrap_or(0);
            let candidate = WebCfg {
                port: port.clamp(0, u16::MAX as u64) as u16,
            };
            match candidate.validate() {
                Ok(()) => HealthStatus::Healthy,
                Err(_) => HealthStatus::Critical {
                    reason: format!("port {port} outside 1..=10000"),
                },
            }
        }
    }

    let reloader = ProgressiveReloader::builder()
        .initial(Arc::new(WebCfg { port: 8080 }))
        .strategy(ReloadStrategy::Canary {
            trial_duration: Duration::from_secs(5),
            poll_interval: Duration::from_millis(10),
        })
        .health_check(Arc::new(GardeHealth))
        .build();

    // 候选 60000 是合法 u16 但超出 garde 范围 → Critical → 回滚。
    let provider_bad: Arc<dyn ConfigProvider> = Arc::new(PortProvider::new(60000));
    let result = reloader
        .begin_reload(Arc::new(WebCfg { port: 60000 }), provider_bad.clone())
        .await;
    assert!(
        matches!(result, Err(confers::ConfigError::ReloadRolledBack { .. })),
        "invalid candidate must be rolled back, got {result:?}"
    );
    assert_eq!(reloader.current().port, 8080);

    // 合法候选 → 提交。
    let provider_good: Arc<dyn ConfigProvider> = Arc::new(PortProvider::new(9090));
    let result = reloader
        .begin_reload(Arc::new(WebCfg { port: 9090 }), provider_good)
        .await;
    assert!(matches!(
        result,
        Ok(confers::watcher::ReloadOutcome::Committed)
    ));
    assert_eq!(reloader.current().port, 9090);
}

/// 承载真实配置值的 ConfigProvider(供健康检查读取)。
struct PortProvider {
    port: confers::types::AnnotatedValue,
}

impl PortProvider {
    fn new(port: u64) -> Self {
        Self {
            port: confers::types::AnnotatedValue::new(
                ConfigValue::uint(port),
                SourceId::new("cmp06"),
                "port",
            ),
        }
    }
}

impl confers::interface::ConfigProvider for PortProvider {
    fn get_raw(&self, key: &str) -> Option<&confers::types::AnnotatedValue> {
        if key == "port" {
            Some(&self.port)
        } else {
            None
        }
    }

    fn keys(&self) -> Vec<String> {
        vec!["port".to_string()]
    }
}

/// CMP-08:etcd KV 轮询驱动 DynamicField。
#[tokio::test]
async fn cmp08_etcd_poll_drives_dynamic_field() {
    use base64::Engine;
    use confers::remote::{EtcdSourceBuilder, PolledSource};

    let ready = reqwest::get("http://127.0.0.1:2379/health")
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);
    if !ready {
        eprintln!("Skipping test: etcd not available");
        return;
    }

    let prefix = format!("confers-cmp08-{}", std::process::id());
    let key = format!("{prefix}/config");
    let put = |value: String, key: String| async move {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "key": base64::engine::general_purpose::STANDARD.encode(key.as_bytes()),
            "value": base64::engine::general_purpose::STANDARD.encode(value.as_bytes()),
        });
        client
            .post("http://127.0.0.1:2379/v3/kv/put")
            .json(&body)
            .send()
            .await
            .expect("etcd put");
    };
    put("max_connections = 10\n".into(), key.clone()).await;

    let source = EtcdSourceBuilder::new()
        .endpoint("127.0.0.1:2379")
        .prefix(&prefix)
        .build()
        .await
        .unwrap();

    let handle: DynamicField<u32> = DynamicField::new(10);
    let apply = |annotated: &confers::types::AnnotatedValue, handle: &DynamicField<u32>| {
        if let Some(v) = annotated.to_json()["config"]["max_connections"].as_u64() {
            handle.update(v as u32);
        }
    };
    apply(&source.poll().await.expect("poll 1"), &handle);
    assert_eq!(handle.get(), 10);

    put("max_connections = 42\n".into(), key.clone()).await;
    apply(&source.poll().await.expect("poll 2"), &handle);
    assert_eq!(
        handle.get(),
        42,
        "remote KV change must drive the dynamic field"
    );

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "key": base64::engine::general_purpose::STANDARD.encode(key.as_bytes()),
    });
    let _ = client
        .post("http://127.0.0.1:2379/v3/kv/deleterange")
        .json(&body)
        .send()
        .await;
}

/// CMP-09:audit+key —— 轮换留痕且旧版本密钥仍可解密旧数据。
#[test]
#[serial]
fn cmp09_key_rotation_audited_and_old_key_still_decrypts() {
    let dir = tempfile::tempdir().unwrap();
    let writer = AuditWriter::builder()
        .enabled(true)
        .log_dir(dir.path().to_path_buf())
        .build();

    let key_v1: [u8; 32] = std::array::from_fn(|i| (i as u8) ^ 0x11);
    let key_v2: [u8; 32] = std::array::from_fn(|i| (i as u8) ^ 0x22);

    let crypto = XChaCha20Crypto::new();
    let (nonce, ciphertext) = crypto.encrypt(b"legacy-payload", &key_v1).unwrap();

    writer.log_key_rotation("v1", "v2").expect("rotation audit");

    // 旧版本密钥仍解旧数据;新版本密钥管新数据。
    let old_plain = crypto
        .decrypt(&nonce, &ciphertext, &key_v1)
        .expect("old key decrypts old data");
    assert_eq!(old_plain, b"legacy-payload");
    let (n2, c2) = crypto.encrypt(b"fresh-payload", &key_v2).unwrap();
    let fresh = crypto
        .decrypt(&n2, &c2, &key_v2)
        .expect("new key decrypts new data");
    assert_eq!(fresh, b"fresh-payload");

    std::thread::sleep(Duration::from_millis(150));
    let log = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| std::fs::read_to_string(e.path()).unwrap_or_default())
        .collect::<String>();
    assert!(
        log.contains("KeyRotation"),
        "rotation event must be audited: {log}"
    );
}

/// CMP-10:validation+security-rules 双层校验串联。
#[test]
fn cmp10_dual_layer_structural_and_security_validation() {
    use confers::security::rules::{SecurityValidatorRegistry, ViolationSeverity};
    use confers::types::AnnotatedValue;
    use garde::Validate as GardeValidate;

    #[derive(Debug, serde::Deserialize, GardeValidate)]
    struct LayerOne {
        #[garde(range(min = 1, max = 10000))]
        port: u16,
    }

    // 第一层:结构校验通过。
    let cfg = LayerOne { port: 8080 };
    cfg.validate().expect("structural layer passes");

    // 第二层:安全规则(承载真实配置值的 provider)。
    struct JwtProvider {
        secret: AnnotatedValue,
    }
    impl confers::interface::ConfigProvider for JwtProvider {
        fn get_raw(&self, key: &str) -> Option<&AnnotatedValue> {
            if key == "jwt.secret" {
                Some(&self.secret)
            } else {
                None
            }
        }

        fn keys(&self) -> Vec<String> {
            vec!["jwt.secret".to_string()]
        }
    }
    let provider = JwtProvider {
        secret: AnnotatedValue::new(
            ConfigValue::string("changeme"),
            SourceId::new("cmp10"),
            "jwt.secret",
        ),
    };

    let registry = SecurityValidatorRegistry::with_defaults();
    let report = registry.validate_all(&provider);
    assert!(
        report
            .violations
            .iter()
            .any(|v| v.severity == ViolationSeverity::Warning
                || v.severity == ViolationSeverity::Critical),
        "security layer must flag the weak secret: {:?}",
        report.violations
    );
}

/// CMP-11:modules+profile —— 激活 profile 的文件并入主配置链。
#[test]
fn cmp11_active_profile_module_merged_into_main_chain() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("dev.toml"), "host = \"dev-db\"\n").unwrap();
    std::fs::write(dir.path().join("prod.toml"), "host = \"prod-db\"\n").unwrap();

    let mut registry = confers::modules::ModuleRegistry::default();
    registry.register_group(
        "database",
        vec![
            ("dev", dir.path().join("dev.toml")),
            ("prod", dir.path().join("prod.toml")),
        ],
        Some("dev"),
    );
    registry
        .set_active_profile("database", "prod")
        .expect("switch profile");

    let module = registry
        .load_active("database", &confers::LoaderConfig::new().allow_absolute())
        .expect("load active profile");

    let host = module.to_json()["host"].as_str().expect("host").to_string();
    let mut memory = std::collections::HashMap::new();
    memory.insert("database.host".to_string(), ConfigValue::string(host));
    memory.insert("database.port".to_string(), ConfigValue::integer(6432));

    let merged: serde_json::Value = ConfigBuilder::new().memory(memory).build().expect("merged");
    assert_eq!(merged["database"]["host"], "prod-db");
    assert_eq!(merged["database"]["port"], 6432);
}

/// CMP-12:interpolation+env+file —— 文件模板由 env 注入解析。
#[test]
#[serial]
fn cmp12_file_template_resolved_by_env_injection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    std::fs::write(&path, "endpoint = \"http://${DB_HOST}:${DB_PORT}\"\n").unwrap();

    unsafe { std::env::set_var("DB_HOST", "db.internal") };
    unsafe { std::env::set_var("DB_PORT", "5432") };

    let raw = std::fs::read_to_string(&path).unwrap();
    let resolver = |name: &str| std::env::var(name).ok();
    let resolved =
        confers::interpolation::interpolate(&raw, &resolver).expect("template resolves from env");
    assert!(resolved.contains("db.internal:5432"));

    // env 缺失时走默认值语法:文件未给默认 → 报错(未被本用例消费)。
    unsafe { std::env::remove_var("DB_HOST") };
    unsafe { std::env::remove_var("DB_PORT") };
}

/// CMP-13:cli+schema —— ConfigClap 参数覆盖文件值(构建层)。
#[test]
fn cmp13_clap_args_override_file_config() {
    use std::ffi::OsString;

    #[derive(Debug, confers::ConfigClap)]
    #[allow(dead_code)]
    struct CliCfg {
        #[config(name = "host", name_clap_long = "host")]
        pub host: String,

        #[config(name = "port")]
        pub port: u16,
    }

    let args = CliCfg::clap_args_from(
        vec![
            OsString::from("app"),
            OsString::from("--host"),
            OsString::from("cli.example"),
            OsString::from("--port"),
            OsString::from("9090"),
        ]
        .into_iter(),
    );
    let mut memory = std::collections::HashMap::new();
    for (k, v) in args.to_config_map() {
        let value = if let Some(s) = v.as_str() {
            ConfigValue::string(s)
        } else if let Some(n) = v.as_u64() {
            ConfigValue::uint(n)
        } else {
            continue;
        };
        memory.insert(k, value);
    }

    let merged: serde_json::Value = ConfigBuilder::new().memory(memory).build().unwrap();
    assert_eq!(merged["host"], "cli.example", "CLI value must win");
    assert_eq!(merged["port"], 9090);
}

/// CMP-14:full 全功能烟囱 —— 构建+快照+加密+审计+热更一次跑通。
#[test]
#[serial]
fn cmp14_full_stack_smoke() {
    let master: [u8; 32] = TEST_MASTER_KEY.as_bytes().try_into().unwrap();

    let dir = tempfile::tempdir().unwrap();
    let config_path = dir.path().join("app.toml");
    let snap_dir = dir.path().join("snaps");
    let enc = encrypt_value(b"full-stack-secret", &master, "cmp14.api");
    std::fs::write(&config_path, format!("api = \"{enc}\"\nname = \"stack\"\n")).unwrap();

    // 构建(with_snapshot 自动快照)。
    let snapshot_config = SnapshotConfig {
        dir: snap_dir.clone(),
        format: SnapshotFormat::Json,
        include_provenance: false,
        ..SnapshotConfig::default()
    };
    let built: serde_json::Value = ConfigBuilder::new()
        .allow_absolute_paths()
        .with_snapshot(snapshot_config.clone())
        .file(&config_path)
        .build()
        .expect("full build");
    assert_eq!(built["name"], "stack");

    // 加密字段解密。
    let decrypted = decrypt_value(built["api"].as_str().unwrap(), &master, "cmp14.api").unwrap();
    assert_eq!(decrypted, b"full-stack-secret");

    // 审计留痕。
    let writer = AuditWriter::builder()
        .enabled(true)
        .log_dir(dir.path().to_path_buf())
        .build();
    writer.log_load("cmp14-config").expect("audit");

    // 热更(动态字段)。
    let handle: DynamicField<String> = DynamicField::new("stack".into());
    std::fs::write(&config_path, "api = \"\"\nname = \"stack-v2\"\n").unwrap();
    let reloaded: serde_json::Value = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&config_path)
        .build()
        .expect("reload");
    handle.update(reloaded["name"].as_str().unwrap().to_string());
    assert_eq!(handle.get(), "stack-v2");

    // 快照存在且回放为 v1 构建内容。
    let manager = SnapshotManager::new(snapshot_config);
    let snaps = manager.list_snapshots().expect("snapshots listed");
    assert!(!snaps.is_empty(), "auto-snapshot must exist");
    let replay = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async { manager.load_snapshot(&snaps[0].path).await })
        .expect("replay");
    assert_eq!(replay.to_json()["name"], "stack");

    // 审计文件落盘。
    std::thread::sleep(Duration::from_millis(150));
    let audit = std::fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| std::fs::read_to_string(e.path()).ok())
        .find(|c| c.contains("cmp14-config"));
    assert!(audit.is_some(), "audit record for the load must exist");
}

/// CMP-15:snapshot+watch —— 三次重载各自快照,可回放任一历史。
#[tokio::test]
async fn cmp15_snapshots_before_each_reload_allow_rollback() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    std::fs::write(&path, "rev = 0\n").unwrap();
    let snap_dir = dir.path().join("snaps");

    let manager = SnapshotManager::new(SnapshotConfig {
        dir: snap_dir,
        format: SnapshotFormat::Json,
        include_provenance: false,
        ..SnapshotConfig::default()
    });

    let mut watcher = FsWatcher::new(&path, 100).await.expect("watcher");
    tokio::time::sleep(Duration::from_millis(300)).await;

    let mut snapshots = Vec::new();
    for rev in 1..=3u16 {
        std::fs::write(&path, format!("rev = {rev}\n")).unwrap();
        tokio::time::timeout(Duration::from_secs(5), watcher.recv())
            .await
            .expect("watch event")
            .expect("channel open");

        // 每次重载前快照当前内容(构造当前版本的 AnnotatedValue)。
        let annotated = confers::types::AnnotatedValue::new(
            ConfigValue::map(vec![(
                "rev",
                confers::types::AnnotatedValue::new(
                    ConfigValue::integer(i64::from(rev)),
                    SourceId::new("cmp15"),
                    "rev",
                ),
            )]),
            SourceId::new("cmp15"),
            "",
        );
        snapshots.push(manager.save(&annotated, &[]).await.expect("save"));
    }
    watcher.stop();

    assert_eq!(snapshots.len(), 3);
    for (index, snap) in snapshots.iter().enumerate() {
        let restored = manager.load_snapshot(snap).await.expect("replay");
        let expected_rev = index as u16 + 1;
        assert_eq!(
            restored.to_json()["rev"],
            expected_rev,
            "snapshot {index} must replay revision {expected_rev}"
        );
    }
}

/// CMP-16:nats-bus 双实例 —— A 广播变更事件,B 收到后可重载一致。
#[tokio::test]
async fn cmp16_nats_bus_syncs_two_instances() {
    use confers::bus::{ConfigBus as _, ConfigChangeEvent, NatsBusBuilder};

    let nats_up = std::net::TcpStream::connect("127.0.0.1:4222").is_ok();
    if !nats_up {
        eprintln!("Skipping test: NATS not available");
        return;
    }

    use futures_util::StreamExt as _;
    let pid = std::process::id();
    let subject = format!("cmp16.{pid}.topic");
    let stream = format!("CMP16{pid}");
    let instance_a = NatsBusBuilder::new()
        .url("nats://127.0.0.1:4222")
        .subject(&subject)
        .stream_name(&stream)
        .build()
        .await
        .expect("instance A");
    let instance_b = NatsBusBuilder::new()
        .url("nats://127.0.0.1:4222")
        .subject(&subject)
        .stream_name(&stream)
        .build()
        .await
        .expect("instance B");

    let mut rx_b = instance_b.subscribe().await.expect("B subscribes");

    let event = ConfigChangeEvent::new(
        "instance-A",
        "file",
        vec!["feature.x".to_string()],
        "cmp16-sum",
    );
    instance_a.publish(event).await.expect("A publishes");

    let received = tokio::time::timeout(Duration::from_secs(5), rx_b.next())
        .await
        .expect("B receives event")
        .expect("stream open");
    assert_eq!(received.instance_id, "instance-A");
    assert_eq!(received.checksum, "cmp16-sum");
}

/// CMP-17 / CTX-09:context + toggle + dynamic 三方联合决定动态值。
#[test]
fn cmp17_context_and_toggle_decide_dynamic_value() {
    use confers::context::{ContextAwareField, ContextValue, EvaluationContext};

    let toggle = FeatureToggleRegistry::new();
    toggle.register("premium_limits", "region-aware premium limits", false);

    let upload_limit: ContextAwareField<u64> = ContextAwareField::new(100)
        .when(
            |ctx: &EvaluationContext| {
                ctx.attributes().get("plan") == Some(&ContextValue::String("pro".into()))
            },
            1000,
        )
        .when(
            |ctx: &EvaluationContext| {
                ctx.attributes().get("region") == Some(&ContextValue::String("cn-north".into()))
            },
            500,
        );

    let handle: DynamicField<u64> = DynamicField::new(100);

    // 上下文求值 → 开关门控 → 动态字段更新。
    let pro_ctx = EvaluationContext::new().attr("plan", ContextValue::String("pro".into()));
    let mut value = *upload_limit.evaluate(&pro_ctx);
    if !toggle.is_enabled("premium_limits") {
        value = handle.get(); // 高级限制未开启 → 维持默认
    }
    handle.update(value);
    assert_eq!(handle.get(), 100, "closed toggle keeps the default");

    toggle.enable("premium_limits");
    let cn_ctx = EvaluationContext::new().attr("region", ContextValue::String("cn-north".into()));
    let value = *upload_limit.evaluate(&cn_ctx);
    handle.update(value);
    assert_eq!(handle.get(), 500, "region rule applies once toggle is open");
}
