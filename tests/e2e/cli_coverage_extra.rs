// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! E2E: CLI 覆盖率补测(tests/e2e/cli_coverage_extra.rs)——真实二进制
//! `confers`(`CARGO_BIN_EXE_confers`)上未被既有场景覆盖的分支:
//! snapshot restore/prune/diff 完整路径、doctor 各健康度分支、
//! docs 知识包、schema --from-instance、get 脱敏/`--env-file`、
//! export 全格式与目录输出。
//!
//! 退出码契约(src/cli/mod.rs):`0` 成功/doctor healthy,`1` 配置错误/
//! doctor warnings,`2` I/O 错误/doctor errors。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run_cli(dir: &Path, args: &[&str]) -> Output {
    run_cli_with(dir, &[], args)
}

/// `envs` 仅注入子进程,不污染测试进程(规避并发 env 竞争)。
fn run_cli_with(dir: &Path, envs: &[(&str, &str)], args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_confers"));
    cmd.current_dir(dir).args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd.output().expect("spawn confers binary")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn setup_dir() -> PathBuf {
    tempfile::tempdir().unwrap().keep()
}

fn write_file(dir: &Path, name: &str, content: &str) -> String {
    std::fs::write(dir.join(name), content).expect("write file");
    name.to_string()
}

// ============================================================================
// snapshot restore
// ============================================================================

#[test]
fn snapshot_restore_from_explicit_file_prints_summary() {
    let dir = setup_dir();
    let snaps = dir.join("snaps");
    std::fs::create_dir_all(&snaps).unwrap();
    // CLI 的 SnapshotConfig 默认 TOML 格式:快照文件按 TOML 解析。
    write_file(
        &snaps,
        "snapshot_20260101_000000.toml",
        "[server]\nhost = \"a\"\nport = 1\nflag = true\n",
    );

    let out = run_cli(
        &dir,
        &[
            "snapshot",
            "restore",
            "--file",
            "snaps/snapshot_20260101_000000.toml",
            "--directory",
            "snaps",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let body = stdout(&out);
    assert!(body.contains("restored"), "must confirm restore: {body}");
    assert!(
        body.contains("2"),
        "must count top-level keys (server, flag): {body}"
    );
}

/// 缺文件 → 明确提示 + 退出码 1(配置级错误),而不是 panic。
#[test]
fn snapshot_restore_missing_file_fails_loudly() {
    let dir = setup_dir();
    std::fs::create_dir_all(dir.join("snaps")).unwrap();

    let out = run_cli(
        &dir,
        &[
            "snapshot",
            "restore",
            "--file",
            "snaps/absent.json",
            "--directory",
            "snaps",
        ],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stdout(&out).contains("absent.json"),
        "must name the missing file: {}",
        stdout(&out)
    );
}

/// 无 `--file` 且目录为空 → 提示无快照 + 退出码 1。
#[test]
fn snapshot_restore_without_file_and_empty_directory_fails_loudly() {
    let dir = setup_dir();
    std::fs::create_dir_all(dir.join("snaps")).unwrap();

    let out = run_cli(&dir, &["snapshot", "restore", "--directory", "snaps"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stdout(&out).contains("snaps"),
        "must name the directory: {}",
        stdout(&out)
    );
}

/// 无 `--file` → 选目录里最新的快照恢复(list_snapshots 按创建时间排序)。
#[test]
fn snapshot_restore_without_file_picks_the_newest_snapshot() {
    let dir = setup_dir();
    let snaps = dir.join("snaps");
    std::fs::create_dir_all(&snaps).unwrap();
    // 先后创建,保证创建时间(birth time)可区分。
    std::fs::write(snaps.join("snapshot_20260101_000000.toml"), "gen = 1\n").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    std::fs::write(snaps.join("snapshot_20260102_000000.toml"), "gen = 2\n").unwrap();

    let out = run_cli(&dir, &["snapshot", "restore", "--directory", "snaps"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(stdout(&out).contains("snapshot_20260102_000000.toml"));
}

// ============================================================================
// snapshot prune / diff
// ============================================================================

#[test]
fn snapshot_prune_zero_days_removes_every_snapshot_and_counts_skips() {
    let dir = setup_dir();
    let snaps = dir.join("snaps");
    std::fs::create_dir_all(&snaps).unwrap();
    write_file(&snaps, "snapshot_20260101_000000.json", "{}");
    write_file(&snaps, "snapshot_20260102_000000.toml", "");
    write_file(&snaps, "notes.txt", "keep me"); // 非快照扩展名 → skipped

    let out = run_cli(
        &dir,
        &[
            "snapshot",
            "prune",
            "--older-than",
            "0d",
            "--directory",
            "snaps",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let body = stdout(&out);
    assert!(
        body.contains("Removing: snapshot_20260101_000000.json"),
        "{body}"
    );
    assert!(
        body.contains("Removing: snapshot_20260102_000000.toml"),
        "{body}"
    );
    assert!(
        body.contains("Pruned 2 snapshot(s) older than 0 days (0 failed, 0 skipped)"),
        "summary must report removed/failed/skipped counts: {body}"
    );
    assert!(!snaps.join("snapshot_20260101_000000.json").exists());
    assert!(
        snaps.join("notes.txt").exists(),
        "non-snapshot files survive"
    );
}

#[test]
fn snapshot_prune_missing_directory_is_a_clean_noop() {
    let dir = setup_dir();
    let out = run_cli(
        &dir,
        &[
            "snapshot",
            "prune",
            "--older-than",
            "30d",
            "--directory",
            "absent",
        ],
    );
    assert_eq!(out.status.code(), Some(0));
    assert!(stdout(&out).contains("absent"));
}

#[test]
fn snapshot_diff_prints_changes_between_two_snapshots() {
    let dir = setup_dir();
    let snaps = dir.join("snaps");
    std::fs::create_dir_all(&snaps).unwrap();
    let a = snaps.join("snapshot_20260101_000000.json");
    let b = snaps.join("snapshot_20260102_000000.json");
    std::fs::write(&a, "{\n  \"port\": 1\n}\n").unwrap();
    std::fs::write(&b, "{\n  \"port\": 2\n}\n").unwrap();
    let t_old = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
    let f = std::fs::File::options().write(true).open(&a).unwrap();
    f.set_modified(t_old).unwrap();

    let out = run_cli(
        &dir,
        &["snapshot", "diff", "--latest", "2", "--directory", "snaps"],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let body = stdout(&out);
    assert!(body.contains("Diff between"), "{body}");
    // 两份快照的 port 值都必须出现在差异输出里(新旧两侧)。
    assert!(body.contains("1") && body.contains("2"), "{body}");
}

// ============================================================================
// doctor
// ============================================================================

#[test]
fn doctor_healthy_config_reports_json_and_exits_zero() {
    let dir = setup_dir();
    let config = write_file(
        &dir,
        "app.toml",
        "server.host = \"localhost\"\nserver.port = 8080\n",
    );

    let out = run_cli(&dir, &["-c", &config, "doctor"]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout(&out));
    let report: serde_json::Value =
        serde_json::from_str(stdout(&out).trim()).expect("single-line JSON report");
    assert_eq!(report["status"], "healthy");
    assert_eq!(report["exit_code"], 0);
    let names: Vec<&str> = report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"load"));
    assert!(names.contains(&"schema"));
    assert!(names.contains(&"sources"));
    assert!(names.contains(&"encryption"));
    assert!(names.contains(&"env_overrides"));
}

#[test]
fn doctor_text_format_lists_each_check() {
    let dir = setup_dir();
    let config = write_file(&dir, "app.toml", "server.port = 8080\n");

    let out = run_cli(&dir, &["-c", &config, "doctor", "--format", "text"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let body = stdout(&out);
    assert!(body.contains("Doctor (status:"), "{body}");
    assert!(body.contains("load"), "{body}");
    assert!(body.contains("sources"), "{body}");
}

/// 无法解析的配置 → load check 报 error,其余 checks 标记 skipped,
/// 退出码 2(doctor errors)。
#[test]
fn doctor_broken_config_reports_load_error_with_skipped_checks() {
    let dir = setup_dir();
    let config = write_file(&dir, "broken.toml", "this is not [ valid toml\n");

    let out = run_cli(&dir, &["-c", &config, "doctor"]);
    assert_eq!(out.status.code(), Some(2), "{}", stdout(&out));
    let body = stdout(&out);
    assert!(body.contains("failed to load"), "{body}");
    for skipped in ["schema", "sources", "encryption", "env_overrides"] {
        assert!(
            body.contains(skipped),
            "skipped check {skipped} must be listed: {body}"
        );
    }
}

/// 两个 env 拼写指向同一个键且值不同 → env_overrides warning,退出码 1。
#[test]
fn doctor_reports_conflicting_env_overrides() {
    let dir = setup_dir();
    let config = write_file(
        &dir,
        "app.toml",
        "server.host = \"file\"\nserver.port = 1\n",
    );

    let out = run_cli_with(
        &dir,
        &[
            ("SERVER__HOST", "double-underscore"),
            ("SERVER_HOST", "single"),
        ],
        &["-c", &config, "doctor"],
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "warnings exit 1: {}",
        stdout(&out)
    );
    let body = stdout(&out);
    assert!(body.contains("env override conflict"), "{body}");
    assert!(body.contains("server.host"), "{body}");
    assert!(
        body.contains("status\":\"warning") || body.contains("\"warning\""),
        "{body}"
    );
}

/// 有加密字段但未设 CONFERS_MASTER_KEY → encryption warning,退出码 1。
#[test]
fn doctor_encrypted_field_without_master_key_warns() {
    let dir = setup_dir();
    let config = write_file(
        &dir,
        "app.toml",
        "server.port = 1\ndb.password = \"enc:v1:k1:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\"\n",
    );

    let out = run_cli(&dir, &["-c", &config, "doctor"]);
    assert_eq!(out.status.code(), Some(1), "{}", stdout(&out));
    let body = stdout(&out);
    assert!(body.contains("CONFERS_MASTER_KEY"), "{body}");
}

/// CONFERS_MASTER_KEY 不是合法 32 字节 hex → encryption error,退出码 2。
#[test]
fn doctor_invalid_master_key_errors() {
    let dir = setup_dir();
    let config = write_file(
        &dir,
        "app.toml",
        "db.password = \"enc:v1:k1:AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\"\n",
    );

    let out = run_cli_with(
        &dir,
        &[("CONFERS_MASTER_KEY", "not-hex-at-all")],
        &["-c", &config, "doctor"],
    );
    assert_eq!(out.status.code(), Some(2), "{}", stdout(&out));
    let body = stdout(&out);
    assert!(body.contains("not a valid 32-byte hex key"), "{body}");
}

// ============================================================================
// docs / schema --from-instance
// ============================================================================

#[test]
fn docs_agent_emits_json_and_markdown_knowledge_packs() {
    let dir = setup_dir();

    let json_out = run_cli(&dir, &["docs", "--agent"]);
    assert_eq!(json_out.status.code(), Some(0), "{}", stderr(&json_out));
    let pack: serde_json::Value =
        serde_json::from_str(stdout(&json_out).trim()).expect("JSON knowledge pack");
    assert!(pack.get("subcommands").is_some(), "{pack}");

    let md_out = run_cli(&dir, &["docs", "--agent", "--format", "markdown"]);
    assert_eq!(md_out.status.code(), Some(0), "{}", stderr(&md_out));
    let md = stdout(&md_out);
    assert!(md.contains('#'), "markdown headings expected: {md}");
    assert!(
        md.contains("Exit codes"),
        "exit-code contract documented: {md}"
    );
}

/// `docs` 不带 `--agent` → 明确报错(规则:fail loud),非零退出。
#[test]
fn docs_without_agent_flag_fails_loudly() {
    let dir = setup_dir();
    let out = run_cli(&dir, &["docs"]);
    assert_ne!(out.status.code(), Some(0));
    assert!(stderr(&out).contains("nothing to show"), "{}", stderr(&out));
}

#[test]
fn schema_from_instance_reverse_engineers_the_shape() {
    let dir = setup_dir();
    let config = write_file(
        &dir,
        "app.toml",
        "server.host = \"h\"\nserver.ports = [1, 2]\ntag = \"x\"\n",
    );

    let out = run_cli(&dir, &["-c", &config, "schema", "--from-instance"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let schema: serde_json::Value = serde_json::from_str(stdout(&out).trim()).expect("schema JSON");
    assert_eq!(schema["type"], "object");
    assert!(schema["properties"]["server"].is_object());
    assert_eq!(
        schema["properties"]["server"]["properties"]["host"]["type"],
        "string"
    );
    assert_eq!(
        schema["properties"]["server"]["properties"]["ports"]["type"],
        "array"
    );
    assert_eq!(schema["properties"]["tag"]["type"], "string");
    let required: Vec<&str> = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(required.contains(&"server") && required.contains(&"tag"));
}

// ============================================================================
// get:脱敏、--reveal、--fields 过滤、--env-file
// ============================================================================

#[test]
fn get_masks_sensitive_keys_and_reveals_on_demand() {
    let dir = setup_dir();
    let config = write_file(
        &dir,
        "app.toml",
        "server.port = 8080\ndb.password = \"hush\"\n",
    );

    let masked = run_cli(&dir, &["-c", &config, "get", "db.password"]);
    assert_eq!(masked.status.code(), Some(0), "{}", stderr(&masked));
    assert_eq!(stdout(&masked).trim(), "\"********\"");

    let revealed = run_cli(&dir, &["-c", &config, "get", "--reveal", "db.password"]);
    assert_eq!(revealed.status.code(), Some(0), "{}", stderr(&revealed));
    assert_eq!(stdout(&revealed).trim(), "\"hush\"");
    assert!(
        stderr(&revealed).to_lowercase().contains("reveal"),
        "reveal warns on stderr: {}",
        stderr(&revealed)
    );

    // 嵌套对象同样按叶子键名脱敏。
    let obj = run_cli(&dir, &["-c", &config, "get", "db"]);
    assert_eq!(obj.status.code(), Some(0));
    assert!(stdout(&obj).contains("********"), "{}", stdout(&obj));
}

#[test]
fn env_file_flag_loads_variables_into_the_config_chain() {
    let dir = setup_dir();
    let config = write_file(&dir, "app.toml", "server.port = 8080\n");

    // 基线:写入 .env 之前取值(避免 dotenvy 自动加载干扰)。
    let without = run_cli(&dir, &["-c", &config, "get", "server.port"]);
    assert_eq!(stdout(&without).trim(), "8080");

    write_file(&dir, ".env", "SERVER_PORT=9999\n");
    let with = run_cli(
        &dir,
        &["--env-file", ".env", "-c", &config, "get", "server.port"],
    );
    assert_eq!(with.status.code(), Some(0), "{}", stderr(&with));
    assert_eq!(
        stdout(&with).trim(),
        "9999",
        "env-file must override file value"
    );
}

// ============================================================================
// export:全格式、目录输出、--with-provenance
// ============================================================================

#[test]
fn export_emits_toml_yaml_and_writes_into_output_directory() {
    let dir = setup_dir();
    let config = write_file(
        &dir,
        "app.toml",
        "server.host = \"h\"\nserver.port = 8080\n",
    );

    let toml_out = run_cli(&dir, &["-c", &config, "export", "--format", "toml"]);
    assert_eq!(toml_out.status.code(), Some(0), "{}", stderr(&toml_out));
    assert!(
        stdout(&toml_out).contains("[server]"),
        "{}",
        stdout(&toml_out)
    );

    let yaml_out = run_cli(&dir, &["-c", &config, "export", "--format", "yaml"]);
    assert_eq!(yaml_out.status.code(), Some(0), "{}", stderr(&yaml_out));
    assert!(
        stdout(&yaml_out).contains("server:"),
        "{}",
        stdout(&yaml_out)
    );

    let out_dir = dir.join("exports");
    std::fs::create_dir_all(&out_dir).unwrap();
    let json_to_dir = run_cli(
        &dir,
        &[
            "-c", &config, "export", "--format", "json", "--output", "exports",
        ],
    );
    assert_eq!(
        json_to_dir.status.code(),
        Some(0),
        "{}",
        stderr(&json_to_dir)
    );
    assert!(
        stdout(&json_to_dir).contains("Exported configuration to:"),
        "{}",
        stdout(&json_to_dir)
    );
    assert!(out_dir.join("exports").is_dir() || out_dir.read_dir().unwrap().next().is_some());
}

#[test]
fn export_with_provenance_prints_annotated_tree_and_writes_files() {
    let dir = setup_dir();
    let config = write_file(
        &dir,
        "app.toml",
        "server.host = \"h\"\nserver.port = 8080\n",
    );

    let prov = run_cli(
        &dir,
        &[
            "-c",
            &config,
            "export",
            "--with-provenance",
            "--format",
            "json",
        ],
    );
    assert_eq!(prov.status.code(), Some(0), "{}", stderr(&prov));
    let body = stdout(&prov);
    assert!(
        body.contains("\"source\""),
        "annotated tree carries provenance: {body}"
    );

    // 目录输出 + json 格式:文件落盘。(provenance 树含 null 字段, TOML
    // 序列化按设计报 unsupported unit type —— json 是 provenance 的出路。)
    let out_dir = dir.join("prov");
    std::fs::create_dir_all(&out_dir).unwrap();
    let written = run_cli(
        &dir,
        &[
            "-c",
            &config,
            "export",
            "--with-provenance",
            "--format",
            "json",
            "--output",
            "prov",
        ],
    );
    assert_eq!(written.status.code(), Some(0), "{}", stderr(&written));
    assert!(
        stdout(&written).contains("Exported annotated configuration to:"),
        "{}",
        stdout(&written)
    );
    assert!(out_dir.read_dir().unwrap().next().is_some(), "file written");
}

// ============================================================================
// diff:json 格式与 --sanitize
// ============================================================================

#[test]
fn diff_json_format_reports_both_sides_and_sanitize_flag_is_accepted() {
    let dir = setup_dir();
    // AWS 访问键样例(匹配 sanitize 的 AKIA 模式)——脱敏断言有据可依。
    let base = write_file(
        &dir,
        "base.toml",
        "server.port = 1\ncreds.access_key = \"AKIAIOSFODNN7EXAMPLE\"\n",
    );
    let overlay = write_file(
        &dir,
        "overlay.toml",
        "server.port = 2\ncreds.access_key = \"AKIAIOSFODNN7EXAMPLE\"\n",
    );

    let out = run_cli(
        &dir,
        &[
            "diff",
            "--base",
            &base,
            "--overlay",
            &overlay,
            "--format",
            "json",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let body = stdout(&out);
    assert!(
        body.contains("\"base\"") && body.contains("\"overlay\""),
        "{body}"
    );
    assert!(body.contains("\"identical\": false"), "{body}");
    // sanitize 默认开启:匹配 AKIA 模式的值被替换,不回显原值。
    assert!(body.contains("\"sanitize\": true"), "{body}");
    assert!(
        !body.contains("AKIAIOSFODNN7EXAMPLE"),
        "sanitized diff must not echo raw keys: {body}"
    );
    assert!(body.contains("<aws_access_key>"), "{body}");
}

#[test]
fn diff_identical_configs_report_equality() {
    let dir = setup_dir();
    let a = write_file(&dir, "a.toml", "server.port = 1\n");
    let b = write_file(&dir, "b.toml", "server.port = 1\n");

    let out = run_cli(&dir, &["diff", "--base", &a, "--overlay", &b]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(stdout(&out).contains("Configurations are identical"));
}

// ============================================================================
// validate:嵌套与数组位置的类型启发式
// ============================================================================

#[test]
fn validate_flags_number_like_strings_in_nested_and_array_positions() {
    let dir = setup_dir();
    let config = write_file(
        &dir,
        "app.toml",
        "server.port = 8080\n[server.auth]\ntimeout = \"300\"\nflags = [\"true\", 1]\n",
    );

    let out = run_cli(&dir, &["-c", &config, "validate"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "lax mode exits 0: {}",
        stdout(&out)
    );
    let body = stdout(&out);
    assert!(body.contains("server.auth.timeout"), "{body}");
    assert!(body.contains("server.auth.flags[0]"), "{body}");
}
