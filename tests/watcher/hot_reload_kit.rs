// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Integration tests for the hot-reload facade.
//!
//! The facade welds together `FsWatcher` (file events), a
//! `ProgressiveReloader` (staged commit of new configuration), a
//! `tokio::sync::watch` broadcast channel (consumer notification), and a
//! `WatcherGuard` (graceful shutdown with timeout). These tests pin the
//! assembled behavior: the manual `begin_reload` entry point, the
//! file-event-driven reload loop, failure accounting on loader errors, and
//! shutdown completing within the timeout.

#![cfg(feature = "hot-reload-kit")]

use std::sync::Arc;
use std::time::Duration;

use serial_test::serial;
use tempfile::TempDir;

use confers::watcher::{
    FsWatcher, HotReloader, ProgressiveReloader, ReloadOutcome, ReloadStrategy,
};
use confers::{AnnotatedValue, ConfigError, ConfigProvider};

#[derive(Clone, Debug, PartialEq)]
struct AppConfig {
    value: u32,
}

/// Minimal provider stub: the facade only hands it through to the
/// reloader (health checks), no keys are read in these scenarios.
struct EmptyProvider;

impl ConfigProvider for EmptyProvider {
    fn get_raw(&self, _key: &str) -> Option<&AnnotatedValue> {
        None
    }

    fn keys(&self) -> Vec<String> {
        Vec::new()
    }
}

async fn spawn_kit(
    initial: AppConfig,
    path: &TempDir,
    disk: Arc<std::sync::atomic::AtomicU64>,
) -> HotReloader<AppConfig> {
    let cfg_path = path.path().join("app.conf");
    std::fs::write(&cfg_path, "value=1").expect("write initial config");

    let watcher = FsWatcher::new(&cfg_path, 50).await.expect("watcher starts");
    // 与既有 watch e2e 的 settle() 惯例一致：等待 inotify watch 建立，
    // 避免 watch 生效前的首写事件被静默丢失。
    tokio::time::sleep(Duration::from_millis(400)).await;
    let reloader = ProgressiveReloader::new(Arc::new(initial), ReloadStrategy::Immediate);
    let loader: confers::watcher::HotReloadLoader<AppConfig> = Arc::new(move || {
        Ok((
            Arc::new(AppConfig {
                value: disk.load(std::sync::atomic::Ordering::SeqCst) as u32,
            }),
            Arc::new(EmptyProvider),
        ))
    });
    HotReloader::spawn(reloader, watcher, loader)
}

/// The manual entry point reloads without any file event and broadcasts
/// the new configuration to subscribers.
#[tokio::test]
#[serial]
async fn test_hot_reloader_manual_begin_reload_broadcasts() {
    let tmp = TempDir::new().expect("tempdir");
    let kit = spawn_kit(
        AppConfig { value: 1 },
        &tmp,
        Arc::new(std::sync::atomic::AtomicU64::new(1)),
    )
    .await;
    let rx = kit.subscribe();

    assert_eq!(
        kit.current().value,
        1,
        "current starts at the initial config"
    );

    let outcome = kit
        .begin_reload(Arc::new(AppConfig { value: 2 }), Arc::new(EmptyProvider))
        .await
        .expect("manual reload commits under Immediate strategy");
    assert!(matches!(outcome, ReloadOutcome::Committed));
    assert_eq!(kit.current().value, 2, "current reflects the manual reload");
    assert_eq!(
        rx.borrow().value,
        2,
        "broadcast channel carries the new config"
    );

    let completed = kit
        .shutdown(Duration::from_secs(2))
        .await
        .expect("shutdown returns Ok");
    assert!(completed, "event loop exits within the shutdown timeout");
}

