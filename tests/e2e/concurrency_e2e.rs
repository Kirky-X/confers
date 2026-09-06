// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 并发与竞态(tests/e2e/concurrency_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.27):
//! - CCY-01 `new_in_memory()` 连接器:多任务并发 set/get/delete/has 无死锁、
//!   最终一致
//! - CCY-02 watch 期间并发追加写(每次写完整内容)→ 每次触发重载均可完整解析
//! - CCY-03 并发 reload + 读:arc-swap 保证读侧只见新旧两版之一
//! - CCY-04 snapshot 并发 save + prune:目录内快照数 ≤ max 且无损坏
//! - CCY-05 bus 4 生产者并发 publish 1000 事件,订阅者合计收全
//! - CCY-07 toggle 高并发 toggle/is_enabled 混合,无 panic、最终状态自洽
//! - CCY-08 大配置(5k 键 + 深度 32 嵌套,默认 limits 内)加载成功;收紧
//!   limits 后超限被拒(默认 max_total_fields=10000,10k 键属超限档)
//!
//! CCY-06(audit 8 线程并发写)已固化于 audit_e2e.rs(AUD-11 同压力档)。
//! 基础档既有覆盖(tests/core/toggle.rs 等)不在此重复。

use confers::bus::{BusBuilder, ConfigBus, ConfigChangeEvent, InMemoryBus};
use confers::interface::{ConfigReader, ConfigWriter};
use confers::snapshot::{SnapshotConfig, SnapshotFormat, SnapshotManager};
use confers::toggle::FeatureToggleRegistry;
use confers::types::{AnnotatedValue, ConfigValue, SourceId};
use confers::watcher::FsWatcher;
use confers::{ConfigLimits, new_in_memory};
use std::sync::Arc;
use std::time::Duration;

fn annotated(key: &str, value: &str) -> AnnotatedValue {
    AnnotatedValue::new(ConfigValue::string(value), SourceId::new("ccy"), key)
}

#[tokio::test]
async fn ccy01_in_memory_connector_concurrent_ops() {
    let config = Arc::new(new_in_memory());
    let mut handles = Vec::new();

    for t in 0..8u32 {
        let cfg = Arc::clone(&config);
        handles.push(tokio::spawn(async move {
            for i in 0..50u32 {
                let key = format!("t{t}.k{i}");
                cfg.set(&key, annotated(&key, &format!("v{t}-{i}")))
                    .await
                    .expect("set must succeed");
                assert!(cfg.has(&key).await.expect("has"));
                let read = cfg.get_string(&key).await.expect("get").unwrap();
                assert_eq!(read, format!("v{t}-{i}"));
            }
            for i in 0..25u32 {
                let key = format!("t{t}.k{i}");
                assert!(
                    cfg.delete(&key).await.expect("delete"),
                    "existing key deletes true"
                );
                assert!(!cfg.has(&key).await.expect("has after delete"));
            }
        }));
    }
    for handle in handles {
        handle.await.expect("worker must not panic");
    }

    // 最终一致:每任务剩 25 键,共 8×25。
    let keys = config.keys().await.expect("keys");
    assert_eq!(
        keys.len(),
        8 * 25,
        "final state must be exactly the survivors"
    );
}

#[tokio::test]
async fn ccy02_concurrent_full_writes_parse_clean_on_every_reload() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hot.toml");
    std::fs::write(&path, "generation = 0\npayload = \"init\"\n").unwrap();

    let mut watcher = FsWatcher::new(&path, 30).await.expect("watcher");

    // 写方:每次写完整内容,连续 10 代。
    let writer_path = path.clone();
    let writer = tokio::spawn(async move {
        for gv in 1..=10u32 {
            std::fs::write(
                &writer_path,
                format!("generation = {gv}\npayload = \"gen-{gv}\"\n"),
            )
            .expect("full-content write");
            tokio::time::sleep(Duration::from_millis(40)).await;
        }
    });

    // 读方:每次事件后重新加载,必须完整解析(无半读)。
    let mut events = 0usize;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while events < 3 && tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(800), watcher.recv()).await {
            Ok(Some(_)) => {
                events += 1;
                // 触发即重载:完整解析(允许读到任一完整代)。
                let value =
                    confers::load_file(&path, &confers::LoaderConfig::new().allow_absolute())
                        .expect("reload after event must parse fully");
                let fields = match &value.inner {
                    ConfigValue::Map(entries) => entries,
                    _ => panic!("map expected"),
                };
                let gv = fields
                    .get("generation")
                    .and_then(|v| v.inner.as_u64())
                    .expect("generation present");
                let payload = fields
                    .get("payload")
                    .and_then(|v| v.inner.as_str().map(str::to_string))
                    .expect("payload present");
                assert_eq!(
                    payload,
                    format!("gen-{gv}"),
                    "pair must be from one full write"
                );
            }
            Ok(None) => break,
            Err(_) => continue,
        }
    }
    writer.await.expect("writer must finish");
    assert!(
        events >= 3,
        "expected at least 3 reload events, got {events}"
    );

    // 最终内容完整可解析。
    let final_value = confers::load_file(&path, &confers::LoaderConfig::new().allow_absolute())
        .expect("final content parses");
    assert!(
        final_value.to_json().to_string().contains("gen-"),
        "final content carries a full generation"
    );
}

