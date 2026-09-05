// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 动态字段(tests/e2e/dynamic_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.11):
//! - DYN-14 `#[config(dynamic = true)]` 字段:派生宏当前接受该属性但**不生成**
//!   DynamicField handle(解析期保留,见报告);本场景按真实行为固化:
//!   带该属性的配置可正常编译加载,并以手动 DynamicField 挂接真实配置重载闭环。
//!
//! DYN-01…13 已有覆盖(tests/core/dynamic.rs)。

use confers::ConfigBuilder;
use confers::dynamic::DynamicField;
use serde::Deserialize;
use std::sync::{Arc, Mutex};

#[derive(Debug, confers::Config, Deserialize)]
struct FeatureFlags {
    #[config(dynamic = true)]
    pub max_connections: u32,
}

#[test]
fn dyn14_dynamic_attr_config_loads_and_manual_handle_tracks_reload() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("features.toml");
    std::fs::write(&path, "max_connections = 100\n").unwrap();

    // 属性被接受:配置照常加载(无自动 handle —— 行为固化)。
    let initial: FeatureFlags = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&path)
        .build()
        .expect("config with dynamic attr must load");

    assert_eq!(initial.max_connections, 100);

    // 应用侧等价做法:手动创建 handle 并接入真实重载。
    let handle: DynamicField<u32> = DynamicField::new(initial.max_connections);
    let observed = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&observed);
    let _guard = handle.on_change(move |v| sink.lock().unwrap().push(*v));

    std::fs::write(&path, "max_connections = 250\n").unwrap();
    let reloaded: FeatureFlags = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&path)
        .build()
        .expect("reload");
    handle.update(reloaded.max_connections);

    assert_eq!(handle.get(), 250);
    assert_eq!(*observed.lock().unwrap(), vec![250]);
}
