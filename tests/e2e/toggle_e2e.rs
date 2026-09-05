// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 特性开关(tests/e2e/toggle_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.12):
//! - TGL-02 对未注册 toggle 执行 enable/disable/toggle/is_enabled → 返回 false,
//!   不 panic,注册表状态保持自洽
//!
//! TGL-01/03/04/05 已有覆盖(tests/core/toggle.rs);
//! TGL-06(toggle × dynamic 组合)固化于 combo_e2e.rs(CMP-03)。

use confers::toggle::FeatureToggleRegistry;

#[test]
fn tgl02_operations_on_unregistered_toggle_are_safe_and_false() {
    let registry = FeatureToggleRegistry::new();

    assert!(
        !registry.is_enabled("never_registered"),
        "unregistered toggle must read as disabled"
    );
    assert!(
        !registry.enable("never_registered"),
        "enable on unregistered toggle must report false"
    );
    assert!(
        !registry.disable("never_registered"),
        "disable on unregistered toggle must report false"
    );
    assert!(
        !registry.toggle("never_registered"),
        "toggle on unregistered toggle must report false"
    );
    assert!(!registry.is_enabled("never_registered"));
    assert!(
        registry.list().is_empty(),
        "failed operations must not create entries"
    );
    assert!(registry.is_empty());
    assert_eq!(registry.len(), 0);
}

#[test]
fn tgl02_unregistered_ops_do_not_disturb_registered_state() {
    let registry = FeatureToggleRegistry::new();
    registry.register("real_feature", "Real", true);

    // 针对未注册名的一系列操作不得影响已注册开关。
    assert!(!registry.enable("ghost"));
    assert!(!registry.disable("ghost"));
    assert!(!registry.toggle("ghost"));

    assert!(
        registry.is_enabled("real_feature"),
        "registered state intact"
    );
    assert_eq!(registry.len(), 1);
    let infos = registry.list();
    assert_eq!(infos.len(), 1);
    assert_eq!(infos[0].name, "real_feature");
    assert!(infos[0].enabled);
}