#[tokio::test]
async fn ccy03_concurrent_reload_reads_see_one_full_version() {
    let reloader = Arc::new(confers::watcher::ProgressiveReloader::new(
        Arc::new(Versioned { tag: 0, value: 0 }),
        confers::watcher::ReloadStrategy::Immediate,
    ));

    // 4 写任务 × 25 次 reload(不同版本值);同时 4 读任务并发校验完整性。
    let mut writers = Vec::new();
    for t in 1..=4u64 {
        let r = Arc::clone(&reloader);
        writers.push(tokio::spawn(async move {
            for i in 1..=25u64 {
                let new = Versioned {
                    tag: t,
                    value: t * 1000 + i,
                };
                let provider: Arc<dyn confers::interface::ConfigProvider> = Arc::new(EmptyProvider);
                r.begin_reload(Arc::new(new), provider)
                    .await
                    .expect("immediate reload commits");
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }));
    }
    let mut readers = Vec::new();
    for _ in 0..4 {
        let r = Arc::clone(&reloader);
        readers.push(tokio::spawn(async move {
            for _ in 0..200 {
                let cur = r.current();
                // 原子快照:tag 与 value 必须来自同一版本,无撕裂。
                assert!(
                    (1..=4).contains(&cur.tag),
                    "tag must be a committed writer id, got {}",
                    cur.tag
                );
                assert_ne!(cur.value % 1000, 0, "value suffix never zero mid-write");
                tokio::time::sleep(Duration::from_micros(50)).await;
            }
        }));
    }
    for h in writers {
        h.await.expect("writer ok");
    }
    for h in readers {
        h.await.expect("reader ok");
    }
    // 最终态:任一写者的最后一次提交(完整)。
    let final_cfg = reloader.current();
    assert_eq!(final_cfg.value % 1000, 25, "last commit of some writer");
}

#[derive(Debug, Clone)]
struct Versioned {
    tag: u64,
    value: u64,
}

/// 空 provider(Immediate 策略不读取;进程内真实数据,非 test double)。
#[derive(Debug)]
struct EmptyProvider;

impl confers::interface::ConfigProvider for EmptyProvider {
    fn get_raw(&self, _key: &str) -> Option<&AnnotatedValue> {
        None
    }
    fn keys(&self) -> Vec<String> {
        Vec::new()
    }
}

#[tokio::test]
async fn ccy04_concurrent_snapshot_save_and_prune() {
    let dir = tempfile::tempdir().unwrap();
    let manager = Arc::new(SnapshotManager::new(SnapshotConfig {
        dir: dir.path().join("snaps"),
        max_snapshots: 5,
        format: SnapshotFormat::Json,
        include_provenance: false,
    }));

    let value = AnnotatedValue::new(
        ConfigValue::map(vec![("ccy".to_string(), annotated("ccy", "4"))]),
        SourceId::new("ccy04"),
        "",
    );

    // 8 任务并发 save,各 3 份 → 目录最多保留 max(5) 份且全部完好。
    let mut handles = Vec::new();
    for _t in 0..8u32 {
        let m = Arc::clone(&manager);
        let v = value.clone();
        handles.push(tokio::spawn(async move {
            for _i in 0..3u32 {
                m.save(&v, &[]).await.expect("snapshot save");
            }
        }));
    }
    for h in handles {
        h.await.expect("saver must not panic");
    }

    manager.prune_old_snapshots().expect("prune");
    let listed = manager.list_snapshots().expect("list");
    assert!(
        listed.len() <= 5,
        "snapshot count must be capped at max_snapshots, got {}",
        listed.len()
    );
    // 存留快照全部可完整回放(无损坏文件)。
    for info in &listed {
        manager
            .load_snapshot(&info.path)
            .await
            .expect("every retained snapshot must load");
    }
}

#[tokio::test]
async fn ccy05_bus_four_publishers_thousand_events_delivered() {
    let bus: InMemoryBus = BusBuilder::new().capacity(4096).build();
    let rx1 = bus.subscribe().await.expect("subscriber 1");
    let rx2 = bus.subscribe().await.expect("subscriber 2");
    let bus = Arc::new(bus);

    const PRODUCERS: usize = 4;
    const PER_PRODUCER: usize = 250;
    const TOTAL: usize = PRODUCERS * PER_PRODUCER;

    let mut producers = Vec::new();
    for p in 0..PRODUCERS {
        let b = Arc::clone(&bus);
        producers.push(tokio::spawn(async move {
            for i in 0..PER_PRODUCER {
                let event =
                    ConfigChangeEvent::new("ccy05", "e2e", vec![format!("p{p}.k{i}")], "checksum");
                b.publish(event).await.expect("publish must succeed");
            }
        }));
    }
    for h in producers {
        h.await.expect("producer must not panic");
    }

    // 广播语义:两个订阅者各自收全(容量足够时)。
    let (s1, s2) = tokio::join!(count_events(rx1, TOTAL), count_events(rx2, TOTAL));
    assert_eq!(s1, TOTAL, "subscriber 1 must receive all events");
    assert_eq!(s2, TOTAL, "subscriber 2 must receive all events");
}

/// 统计订阅者在时限内收到的事件数(广播流)。
async fn count_events(
    mut rx: std::pin::Pin<Box<dyn futures_util::Stream<Item = ConfigChangeEvent> + Send>>,
    expected: usize,
) -> usize {
    use futures_util::StreamExt;
    let mut seen = 0usize;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while seen < expected && tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_millis(500), rx.next()).await {
            Ok(Some(_)) => seen += 1,
            _ => break,
        }
    }
    seen
}

