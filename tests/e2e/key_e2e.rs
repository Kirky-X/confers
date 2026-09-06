// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 密钥管理(tests/e2e/key_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.6):
//! - KEY-01/02 `KeyBundle::generate` 随机 32 字节密钥;正确 master 解出明文,错误 master 报错
//! - KEY-03/04/05 `KeyRing` 生命周期:rotate 切换默认版本、查无此版本 → None、
//!   deactivate 后标记 Deprecated、多版本共存且旧版本仍可解密
//! - KEY-06 `KeyRotationSchedule` 到期/未到期两分支与 update_after_rotation
//! - KEY-07/08/09 `KeyManager` initialize/generate_key/rotate_key 全流程,
//!   get_rotation_status 反映状态;get_key_info(不存在) → 错误
//! - KEY-10/11/12/13 plan/cleanup/deprecate/default 的错误路径与时间边界
//!
//! 既有覆盖为 src 内联 key tests(孤立函数级);本文件固化跨 API 端到端业务流。

use confers::key::{KeyBundle, KeyManager, KeyMetadata, KeyRing, KeyRotationSchedule, KeyStatus};

fn master() -> [u8; 32] {
    let mut k = [0u8; 32];
    k[..16].copy_from_slice(b"e2e-master-key-1");
    k[16..].copy_from_slice(b"e2e-master-key-2");
    k
}

#[test]
fn key0102_generate_roundtrip_and_wrong_master_rejected() {
    let mk = master();

    let bundle = KeyBundle::generate(&mk, 1, "e2e".to_string(), Some("E2E 密钥".to_string()))
        .expect("generate must succeed");
    assert_eq!(bundle.metadata.version, 1);
    assert_eq!(bundle.key_id, "v_1");
    assert!(bundle.metadata.is_active(), "fresh key must be active");

    // 正确 master 解出 32 字节明文密钥(KEY-02 正常分支)。
    let plaintext = bundle
        .get_plaintext_key(&mk)
        .expect("correct master must decrypt");
    assert_ne!(plaintext, [0u8; 32], "generated key must be random");

    // 两次 generate 产生不同密钥(KEY-01 随机性)。
    let other = KeyBundle::generate(&mk, 2, "e2e".to_string(), None).expect("generate #2");
    assert_ne!(
        plaintext,
        other.get_plaintext_key(&mk).expect("decrypt #2"),
        "two generated keys must differ"
    );

    // 错误 master → ConfigError(KEY-02 异常分支)。
    let wrong = [7u8; 32];
    assert!(
        bundle.get_plaintext_key(&wrong).is_err(),
        "wrong master key must not decrypt"
    );
}

#[test]
fn key030405_ring_rotation_multiversion_and_deactivation() {
    let mk = master();
    let mut ring = KeyRing::new(&mk, "ring-e2e".to_string(), "e2e".to_string())
        .expect("ring init must succeed");
    assert_eq!(ring.current_version, 1);

    let v1 = ring
        .get_key_by_version(1)
        .expect("initial version must exist")
        .clone();

    // rotate 产生新版本并切换默认(KEY-03)。
    let rotated = ring
        .rotate(&mk, "e2e".to_string(), Some("轮换".to_string()))
        .expect("rotate must succeed");
    assert_eq!(rotated.metadata.version, 2);
    assert_eq!(ring.current_version, 2);
    assert!(ring.last_rotated_at.is_some());

    // 查无此版本 → None(KEY-04)。
    assert!(ring.get_key_by_version(99).is_none());

    // 多版本共存,旧版本仍可解密(KEY-05)。
    ring.add_secondary_key(v1.clone());
    let recovered_old = ring
        .get_key_by_version(1)
        .expect("old version still resolvable after secondary re-add")
        .get_plaintext_key(&mk)
        .expect("old version key must still decrypt with same master");
    let recovered_new = ring
        .get_key_by_version(2)
        .expect("primary exists")
        .get_plaintext_key(&mk)
        .expect("primary decrypts");
    assert_ne!(recovered_old, recovered_new);

    // deactivate 后查得到但状态为 Deprecated(KEY-04 后半)。
    ring.deactivate_version(1);
    let deactivated = ring.get_key_by_version(1).expect("still present");
    assert_eq!(deactivated.metadata.status, KeyStatus::Deprecated);
    assert!(
        !deactivated.metadata.is_active(),
        "deactivated key must not be active"
    );
}

