// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! E2E: 文件热更新(tests/e2e/watch_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.9):
//! - WAT-16 端到端热更闭环:改 TOML → FsWatcher 事件 → 重载 → `DynamicField` 读到新值并触发回调
//! - WAT-17 编辑器原子替换(临时文件 + rename 覆盖)触发且仅触发一次事件
//! - WAT-18 快速连续写 10 次(10ms 间隔)→ debounce 合并,事件数远小于写入数
//! - WAT-11 应用层最小重载间隔模式:依据 `WatcherConfig::min_reload_interval_ms`
//!   节流事件流,窗口内第二次变更不触发重载
//! - WAT-12 失败暂停(库内行为):重载循环以 `ReloadFailurePolicy` 执行
//!   「连续失败达 `max_consecutive_failures` → 按 `failure_pause_ms` 暂停 →
//!   到期恢复」
//! - WAT-13 应用层回滚模式:重载解析失败时保留上一份好配置(`rollback_on_validation_failure` 语义)
//!
//! WAT-11/13 说明:FsWatcher 仅提供事件流(debounce 在库内);
//! min_reload_interval / rollback_on_validation_failure 为 `WatcherConfig`
//! 上的策略参数,由使用方在事件循环中执行(官方示例 examples/hot_reload.rs
//! 即此模式)。WAT-12 的失败暂停已内建为 `ReloadFailurePolicy`(失败计数、
//! 暂停窗口与到期恢复是确定性逻辑,由库统一执行)。
//!
//! WAT-01…10/14/15 已有覆盖(tests/watcher/watcher.rs)。

use confers::ConfigBuilder;
use confers::dynamic::DynamicField;
use confers::watcher::{FsWatcher, ReloadFailurePolicy, WatcherConfig};
use serde::Deserialize;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug, Deserialize, Clone, Default)]
struct ConfersConfig {
    port: u16,
}

fn write_config(path: &Path, port: u16) {
    std::fs::write(path, format!("port = {port}\n")).expect("write config");
}

fn load_config(path: &Path) -> ConfersConfig {
    ConfigBuilder::<ConfersConfig>::new()
        .allow_absolute_paths()
        .file(path)
        .build()
        .expect("reload must parse")
}

async fn settle() {
    // 等待 inotify watch 建立,避免首事件竞态(见 WAT-17 探针校准)。
    tokio::time::sleep(Duration::from_millis(400)).await;
}

/// 消费事件流直到 deadline,返回事件数(应用层使用 FsWatcher 的标准方式)。
async fn count_events_until(watcher: &mut FsWatcher, deadline: tokio::time::Instant) -> usize {
    let mut count = 0usize;
    while let Ok(Some(_)) = tokio::time::timeout_at(deadline, watcher.recv()).await {
        count += 1;
    }
    count
}

#[tokio::test]
async fn wat16_end_to_end_hot_reload_updates_dynamic_field() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    write_config(&path, 8080);

    // 初始构建 + 动态字段。
    let config = load_config(&path);
    let port: DynamicField<u16> = DynamicField::new(config.port);
    let observed: Arc<Mutex<Vec<u16>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&observed);
    let _guard = port.on_change(move |p| sink.lock().unwrap().push(*p));
    assert_eq!(port.get(), 8080);

    let mut watcher = FsWatcher::new(&path, 100).await.expect("watcher starts");
    settle().await;

    // 修改文件 → 事件 → 重载 → 更新动态字段(官方热更循环的应用方式)。
    write_config(&path, 9090);
    let reload_loop = async {
        while let Some(changed) = watcher.recv().await {
            if changed == path {
                let new_cfg = load_config(&path);
                port.update(new_cfg.port);
                break;
            }
        }
    };
    tokio::time::timeout(Duration::from_secs(10), reload_loop)
        .await
        .expect("hot reload must complete within 10s");

    assert_eq!(port.get(), 9090, "dynamic field must observe the new value");
    assert_eq!(
        *observed.lock().unwrap(),
        vec![9090],
        "callback must fire once"
    );
    watcher.stop();
}

