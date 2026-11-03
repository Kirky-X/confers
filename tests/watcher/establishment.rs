// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Watch establishment contract tests (R-watch-008).
//!
//! `FsWatcher::new()` 返回即代表监听已生效：调用方在 `new()` 之后立刻写入
//! 文件，事件必须可达。修复前 watch 的建立发生在 watcher 线程里，`new()`
//! 返回时并不保证已建立——建立窗口内的写入事件会永久丢失（macOS 后端为
//! FSEvents，`FSEventStreamStart` 之前的变化不会被补报），fluxgate 热更新
//! 测试的并发失败即源于此。

#![cfg(feature = "watch")]

use std::sync::Arc;
use std::time::Duration;

use confers::watcher::FsWatcher;

/// 等待单个事件的有界循环：事件缺失让测试失败而不是挂死套件。
async fn expect_event(watcher: &mut FsWatcher, label: &str, patience: Duration) {
    let deadline = std::time::Instant::now() + patience;
    loop {
        assert!(
            std::time::Instant::now() < deadline,
            "{label}: event must arrive within {patience:?}"
        );
        match tokio::time::timeout(Duration::from_millis(500), watcher.recv()).await {
            Ok(Some(_)) => return,
            Ok(None) => panic!(
                "{label}: watcher terminated before delivering the event (is_running={})",
                watcher.is_running()
            ),
            Err(_elapsed) => continue,
        }
    }
}

/// R-watch-008：`new()` 返回后的第一次写入（无任何沉降时间）必须产生事件。
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn immediate_write_after_creation_is_delivered() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("cfg.toml");
    std::fs::write(&path, "a = 1\n").expect("seed");

    let mut watcher = FsWatcher::new(&path, 50).await.expect("watch established");

    std::fs::write(&path, "a = 2\n").expect("rewrite");

    expect_event(
        &mut watcher,
        "immediate first write",
        Duration::from_secs(10),
    )
    .await;

    // 事件之后监听必须继续存活（丢失的是"建立窗口首写"，不是整个监听）。
    std::fs::write(&path, "a = 3\n").expect("second rewrite");
    expect_event(&mut watcher, "follow-up write", Duration::from_secs(10)).await;

    watcher.stop();
}

/// R-watch-008：并发建立 + 同时首写。9 个 watcher（对齐 fluxgate 热更新
/// 测试规模）在栅栏后同时写入，所有事件必须可达。
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_watchers_immediate_first_writes_all_delivered() {
    const N: usize = 9;
    let barrier = Arc::new(tokio::sync::Barrier::new(N));
    let mut handles = Vec::with_capacity(N);
    for i in 0..N {
        let barrier = Arc::clone(&barrier);
        handles.push(tokio::spawn(async move {
            let dir = tempfile::tempdir().expect("tempdir");
            let path = dir.path().join("cfg.toml");
            std::fs::write(&path, "a = 1\n").expect("seed");

            let mut watcher = FsWatcher::new(&path, 50).await.expect("watch established");
            barrier.wait().await;
            std::fs::write(&path, format!("a = {}\n", i + 2)).expect("rewrite");

            expect_event(
                &mut watcher,
                &format!("watcher {i} immediate first write"),
                Duration::from_secs(15),
            )
            .await;
            watcher.stop();
        }));
    }
    for handle in handles {
        handle.await.expect("watcher task must not panic");
    }
}

/// R-watch-008：构造期语义——无法监听的路径必须以 `Err` 显性失败，不得
/// 返回一个"看似存活实则失聪"的 watcher。本用例覆盖的是构造函数的
/// 路径不存在预检分支（FileNotFound）；`establish_watch` → `Err` 的
/// 传播路径由 src/watcher/fs_watcher.rs 内的单元测试
/// `establish_watch_fails_for_unwatchable_path` /
/// `establish_watch_multi_fails_when_no_path_is_watchable` /
/// `establish_watch_fails_for_relative_nonexistent_path` 直接覆盖
/// （"存在但不可 watch"的路径无法跨平台确定性构造）。
#[tokio::test]
async fn unwatchable_path_fails_construction_explicitly() {
    let result = FsWatcher::new("/nonexistent/confers-watch-target.toml", 50).await;
    assert!(result.is_err(), "unwatchable path must fail construction");
}
