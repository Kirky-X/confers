// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 插值(tests/e2e/interpolation_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.4):
//! - IPL-01 `${VAR}` 由 resolver 解析替换
//! - IPL-02 `${VAR:default}` 默认值语法:VAR 缺失时取默认
//! - IPL-03 未定义变量且无默认 → `InterpolationError`(2800 族)
//! - IPL-04 循环引用 A→B→A → `CircularReference`(2900 族)
//! - IPL-05 自引用 `${A}` 定义于 A 自身 → 报循环
//! - IPL-06 嵌套循环链(3 节点 A→B→C→A)检测并报路径
//! - IPL-07 敏感变量标记:`interpolate_tracked(..., true)` 记录 sensitive refs;
//!   `InterpolationContext.record` 后 `is_sensitive_ref` 生效
//! - IPL-08 截断模板 `"${VAR"`(未闭合)→ 明确解析错误不 panic
//!
//! IPL-09(examples/interpolation 全链路)由批 4 examples 验收覆盖。

use confers::interpolation::{InterpolationContext, interpolate, interpolate_tracked};
use confers::{ConfigError, ErrorCode};

#[test]
fn ipl01_resolver_replaces_variable() {
    let out = interpolate("Server: ${HOST}", &|name| {
        if name == "HOST" {
            Some("localhost".to_string())
        } else {
            None
        }
    })
    .expect("resolvable template must interpolate");

    assert_eq!(out, "Server: localhost");
}

#[test]
fn ipl02_default_value_used_when_variable_missing() {
    let out = interpolate("port=${PORT:8080}", &|_| None)
        .expect("default value must satisfy a missing variable");
    assert_eq!(out, "port=8080");

    let out = interpolate("port=${PORT:8080}", &|name| {
        if name == "PORT" {
            Some("9000".to_string())
        } else {
            None
        }
    })
    .expect("present variable must win over default");
    assert_eq!(out, "port=9000");
}

#[test]
fn ipl03_undefined_variable_without_default_is_an_error() {
    let err = interpolate("value=${TOTALLY_MISSING}", &|_| None)
        .expect_err("undefined variable without default must fail");

    assert!(
        matches!(err, ConfigError::InterpolationError { ref variable, .. } if variable == "TOTALLY_MISSING"),
        "expected InterpolationError, got: {err:?}"
    );
    assert_eq!(err.code(), ErrorCode::InterpolationError);
    assert!(
        err.to_string().contains("not found"),
        "message should explain the cause: {err}"
    );
}

/// resolver:A 的值又是模板 `${B}`,B 的值是 `${A}` → 展开期检出循环。
#[test]
fn ipl04_circular_reference_two_nodes_is_detected() {
    let err = interpolate("${A}", &|name| match name {
        "A" => Some("${B}".to_string()),
        "B" => Some("${A}".to_string()),
        _ => None,
    })
    .expect_err("A→B→A must be detected as circular");

    assert!(
        matches!(err, ConfigError::CircularReference { .. }),
        "expected CircularReference, got: {err:?}"
    );
    assert_eq!(err.code(), ErrorCode::CircularReference);
}

#[test]
fn ipl05_self_reference_is_detected_as_circular() {
    let err = interpolate("${A}", &|name| {
        if name == "A" {
            Some("${A}".to_string())
        } else {
            None
        }
    })
    .expect_err("self reference must be detected as circular");

    assert!(
        matches!(err, ConfigError::CircularReference { .. }),
        "expected CircularReference, got: {err:?}"
    );
}

#[test]
fn ipl06_three_node_cycle_is_detected_with_offending_variable() {
    let err = interpolate("${A}", &|name| match name {
        "A" => Some("${B}".to_string()),
        "B" => Some("${C}".to_string()),
        "C" => Some("${A}".to_string()),
        _ => None,
    })
    .expect_err("A→B→C→A must be detected as circular");

    match &err {
        ConfigError::CircularReference { path } => {
            assert!(!path.is_empty(), "cycle path must identify the variable");
        }
        other => panic!("expected CircularReference, got: {other:?}"),
    }
}

#[test]
fn ipl07_tracked_interpolation_records_sensitive_refs() {
    let result = interpolate_tracked(
        "token=${API_KEY}",
        &|name| {
            if name == "API_KEY" {
                Some("test-key".to_string())
            } else {
                None
            }
        },
        true,
    )
    .expect("tracked interpolation must succeed");

    assert!(
        result.has_sensitive_refs(),
        "sensitive flag must be recorded"
    );
    assert!(
        result.referenced("API_KEY"),
        "referenced var must be tracked"
    );
    assert_eq!(result.value, "token=test-key");

    // InterpolationContext.record 后 is_sensitive_ref 生效。
    let mut ctx = InterpolationContext::new();
    ctx.record("api_token_field", &result);
    assert!(
        ctx.is_sensitive_ref("API_KEY"),
        "record must make the sensitive ref queryable"
    );
    assert_eq!(ctx.sensitive_ref_field("API_KEY"), Some("api_token_field"));
}

#[test]
fn ipl08_unterminated_template_is_an_error_not_panic() {
    let err = interpolate("broken=${VAR", &|_| Some("x".to_string()))
        .expect_err("unterminated variable reference must fail");

    assert!(
        matches!(err, ConfigError::InterpolationError { ref message, .. }
            if message.contains("unterminated")),
        "expected unterminated-reference error, got: {err:?}"
    );
}
