// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! CLI command execution tests — inspect, validate, export, diff, snapshot.

use serial_test::serial;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// 直接执行 cargo test 注入的 confers bin(与 tests/e2e/cli_e2e.rs 惯例一致)。
/// 严禁在测试中 `cargo run/build`：那会以局部特性集重建 target/debug/confers，
/// 覆盖掉 `--all-features` 门禁构建的全特性 bin，令并行的 CLI 测试拿到缺特性的产物。
fn run_confers(args: &[&str]) -> std::process::Command {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_confers"));
    cmd.args(args);
    cmd
}

fn create_test_config(dir: &TempDir, filename: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(filename);
    let mut file = fs::File::create(&path).expect("Failed to create test config");
    file.write_all(content.as_bytes())
        .expect("Failed to write test config");
    path
}

#[test]
#[serial]
fn test_cli_compiles() {
    // bin 由 cargo test 按当前门禁特性集构建(CARGO_BIN_EXE 编译期注入)；
    // 此处只验证它可执行并响应 --version，不再触发 cargo build 覆盖产物。
    let bin = env!("CARGO_BIN_EXE_confers");
    let output = std::process::Command::new(bin)
        .arg("--version")
        .output()
        .expect("Failed to execute confers binary");

    assert!(
        output.status.success(),
        "CLI binary not runnable: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[serial]
fn test_inspect_command() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = create_test_config(
        &dir,
        "config.toml",
        r#"
[app]
name = "test-app"
version = "1.0.0"

[server]
host = "localhost"
port = 8080
"#,
    );

    let output = run_confers(&[
        "--allow-absolute-paths",
        "-c",
        config_path.to_str().unwrap(),
        "inspect",
    ])
    .output()
    .expect("Failed to run inspect command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Configuration Inspection") || stdout.contains("app"),
        "Output: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_inspect_json_output() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = create_test_config(
        &dir,
        "config.toml",
        r#"
[app]
name = "json-test"
"#,
    );

    let output = run_confers(&[
        "--allow-absolute-paths",
        "-c",
        config_path.to_str().unwrap(),
        "inspect",
        "-f",
        "json",
    ])
    .output()
    .expect("Failed to run inspect command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("inner") || stdout.contains("app"),
        "Output: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_validate_command() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = create_test_config(
        &dir,
        "config.toml",
        r#"
[app]
name = "validate-test"
"#,
    );

    let output = run_confers(&[
        "--allow-absolute-paths",
        "-c",
        config_path.to_str().unwrap(),
        "validate",
    ])
    .output()
    .expect("Failed to run validate command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Validation") || stdout.contains("valid"),
        "Output: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_validate_json_output() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = create_test_config(
        &dir,
        "config.toml",
        r#"
[app]
name = "validate-json-test"
"#,
    );

    let output = run_confers(&[
        "--allow-absolute-paths",
        "-c",
        config_path.to_str().unwrap(),
        "validate",
        "-f",
        "json",
    ])
    .output()
    .expect("Failed to run validate command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("valid"), "Output: {}", stdout);
}

#[test]
#[serial]
fn test_export_command_json() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = create_test_config(
        &dir,
        "config.toml",
        r#"
[app]
name = "export-test"
"#,
    );

    let output = run_confers(&[
        "--allow-absolute-paths",
        "-c",
        config_path.to_str().unwrap(),
        "export",
        "-f",
        "json",
    ])
    .output()
    .expect("Failed to run export command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("app") || stdout.contains("name"),
        "Output: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_export_command_toml() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = create_test_config(
        &dir,
        "config.toml",
        r#"
[app]
name = "export-toml-test"
"#,
    );

    let output = run_confers(&[
        "--allow-absolute-paths",
        "-c",
        config_path.to_str().unwrap(),
        "export",
        "-f",
        "toml",
    ])
    .output()
    .expect("Failed to run export command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("app") || stdout.contains("name"),
        "Output: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_export_with_provenance() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = create_test_config(
        &dir,
        "config.toml",
        r#"
[app]
name = "provenance-test"
"#,
    );

    let output = run_confers(&[
        "--allow-absolute-paths",
        "-c",
        config_path.to_str().unwrap(),
        "export",
        "-f",
        "json",
        "--with-provenance",
    ])
    .output()
    .expect("Failed to run export command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("source") || stdout.contains("priority"),
        "Output: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_export_to_file() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = create_test_config(
        &dir,
        "config.toml",
        r#"
[app]
name = "file-export-test"
"#,
    );
    let output_path = dir.path().join("output.json");

    let output = run_confers(&[
        "--allow-absolute-paths",
        "-c",
        config_path.to_str().unwrap(),
        "export",
        "-f",
        "json",
        "-o",
        output_path.to_str().unwrap(),
    ])
    .output()
    .expect("Failed to run export command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output_path.exists(), "Output file not created");

    let content = fs::read_to_string(&output_path).expect("Failed to read output file");
    assert!(content.contains("app"), "Content: {}", content);
}