#[tokio::test]
async fn wat17_atomic_replace_triggers_exactly_one_event() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    write_config(&path, 1);

    let mut watcher = FsWatcher::new(&path, 200).await.expect("watcher starts");
    settle().await;

    // 编辑器式原子替换:写临时文件 + rename 覆盖。
    let tmp = dir.path().join("app.toml.tmp");
    std::fs::write(&tmp, "port = 2\n").unwrap();
    std::fs::rename(&tmp, &path).unwrap();

    let count = count_events_until(
        &mut watcher,
        tokio::time::Instant::now() + Duration::from_millis(1500),
    )
    .await;
    assert_eq!(count, 1, "atomic replace must coalesce to a single event");
    watcher.stop();
}

/// WAT-18(去 ignore):库内 FsWatcher 事件出口已接入 AdaptiveDebouncer,
/// 快速连写被硬性限流合并,不再依赖 CI 时序。
#[tokio::test]
async fn wat18_rapid_writes_collapse_via_debounce() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    write_config(&path, 0);

    let mut watcher = FsWatcher::new(&path, 200).await.expect("watcher starts");
    settle().await;

    for i in 1..=10u16 {
        write_config(&path, i);
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    let count = count_events_until(
        &mut watcher,
        tokio::time::Instant::now() + Duration::from_millis(2000),
    )
    .await;
    assert!(
        (1..=5).contains(&count),
        "10 rapid writes must collapse to a handful of events (observed ≈3), got {count}"
    );
    watcher.stop();
}

/// 事件流 + `min_reload_interval_ms` 节流的应用模式。
#[tokio::test]
async fn wat11_min_reload_interval_throttles_reload_execution() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    write_config(&path, 1);

    let watcher_config = WatcherConfig::builder()
        .debounce_ms(50)
        .min_reload_interval_ms(400)
        .build();
    assert_eq!(watcher_config.min_reload_interval_ms, 400);

    let mut watcher = FsWatcher::new(&path, watcher_config.debounce_ms)
        .await
        .expect("watcher starts");
    settle().await;

    let reloads = Arc::new(Mutex::new(Vec::<u16>::new()));
    let mut last_reload = tokio::time::Instant::now() - Duration::from_secs(1);

    // 两次变更:第一次立即重载;第二次落在最小间隔窗口内 → 跳过。
    for port in [2u16, 3] {
        write_config(&path, port);
        let event = tokio::time::timeout(Duration::from_secs(5), watcher.recv())
            .await
            .expect("event must arrive")
            .expect("channel open");
        assert_eq!(event, path);
        let now = tokio::time::Instant::now();
        if now.duration_since(last_reload)
            < Duration::from_millis(watcher_config.min_reload_interval_ms)
        {
            continue; // 节流:窗口内不重载(与示例 hot_reload 相同的模式)
        }
        last_reload = now;
        reloads.lock().unwrap().push(load_config(&path).port);
    }

    let reloads = reloads.lock().unwrap();
    assert_eq!(
        reloads.len(),
        1,
        "second in-window change must be throttled"
    );
    assert_eq!(reloads[0], 2);
    watcher.stop();
}

