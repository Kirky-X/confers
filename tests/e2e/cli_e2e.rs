// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: CLI(tests/e2e/cli_e2e.rs)—— 真实二进制 `confers`(
//! `CARGO_BIN_EXE_confers`,cargo 以 `--features cli` 构建后注入)。
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.24):
//! - CLI-03 `inspect -k 不存在的键` → `[NOT FOUND]` 且退出码 0
//! - CLI-04 `inspect --show-conflicts` → 被覆盖键带 `*` 标记
//! - CLI-06 `validate --strict` 有 issue 时非零退出码
//! - CLI-11 `export --raw` 向 stderr 打印未脱敏警告
//! - CLI-14 `diff` 传绝对路径 → 报错退出;全局 `--allow-absolute-paths` 后成功
//! - CLI-15 `snapshot list --directory` 非空目录输出快照表
//! - CLI-16 `snapshot diff --latest 2` 快照不足 2 份时友好提示(退出码 0,行为固化)
//! - CLI-17 `snapshot prune --older-than abc` → fail loud:invalid duration
//! - CLI-23 `export --format xml` → "Unsupported format" 非零退出
//! - CLI-24 inspect 对长字符串截断;UTF-8 多字节按字符截断不 panic
//! - SCH-05 validate `--format json` 对坏配置输出 `valid:false + issues`(schema 语义联动)
//!
//! CLI-01/02/05/07…13/15(空目录)/18…22 已有覆盖(tests/cli/、src 内联)。

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// 在 `dir` 内写一个相对路径文件并返回文件名(子进程以 `dir` 为 cwd)。
fn write_config(dir: &Path, name: &str, content: &str) -> String {
    std::fs::write(dir.join(name), content).expect("write config file");
    name.to_string()
}

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