#[test]
#[serial]
fn test_diff_command() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let base_path = create_test_config(
        &dir,
        "base.toml",
        r#"
[app]
name = "base-app"
version = "1.0.0"
"#,
    );
    let overlay_path = create_test_config(
        &dir,
        "overlay.toml",
        r#"
[app]
name = "overlay-app"
version = "2.0.0"
"#,
    );

    let output = run_confers(&[
        "--allow-absolute-paths",
        "diff",
        "--base",
        base_path.to_str().unwrap(),
        "--overlay",
        overlay_path.to_str().unwrap(),
    ])
    .output()
    .expect("Failed to run diff command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Diff") || stdout.contains("differ"),
        "Output: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_diff_json_output() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let base_path = create_test_config(
        &dir,
        "base.toml",
        r#"
[app]
name = "base"
"#,
    );
    let overlay_path = create_test_config(
        &dir,
        "overlay.toml",
        r#"
[app]
name = "overlay"
"#,
    );

    let output = run_confers(&[
        "--allow-absolute-paths",
        "diff",
        "--base",
        base_path.to_str().unwrap(),
        "--overlay",
        overlay_path.to_str().unwrap(),
        "-f",
        "json",
    ])
    .output()
    .expect("Failed to run diff command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("base") && stdout.contains("overlay"),
        "Output: {}",
        stdout
    );
}

#[test]
#[serial]
fn test_diff_identical_configs() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let config_content = r#"
[app]
name = "same"
"#;
    let base_path = create_test_config(&dir, "base.toml", config_content);
    let overlay_path = create_test_config(&dir, "overlay.toml", config_content);

    let output = run_confers(&[
        "--allow-absolute-paths",
        "diff",
        "--base",
        base_path.to_str().unwrap(),
        "--overlay",
        overlay_path.to_str().unwrap(),
    ])
    .output()
    .expect("Failed to run diff command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("identical"), "Output: {}", stdout);
}

#[test]
#[serial]
fn test_snapshot_list_empty() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let empty_dir = dir.path().join("snapshots");
    fs::create_dir_all(&empty_dir).expect("Failed to create snapshots dir");

    let output = run_confers(&[
        "snapshot",
        "list",
        "--directory",
        empty_dir.to_str().unwrap(),
    ])
    .output()
    .expect("Failed to run snapshot list command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No snapshots found"), "Output: {}", stdout);
}

#[test]
#[serial]
fn test_env_file_loading() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let env_path = create_test_config(
        &dir,
        ".env",
        r#"
TEST_VAR=test_value
"#,
    );
    let config_path = create_test_config(
        &dir,
        "config.toml",
        r#"
[app]
name = "env-test"
"#,
    );

    let output = run_confers(&[
        "--allow-absolute-paths",
        "--env-file",
        env_path.to_str().unwrap(),
        "-c",
        config_path.to_str().unwrap(),
        "inspect",
    ])
    .output()
    .expect("Failed to run with env file");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[serial]
fn test_help_flag() {
    let output = run_confers(&["--help"])
        .output()
        .expect("Failed to run help command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Configuration diagnostics tool"),
        "Output: {}",
        stdout
    );
    assert!(stdout.contains("inspect"), "Output: {}", stdout);
    assert!(stdout.contains("validate"), "Output: {}", stdout);
    assert!(stdout.contains("export"), "Output: {}", stdout);
    assert!(stdout.contains("diff"), "Output: {}", stdout);
    assert!(stdout.contains("snapshot"), "Output: {}", stdout);
    assert!(stdout.contains("schema"), "Output: {}", stdout);
    assert!(stdout.contains("get"), "Output: {}", stdout);
}

#[test]
#[serial]
fn test_version_flag() {
    let output = run_confers(&["--version"])
        .output()
        .expect("Failed to run version command");

    assert!(
        output.status.success(),
        "Command failed: {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("confers"), "Output: {}", stdout);
}

#[test]
#[serial]
fn test_schema_from_schema_generates_rust_scaffolding() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let schema_path = create_test_config(
        &dir,
        "schema.json",
        r#"{
            "title": "AppConfig",
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "port": {"type": "integer", "default": 8080},
                "status": {"type": "string", "enum": ["active", "paused"]},
                "nickname": {"type": "string"},
                "tags": {"type": "array", "items": {"type": "string"}}
            },
            "required": ["name", "status", "tags"]
        }"#,
    );
    let output = run_confers(&[
        "--allow-absolute-paths",
        "schema",
        "--from-schema",
        schema_path.to_str().expect("utf-8 path"),
    ])
    .output()
    .expect("Failed to run confers");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pub struct AppConfig"), "{stdout}");
    assert!(stdout.contains("pub name: String"), "{stdout}");
    assert!(
        stdout.contains("#[serde(default = \"_default_port\")]"),
        "{stdout}"
    );
    assert!(stdout.contains("pub enum AppConfigStatus"), "{stdout}");
    assert!(stdout.contains("pub nickname: Option<String>"), "{stdout}");
    assert!(stdout.contains("pub tags: Vec<String>"), "{stdout}");
}