#[test]
fn ccy07_toggle_concurrent_mixed_ops_self_consistent() {
    let registry = Arc::new(FeatureToggleRegistry::new());
    for f in 0..8u32 {
        registry.register(format!("feature_{f}"), format!("Feature {f}"), false);
    }

    let mut handles = Vec::new();
    for t in 0..8u32 {
        let reg = Arc::clone(&registry);
        handles.push(std::thread::spawn(move || {
            for i in 0..200u32 {
                let name = format!("feature_{}", (t + i) % 8);
                if i % 3 == 0 {
                    reg.enable(&name);
                } else if i % 3 == 1 {
                    reg.disable(&name);
                } else {
                    reg.toggle(&name);
                }
                let _ = reg.is_enabled(&name);
            }
        }));
    }
    for h in handles {
        h.join().expect("no panic under mixed concurrency");
    }

    // 最终自洽:list 长度 == 注册数;is_enabled 与 list 状态一一对应。
    let infos = registry.list();
    assert_eq!(infos.len(), 8, "no entries lost or fabricated");
    for info in &infos {
        assert_eq!(
            registry.is_enabled(&info.name),
            info.enabled,
            "list state and is_enabled must agree for {}",
            info.name
        );
    }
}

#[test]
fn ccy08_large_config_within_limits_and_overlimit_rejected() {
    let dir = tempfile::tempdir().unwrap();

    // 大配置:10k 顶层键 + 32 层嵌套表(toml)。
    let mut content = String::with_capacity(256 * 1024);
    for i in 0..5_000u32 {
        content.push_str(&format!("key_{i} = {i}\n"));
    }
    let mut header = String::new();
    for d in 0..32 {
        header.push_str(&format!("[deep.d{d}]\n"));
    }
    header.push_str("leaf = \"bottom\"\n");
    let path = dir.path().join("big.toml");
    std::fs::write(&path, format!("{header}{content}")).unwrap();

    let loaded = confers::load_file(&path, &confers::LoaderConfig::new().allow_absolute())
        .expect("large config within default limits must load");

    // 默认 limits 内通过结构校验。
    let limits = ConfigLimits::default();
    limits
        .validate_value(&loaded)
        .expect("5k keys + depth 32 within default limits");

    // 超限拒绝:max_string_length 收紧后,长字符串被拒。
    let tight = ConfigLimits::default().with_max_string_length(8);
    let long_string = AnnotatedValue::new(
        ConfigValue::string("x".repeat(128)),
        SourceId::new("ccy08"),
        "oversized",
    );
    assert!(
        tight.validate_value(&long_string).is_err(),
        "string over max_string_length must be rejected"
    );

    // 超限拒绝:max_total_fields 收紧后,10k 键被拒。
    let few_fields = ConfigLimits::default().with_max_total_fields(100);
    assert!(
        few_fields.validate_value(&loaded).is_err(),
        "5k keys over tightened max_total_fields must be rejected"
    );
}