#[test]
fn key06_rotation_schedule_due_branches() {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // 未到期:90 天间隔,刚轮换(KEY-06 正常分支)。
    let fresh = KeyRotationSchedule::new("svc".to_string(), 90, now, 5);
    assert!(!fresh.is_rotation_due());
    let days = fresh.days_until_rotation();
    assert!(
        (0..=90).contains(&days),
        "days until rotation in [0,90], got {days}"
    );

    // 已到期:last_rotation 在 91 天前(KEY-06 到期分支)。
    let overdue = KeyRotationSchedule::new("svc".to_string(), 90, now - 91 * 86_400, 5);
    assert!(overdue.is_rotation_due());
    assert!(overdue.days_until_rotation() <= 0);

    // 轮换后重置窗口。
    let mut due = overdue;
    due.update_after_rotation();
    assert!(!due.is_rotation_due(), "update_after_rotation must reset");
}

#[test]
fn key070809_manager_initialize_rotate_and_status() {
    let mk = master();
    let mut km = KeyManager::new().expect("manager init");

    // 随机生成(KEY-07)。
    let k1 = km.generate_key().expect("generate_key");
    let k2 = km.generate_key().expect("generate_key #2");
    assert_ne!(k1, k2, "generated keys must differ");

    // initialize 建环并设默认(KEY-07)。
    let version = km
        .initialize(&mk, "production".to_string(), "security-team".to_string())
        .expect("initialize");
    assert_eq!(version.version, 1);
    assert!(version.algorithm.contains("XChaCha20"));
    assert_eq!(km.get_default_key_id(), "production");

    // rotate_key 全流程,get_rotation_status 反映(KEY-08)。
    let result = km
        .rotate_key(
            &mk,
            None,
            "security-team".to_string(),
            Some("定时轮换".to_string()),
        )
        .expect("rotate_key");
    assert_eq!(result.previous_version, 1);
    assert_eq!(result.new_version, 2);
    assert!(result.reencryption_required);

    let statuses = km.get_rotation_status();
    assert_eq!(statuses.len(), 1);
    let status = &statuses[0];
    assert_eq!(status.key_id, "production");
    assert_eq!(status.current_version, 2);
    assert!(!status.is_overdue, "fresh rotation must not be overdue");

    // get_key_info(不存在 key_id) → 错误(KEY-09)。
    assert!(km.get_key_info("ghost").is_err());
}

#[test]
fn key10111213_manager_error_paths_cleanup_and_expiry() {
    let mk = master();
    let mut km = KeyManager::new().expect("manager init");
    km.initialize(&mk, "app".to_string(), "e2e".to_string())
        .expect("initialize");
    km.rotate_key(&mk, None, "e2e".to_string(), None)
        .expect("rotate to v2");
    km.rotate_key(&mk, None, "e2e".to_string(), None)
        .expect("rotate to v3");

    // plan_rotation:target <= current → 错误(KEY-10)。
    assert!(km.plan_rotation(1, None).is_err());
    let plan = km.plan_rotation(5, None).expect("valid plan");
    assert_eq!(plan.current_version, 3);
    assert_eq!(plan.target_version, 5);

    // cleanup_old_keys:保留 keep_n 个最新旧版本(KEY-10):v1 被清,v2 保留,
    // 当前默认版本 v3 不受影响。
    let removed = km.cleanup_old_keys("app", 1).expect("cleanup");
    assert_eq!(removed, 1, "2 secondary keys → keep newest 1 → remove 1");
    let ring = km.get_key_by_version("app", 3).expect("ring");
    assert!(ring.is_some(), "current version must survive cleanup");
    assert!(
        ring.is_some() && km.get_key_by_version("app", 1).expect("ring").is_none(),
        "oldest v1 pruned"
    );
    assert!(
        km.get_key_by_version("app", 2).expect("ring").is_some(),
        "newest secondary kept"
    );

    // deprecate_version:当前版本拒绝(KEY-12 同族),存留旧版本允许(KEY-11)。
    assert!(km.deprecate_version("app", 3).is_err());
    km.deprecate_version("app", 2)
        .expect("old version deprecates");
    let deprecated = km
        .get_key_by_version("app", 2)
        .expect("ring")
        .expect("v2 still resolvable")
        .clone();
    assert_eq!(deprecated.metadata.status, KeyStatus::Deprecated);

    // set_default_key_id(非法 id) → 错误(KEY-12)。
    assert!(km.set_default_key_id("ghost").is_err());
    km.set_default_key_id("app").expect("valid id accepted");

    // KeyMetadata 时间边界(KEY-13)。
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let live = KeyMetadata::new(1, "e2e".to_string(), None);
    assert!(live.is_active() && !live.is_expired(), "no expiry → active");

    let mut expired = KeyMetadata::new(2, "e2e".to_string(), None);
    expired.expires_at = Some(now.saturating_sub(1));
    assert!(expired.is_expired(), "past expires_at → expired");
    assert!(!expired.is_active());
}