/// 递归剥离草稿 schema 中空命名的 properties/required 条目。
///
/// 覆盖率插桩构建里 LLVM profile 运行时会在进程内 setenv
/// `__LLVM_PROFILE_RT_INIT_ONCE`，env 源按既定映射把前导下划线折叠成空的
/// config path 段（见 tests/core/env_types.rs 的
/// `test_env_parse_key_unicode_and_case` 对空键行为的断言），因此
/// `env_clear` 无法阻止这类插桩内部变量混入实例。空键不可能对应任何 Rust
/// 标识符，剥掉后第二跳只针对实例里真实可用的形状生成脚手架。
fn strip_empty_named_entries(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for child in map.values_mut() {
                strip_empty_named_entries(child);
            }
            if let Some(props) = map.get_mut("properties").and_then(|v| v.as_object_mut()) {
                props.retain(|name, _| !name.is_empty());
            }
            if let Some(required) = map.get_mut("required").and_then(|v| v.as_array_mut()) {
                required.retain(|entry| !entry.as_str().is_some_and(str::is_empty));
            }
        }
        serde_json::Value::Array(items) => {
            for item in items.iter_mut() {
                strip_empty_named_entries(item);
            }
        }
        _ => {}
    }
}

#[test]
#[serial]
fn test_schema_from_instance_feeds_from_schema_round_trip() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let config_path = create_test_config(
        &dir,
        "config.toml",
        "name = \"svc\"\nport = 8080\ntags = [\"a\", \"b\"]\n",
    );

    // 第一跳：配置实例 → JSON Schema 草稿。清空子进程环境：env 源按设计
    // 参与实例组装，继承宿主环境会让草稿混入不可预测的变量键。
    let infer_output = run_confers(&[
        "--allow-absolute-paths",
        "-c",
        config_path.to_str().expect("utf-8 path"),
        "schema",
        "--from-instance",
    ])
    .env_clear()
    .output()
    .expect("Failed to run confers");
    assert!(
        infer_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&infer_output.stderr)
    );
    let mut draft: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&infer_output.stdout))
            .expect("draft schema must be valid JSON");
    strip_empty_named_entries(&mut draft);
    let schema_json = serde_json::to_string(&draft).expect("serialize draft schema");

    // 第二跳：草稿 → 可编译 Rust 脚手架（端到端闭环）。
    let schema_path = create_test_config(&dir, "draft-schema.json", &schema_json);
    let output = run_confers(&[
        "--allow-absolute-paths",
        "schema",
        "--from-schema",
        schema_path.to_str().expect("utf-8 path"),
    ])
    .env_clear()
    .output()
    .expect("Failed to run confers");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("pub struct Config"), "{stdout}");
    assert!(stdout.contains("pub name: String"), "{stdout}");
    assert!(stdout.contains("pub port: i64"), "{stdout}");
    assert!(stdout.contains("pub tags: Vec<String>"), "{stdout}");
}

#[test]
#[serial]
fn test_schema_from_schema_fails_loud_on_unsupported_construct() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let schema_path = create_test_config(
        &dir,
        "bad_schema.json",
        r#"{
            "type": "object",
            "properties": {
                "weird": {"oneOf": [{"type": "string"}, {"type": "integer"}]}
            },
            "required": ["weird"]
        }"#,
    );
    let output = run_confers(&[
        "--allow-absolute-paths",
        "schema",
        "--from-schema",
        schema_path.to_str().expect("utf-8 path"),
    ])
    .output()
    .expect("Failed to run confers");

    assert!(!output.status.success(), "oneOf must fail loudly");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("oneOf"), "{stderr}");
}

#[test]
#[serial]
fn test_schema_from_schema_rejects_missing_file() {
    let dir = TempDir::new().expect("Failed to create temp dir");
    let missing = dir.path().join("nope.json");
    let output = run_confers(&[
        "--allow-absolute-paths",
        "schema",
        "--from-schema",
        missing.to_str().expect("utf-8 path"),
    ])
    .output()
    .expect("Failed to run confers");

    assert!(!output.status.success(), "missing schema file must fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Failed to read schema"), "{stderr}");
}

#[test]
#[serial]
fn test_schema_from_schema_conflicts_with_from_instance() {
    let output = run_confers(&["schema", "--from-instance", "--from-schema", "s.json"])
        .output()
        .expect("Failed to run confers");

    assert!(
        !output.status.success(),
        "clap must reject mutually exclusive flags"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("cannot be used with"), "{stderr}");
}