/// A file change flows through the assembled pipeline: FsWatcher event →
/// loader → begin_reload → broadcast. Subscribers observe the new value.
#[tokio::test]
#[serial]
async fn test_hot_reloader_file_change_triggers_reload_and_broadcast() {
    let tmp = TempDir::new().expect("tempdir");
    let disk = Arc::new(std::sync::atomic::AtomicU64::new(1));
    let kit = spawn_kit(AppConfig { value: 1 }, &tmp, disk.clone()).await;
    let mut rx = kit.subscribe();

    let cfg_path = tmp.path().join("app.conf");
    disk.store(7, std::sync::atomic::Ordering::SeqCst);
    std::fs::write(&cfg_path, "value=7").expect("modify watched file");

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            rx.changed()
                .await
                .expect("broadcast channel stays open while the kit runs");
            if rx.borrow().value == 7 {
                break;
            }
        }
    })
    .await
    .expect("file change must reach subscribers as a reloaded config");

    assert_eq!(kit.current().value, 7);

    let completed = kit
        .shutdown(Duration::from_secs(2))
        .await
        .expect("shutdown returns Ok");
    assert!(completed);
}

/// A loader error must not be swallowed: the failure counter increments,
/// the current configuration stays untouched, and the loop keeps serving
/// later events.
#[tokio::test]
#[serial]
async fn test_hot_reloader_counts_failed_reloads_and_keeps_current() {
    let tmp = TempDir::new().expect("tempdir");
    let cfg_path = tmp.path().join("app.conf");
    std::fs::write(&cfg_path, "value=1").expect("write initial config");

    let watcher = FsWatcher::new(&cfg_path, 50).await.expect("watcher starts");
    // 与既有 watch e2e 的 settle() 惯例一致：等待 inotify watch 建立。
    tokio::time::sleep(Duration::from_millis(400)).await;
    let reloader =
        ProgressiveReloader::new(Arc::new(AppConfig { value: 1 }), ReloadStrategy::Immediate);
    let failing_loader: confers::watcher::HotReloadLoader<AppConfig> = Arc::new(|| {
        Err(ConfigError::FileNotFound {
            filename: std::path::PathBuf::from("gone.conf"),
            source: None,
        })
    });
    let kit = HotReloader::spawn(reloader, watcher, failing_loader);

    std::fs::write(&cfg_path, "value=2").expect("trigger a file event");

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if kit.reload_failures() >= 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("a failed loader call must surface in the failure counter");

    assert_eq!(
        kit.current().value,
        1,
        "current config stays untouched when the loader fails"
    );

    let completed = kit
        .shutdown(Duration::from_secs(2))
        .await
        .expect("shutdown returns Ok");
    assert!(completed);
}

#[tokio::test]
#[serial]
async fn test_hot_reloader_loader_panic_is_counted_and_loop_survives() {
    // A panicking loader must not kill the event loop silently: the failure
    // counter increments (the panic surfaces as a JoinError from the
    // blocking pool), and the loop keeps serving later events.
    let tmp = TempDir::new().expect("tempdir");
    let cfg_path = tmp.path().join("app.conf");
    std::fs::write(&cfg_path, "value=1").expect("write initial config");

    let watcher = FsWatcher::new(&cfg_path, 50).await.expect("watcher starts");
    // 与既有 watch e2e 的 settle() 惯例一致：等待 inotify watch 建立。
    tokio::time::sleep(Duration::from_millis(400)).await;
    let reloader =
        ProgressiveReloader::new(Arc::new(AppConfig { value: 1 }), ReloadStrategy::Immediate);
    let panicking_loader: confers::watcher::HotReloadLoader<AppConfig> =
        Arc::new(|| panic!("loader boom"));
    let kit = HotReloader::spawn(reloader, watcher, panicking_loader);

    std::fs::write(&cfg_path, "value=2").expect("trigger a file event");

    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if kit.reload_failures() >= 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("a panicking loader must surface in the failure counter");
    assert_eq!(kit.current().value, 1, "current stays untouched");

    // The loop survived the panic: a later event still flows through.
    std::fs::write(&cfg_path, "value=3").expect("trigger another file event");
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if kit.reload_failures() >= 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the event loop must keep serving events after a loader panic");

    let completed = kit
        .shutdown(Duration::from_secs(2))
        .await
        .expect("shutdown returns Ok");
    assert!(completed, "loop exits cleanly on shutdown");
}