#[test]
fn cli03_inspect_missing_key_reports_not_found_with_zero_exit() {
    let dir = setup_dir();
    let config = write_config(&dir, "app.toml", "server.port = 8080\n");

    let out = run_cli(&dir, &["-c", &config, "inspect", "-k", "nope.key"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "missing key must not fail: {}",
        stdout(&out)
    );
    assert!(
        stdout(&out).contains("[NOT FOUND]"),
        "missing key must print [NOT FOUND]: {}",
        stdout(&out)
    );

    let hit = run_cli(&dir, &["-c", &config, "inspect", "-k", "server.port"]);
    assert_eq!(hit.status.code(), Some(0));
    assert!(stdout(&hit).contains("8080"));
}

#[test]
fn cli04_show_conflicts_marks_overridden_keys() {
    let dir = setup_dir();
    let config = write_config(&dir, "app.toml", "server.port = 8080\n");

    // env 优先级高于文件:SERVER_PORT 覆盖文件值,env 行带 `*`(priority > 0)。
    let with_flag = run_cli_with(
        &dir,
        &[("SERVER_PORT", "1")],
        &["-c", &config, "inspect", "--show-conflicts"],
    );
    assert_eq!(with_flag.status.code(), Some(0));
    let marked = stdout(&with_flag);
    let row = marked
        .lines()
        .find(|l| l.starts_with("server.port"))
        .expect("server.port row must be present");
    assert!(row.ends_with('*'), "conflict row must end with '*': {row}");
    assert!(row.contains("env"), "winning source must be env: {row}");

    let without_flag = run_cli_with(&dir, &[("SERVER_PORT", "1")], &["-c", &config, "inspect"]);
    let body = stdout(&without_flag);
    let row = body
        .lines()
        .find(|l| l.starts_with("server.port"))
        .expect("server.port row must be present");
    assert!(!row.ends_with('*'), "no marker without flag: {row}");
}

#[test]
fn cli06_validate_strict_exits_nonzero_on_issues() {
    let dir = setup_dir();
    // 启发式校验:字符串形如数字 → issue。
    let config = write_config(&dir, "issue.toml", "version = \"123\"\n");

    let lax = run_cli(&dir, &["-c", &config, "validate"]);
    assert_eq!(
        lax.status.code(),
        Some(0),
        "lax mode exits 0: {}",
        stdout(&lax)
    );
    assert!(stdout(&lax).contains("Found 1 validation issue(s)"));

    let strict = run_cli(&dir, &["-c", &config, "validate", "--strict"]);
    assert_eq!(
        strict.status.code(),
        Some(1),
        "strict mode must exit non-zero: {}",
        stdout(&strict)
    );
}

#[test]
fn sch05_validate_json_reports_issues_for_bad_config() {
    let dir = setup_dir();
    let config = write_config(&dir, "issue.toml", "version = \"123\"\n");

    let out = run_cli(&dir, &["-c", &config, "validate", "--format", "json"]);
    assert_eq!(out.status.code(), Some(0));
    let body = stdout(&out);
    let json: serde_json::Value = serde_json::from_str(&body).expect("validate json must parse");
    assert_eq!(json["valid"], serde_json::Value::Bool(false));
    let issues = json["issues"].as_array().expect("issues array");
    assert!(!issues.is_empty(), "bad config must surface issues: {body}");
}

#[test]
fn cli11_export_raw_warns_on_stderr() {
    let dir = setup_dir();
    let config = write_config(&dir, "app.toml", "password = \"hunter2\"\n");

    let out = run_cli(&dir, &["-c", &config, "export", "--raw"]);
    assert_eq!(out.status.code(), Some(0));
    let err = stderr(&out);
    assert!(
        err.contains("raw") || err.contains("敏感"),
        "stderr must warn about raw sensitive export: {err}"
    );

    let safe = run_cli(&dir, &["-c", &config, "export"]);
    assert!(
        !stderr(&safe).contains("raw") && !stderr(&safe).contains("敏感"),
        "no raw warning without --raw: {}",
        stderr(&safe)
    );
}

#[test]
fn cli14_diff_rejects_absolute_paths_then_allows_with_flag() {
    let dir = setup_dir();
    let base = dir.join("base.toml");
    let overlay = dir.join("overlay.toml");
    std::fs::write(&base, "server.port = 8080\n").unwrap();
    std::fs::write(&overlay, "server.port = 9090\n").unwrap();
    let base = base.to_string_lossy().into_owned();
    let overlay = overlay.to_string_lossy().into_owned();

    let rejected = run_cli(&dir, &["diff", "--base", &base, "--overlay", &overlay]);
    assert_eq!(
        rejected.status.code(),
        Some(1),
        "absolute paths must be rejected: {}",
        stderr(&rejected)
    );
    assert!(
        stderr(&rejected).contains("Absolute path not allowed"),
        "rejection message must mention absolute path policy: {}",
        stderr(&rejected)
    );

    let allowed = run_cli(
        &dir,
        &[
            "--allow-absolute-paths",
            "diff",
            "--base",
            &base,
            "--overlay",
            &overlay,
        ],
    );
    assert_eq!(
        allowed.status.code(),
        Some(0),
        "global flag must allow absolute paths: {} {}",
        stdout(&allowed),
        stderr(&allowed)
    );
    assert!(stdout(&allowed).contains("Configuration Diff"));
}

#[test]
fn cli15_snapshot_list_outputs_snapshot_table() {
    let dir = setup_dir();
    let snaps = dir.join("snaps");
    std::fs::create_dir(&snaps).unwrap();
    std::fs::write(
        snaps.join("snapshot_20260101_000000.json"),
        r#"{"config":{"a":1}}"#,
    )
    .unwrap();
    std::fs::write(
        snaps.join("snapshot_20260102_000000.json"),
        r#"{"config":{"a":2}}"#,
    )
    .unwrap();

    let out = run_cli(&dir, &["snapshot", "list", "--directory", "snaps"]);
    assert_eq!(out.status.code(), Some(0));
    let body = stdout(&out);
    assert!(body.contains("Snapshots in"), "list header: {body}");
    assert!(body.contains("snapshot_20260101_000000.json"));
    assert!(body.contains("snapshot_20260102_000000.json"));
    assert!(body.contains("json"), "format column: {body}");
}

#[test]
fn cli16_snapshot_diff_with_insufficient_snapshots_exits_cleanly() {
    let dir = setup_dir();
    let snaps = dir.join("snaps");
    std::fs::create_dir(&snaps).unwrap();
    std::fs::write(
        snaps.join("snapshot_20260101_000000.json"),
        r#"{"config":{"a":1}}"#,
    )
    .unwrap();

    let out = run_cli(
        &dir,
        &["snapshot", "diff", "--latest", "2", "--directory", "snaps"],
    );
    // 行为固化:不足 2 份时输出友好提示并成功退出(非 fail loud)。
    assert_eq!(out.status.code(), Some(0), "{}", stdout(&out));
    assert!(
        stdout(&out).contains("Need at least 2 snapshots"),
        "friendly message expected: {}",
        stdout(&out)
    );
}

#[test]
fn cli17_snapshot_prune_invalid_duration_fails_loud() {
    let dir = setup_dir();
    let snaps = dir.join("snaps");
    std::fs::create_dir(&snaps).unwrap();

    let out = run_cli(
        &dir,
        &[
            "snapshot",
            "prune",
            "--older-than",
            "abc",
            "--directory",
            "snaps",
        ],
    );
    assert_eq!(
        out.status.code(),
        Some(1),
        "invalid duration must fail loud"
    );
    let err = stderr(&out);
    assert!(
        err.contains("invalid duration") || err.contains("invalid digit"),
        "error must explain the bad duration: {err}"
    );
}

#[test]
fn cli23_export_unsupported_format_bails_nonzero() {
    let dir = setup_dir();
    let config = write_config(&dir, "app.toml", "server.port = 8080\n");

    let out = run_cli(&dir, &["-c", &config, "export", "--format", "xml"]);
    assert_eq!(out.status.code(), Some(1), "unsupported format must bail");
    let err = stderr(&out);
    assert!(
        err.contains("Unsupported format"),
        "error must name the problem: {err}"
    );
}

#[test]
fn cli24_inspect_truncates_long_strings_char_safely() {
    let dir = setup_dir();
    let ascii = "this_is_a_very_long_string_value_for_truncation_testing";
    let cjk = "一二三四五六七八九十一二三四五六七八九十一二三四五六七八九十AB";
    let config = write_config(
        &dir,
        "long.toml",
        &format!("description = \"{ascii}\"\n\"描述\" = \"{cjk}\"\n"),
    );

    let out = run_cli(&dir, &["-c", &config, "inspect"]);
    assert_eq!(out.status.code(), Some(0));
    let body = stdout(&out);

    let row = body
        .lines()
        .find(|l| l.starts_with("description"))
        .expect("description row");
    assert!(row.contains("..."), "long value must be truncated: {row}");
    assert!(
        !row.contains(ascii),
        "full value must not be printed: {row}"
    );

    let cjk_row = body
        .lines()
        .find(|l| l.starts_with("描述"))
        .expect("utf-8 key row must exist (no panic on multibyte)");
    assert!(
        cjk_row.contains("...") && cjk_row.contains("..."),
        "utf-8 value must be truncated on char boundaries: {cjk_row}"
    );
    assert!(!body.contains('\u{fffd}'), "no replacement chars allowed");
}
