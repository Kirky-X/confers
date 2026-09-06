// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: feature 预设编译矩阵(tests/e2e/presets_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.28):
//! - PRS-01 本文件声明于 `[[test]]` 且**不设 required-features**——任何特性
//!   组合(含 `--no-default-features`)都会编译并运行它,即"仅 core 也能全库
//!   编译"的固化门禁;用例本体只依赖无 feature 的核心公开 API。
//! - PRS-02…06 minimal/recommended/dev/production/distributed 预设定义与
//!   §3.3 展开清单一致(从本仓 Cargo.toml [features] 现场解析)。
//! - PRS-07 full 预设覆盖全部功能域 feature;`cfg(feature = "full")` 分支在
//!   full 构建态做正向断言。
//! - PRS-08 §3.1 依赖链逐条核对(security→encryption、nats-bus→config-bus 等)。
//!
//! PRS 编译矩阵的**执行记录**(37 组特性组合逐组 cargo test 全绿)见
//! reviews/acceptance-report.md 特性组合台账;本文件固化其输入依据的结构完整性。

use std::collections::HashMap;
use std::path::PathBuf;

/// 从本仓 Cargo.toml 解析 [features] 段(name → 直接成员,dep: 前缀剔除)。
fn parse_features() -> HashMap<String, Vec<String>> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let content = std::fs::read_to_string(&manifest).expect("Cargo.toml readable");

    let mut in_features = false;
    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_features = trimmed == "[features]";
            continue;
        }
        if !in_features || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((name, value)) = trimmed.split_once('=') else {
            continue;
        };
        let members: Vec<String> = value
            .split(',')
            .map(|s| s.trim().trim_matches(['"', '[', ']', ' ']).to_string())
            .filter(|s| !s.is_empty() && !s.starts_with("dep:") && !s.starts_with('#'))
            .collect();
        map.insert(name.trim().to_string(), members);
    }
    map
}

fn assert_contains(features: &HashMap<String, Vec<String>>, preset: &str, expected: &[&str]) {
    let members = features
        .get(preset)
        .unwrap_or_else(|| panic!("preset '{preset}' must be defined"));
    for member in expected {
        assert!(
            members.iter().any(|m| m == member),
            "preset '{preset}' must include '{member}', got {members:?}"
        );
    }
}

#[test]
fn prs01_core_api_compiles_without_any_feature() {
    // 本测试(与整个文件)在 --no-default-features 下仍被编译运行:
    // 这里只触碰无 feature 依赖的核心公开 API,即 PRS-01 门禁的运行态复验。
    let value = confers::types::AnnotatedValue::new(
        confers::types::ConfigValue::string("core"),
        confers::types::SourceId::new("prs01"),
        "prs01",
    );
    assert_eq!(value.inner.as_str(), Some("core"));

    // [features] 段结构完整可解析。
    let features = parse_features();
    assert!(
        features.contains_key("default"),
        "default preset must exist"
    );
    assert!(features.len() > 30, "expected the full feature surface");
}

#[test]
fn prs0203040506_preset_expansions_match_doc() {
    let features = parse_features();

    // default(§3.3 最小可用)。
    assert_contains(&features, "default", &["toml", "json", "env"]);
    // minimal(PRS-02):env+json,无 toml。
    assert_contains(&features, "minimal", &["env", "json"]);
    assert!(
        !features["minimal"].iter().any(|m| m == "toml"),
        "minimal must not include toml"
    );
    // recommended(PRS-03):security-rules 隐式带 security→encryption。
    assert_contains(
        &features,
        "recommended",
        &["toml", "env", "validation", "json", "security-rules"],
    );
    // dev(PRS-04):12 项。
    assert_contains(
        &features,
        "dev",
        &[
            "toml",
            "json",
            "yaml",
            "env",
            "cli",
            "validation",
            "schema",
            "audit",
            "watch",
            "migration",
            "snapshot",
            "dynamic",
        ],
    );
    assert_eq!(features["dev"].len(), 12, "dev preset must stay 12 items");
    // production(PRS-05):14 项。
    assert_contains(
        &features,
        "production",
        &[
            "toml",
            "env",
            "watch",
            "encryption",
            "validation",
            "audit",
            "schema",
            "cli",
            "migration",
            "dynamic",
            "progressive-reload",
            "snapshot",
            "security-rules",
            "feature-toggle",
        ],
    );
    assert_eq!(
        features["production"].len(),
        14,
        "production preset must stay 14 items"
    );
    // distributed(PRS-06):8 项,总线仅进程内。
    assert_contains(
        &features,
        "distributed",
        &[
            "toml",
            "json",
            "env",
            "watch",
            "validation",
            "config-bus",
            "progressive-reload",
            "audit",
        ],
    );
    assert_eq!(
        features["distributed"].len(),
        8,
        "distributed preset must stay 8 items"
    );
    assert!(
        !features["distributed"]
            .iter()
            .any(|m| m == "nats-bus" || m == "redis-bus"),
        "distributed must not pull external buses"
    );
}

#[test]
fn prs07_full_preset_covers_all_domain_features() {
    let features = parse_features();
    let domains = [
        "toml",
        "json",
        "yaml",
        "ini",
        "env",
        "dotenv",
        "cli",
        "validation",
        "watch",
        "encryption",
        "security",
        "security-rules",
        "key",
        "schema",
        "typescript-schema",
        "dynamic",
        "progressive-reload",
        "audit",
        "migration",
        "snapshot",
        "interpolation",
        "remote",
        "config-bus",
        "nats-bus",
        "redis-bus",
        "context-aware",
        "modules",
        "etcd",
        "consul",
        "feature-toggle",
    ];
    assert_contains(&features, "full", &domains);
    assert_eq!(
        features["full"].len(),
        30,
        "full preset must stay the 30-item surface"
    );

    // full 构建态正向断言(PRS-07:full 编译 + 测试可通过)。
    #[cfg(feature = "full")]
    {
        assert!(
            features["full"].contains(&"interpolation".to_string()),
            "full build must declare interpolation"
        );
    }
}

#[test]
fn prs08_dependency_chains_from_doc_section_3_1() {
    let features = parse_features();

    let chains: &[(&str, &[&str])] = &[
        ("security", &["encryption"]),
        ("security-rules", &["security"]),
        ("key", &["encryption"]),
        ("cli", &["toml", "json", "yaml"]),
        ("progressive-reload", &["watch"]),
        ("snapshot", &["json", "toml", "yaml", "dynamic"]),
        ("etcd", &["remote"]),
        ("consul", &["remote"]),
        ("nats-bus", &["config-bus"]),
        ("redis-bus", &["config-bus"]),
        ("typescript-schema", &["schema"]),
        ("dotenv", &["env"]),
        ("default", &["toml", "json", "env"]),
        ("modules", &["toml"]),
    ];
    for (feature, deps) in chains {
        assert_contains(&features, feature, deps);
    }
}