/// WAT-12(行为级):重载循环以库内 `ReloadFailurePolicy` 执行
/// 「连续失败达 `max_consecutive_failures` → 按 `failure_pause_ms` 暂停」。
#[tokio::test]
async fn wat12_consecutive_failures_trigger_failure_pause() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    write_config(&path, 1);

    let watcher_config = WatcherConfig::builder()
        .debounce_ms(50)
        .max_consecutive_failures(2)
        .failure_pause_ms(300)
        .build();

    let mut watcher = FsWatcher::new(&path, watcher_config.debounce_ms)
        .await
        .expect("watcher starts");
    settle().await;

    let mut policy = ReloadFailurePolicy::new(watcher_config.clone());
    let mut pause_opened = false;

    // 两次写入损坏内容 → 重载各失败一次 → 第二次失败达上限,策略开启暂停。
    for content in ["invalid toml {{{", "still bad {{{"] {
        std::fs::write(&path, content).unwrap();
        tokio::time::timeout(Duration::from_secs(5), watcher.recv())
            .await
            .expect("event must arrive")
            .expect("channel open");
        let result = ConfigBuilder::<ConfersConfig>::new()
            .allow_absolute_paths()
            .file(&path)
            .build();
        assert!(result.is_err(), "corrupt content must fail to reload");
        if policy.record_failure() {
            pause_opened = true;
        }
    }
    assert!(
        pause_opened,
        "reaching the failure cap must open the pause window"
    );
    assert_eq!(
        policy.consecutive_failures(),
        0,
        "counter resets as the pause opens"
    );

    // 暂停开启后窗口内剩余时间 ∈ (0, failure_pause_ms]。
    let remaining = policy
        .pause_remaining()
        .expect("pause window must be open right after the cap");
    assert!(remaining > Duration::ZERO && remaining <= Duration::from_millis(300));

    // 暂停期内事件照常送达,但重载被策略抑制(仅观察剩余时间递减)。
    write_config(&path, 6);
    let event = tokio::time::timeout(Duration::from_secs(5), watcher.recv())
        .await
        .expect("event must arrive during the pause")
        .expect("channel open");
    assert_eq!(event, path.clone());
    let later = policy
        .pause_remaining()
        .expect("reload must still be suppressed within the window");
    assert!(
        later < remaining,
        "pause must count down: {later:?} vs {remaining:?}"
    );

    // 暂停结束后窗口自动关闭,恢复正常重载;成功重置失败计数。
    tokio::time::sleep(later + Duration::from_millis(20)).await;
    assert!(
        policy.pause_remaining().is_none(),
        "pause must expire after failure_pause_ms"
    );
    write_config(&path, 7);
    let event = tokio::time::timeout(Duration::from_secs(5), watcher.recv())
        .await
        .expect("watcher must deliver after pause expiry")
        .expect("channel open");
    assert_eq!(event, path.clone());
    assert_eq!(load_config(&path).port, 7);
    policy.record_success();
    assert_eq!(policy.consecutive_failures(), 0);
    watcher.stop();
}

/// 重载失败回滚到上一份好配置的应用模式。
#[tokio::test]
async fn wat13_rollback_keeps_last_good_config_on_failed_reload() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("app.toml");
    write_config(&path, 100);

    let watcher_config = WatcherConfig::builder()
        .debounce_ms(50)
        .rollback_on_validation_failure(true)
        .build();
    assert!(watcher_config.rollback_on_validation_failure);

    let current: Arc<Mutex<Option<ConfersConfig>>> = Arc::new(Mutex::new(Some(load_config(&path))));
    let mut watcher = FsWatcher::new(&path, watcher_config.debounce_ms)
        .await
        .expect("watcher starts");
    settle().await;

    // 1) 写坏文件:重载失败 → 回滚(保留旧值)。
    std::fs::write(&path, "port = \"not-a-number\"\n").unwrap();
    tokio::time::timeout(Duration::from_secs(5), watcher.recv())
        .await
        .expect("event must arrive")
        .expect("channel open");
    match ConfigBuilder::<ConfersConfig>::new()
        .allow_absolute_paths()
        .file(&path)
        .build()
    {
        Ok(new_cfg) => *current.lock().unwrap() = Some(new_cfg),
        Err(_) => { /* rollback: keep current */ }
    }
    assert_eq!(
        current.lock().unwrap().as_ref().unwrap().port,
        100,
        "failed reload must not clobber the good config"
    );

    // 2) 写好文件:重载成功 → 新值生效。
    write_config(&path, 200);
    tokio::time::timeout(Duration::from_secs(5), watcher.recv())
        .await
        .expect("event must arrive")
        .expect("channel open");
    let new_cfg = load_config(&path);
    *current.lock().unwrap() = Some(new_cfg);
    assert_eq!(current.lock().unwrap().as_ref().unwrap().port, 200);
    watcher.stop();
}
