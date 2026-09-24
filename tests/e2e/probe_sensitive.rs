// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! E2E: 敏感字段 `_FILE` 引用(tests/e2e/probe_sensitive.rs)
//!
//! 回归:敏感字段经 `<VAR>_FILE` 引用密钥文件时,无效/不可读路径此前
//! 被静默跳过,现在必须返回错误(与 EnvSource 的 `_FILE` 语义一致);
//! 有效路径必须真实读取文件内容。

use confers::secret::SecretString;
use confers::{Config, PathValidator};
use serial_test::serial;

#[derive(Debug, Config, serde::Deserialize)]
struct SensitiveFileProbe {
    #[config(default = "localhost".to_string())]
    pub host: String,

    #[config(sensitive = true)]
    pub api_key: SecretString,
}

fn write_secret_file(body: &str) -> (tempfile::NamedTempFile, std::path::PathBuf) {
    let cwd = std::env::current_dir().unwrap();
    let file = tempfile::Builder::new()
        .prefix("t012_secret")
        .suffix(".txt")
        .tempfile_in(&cwd)
        .unwrap();
    std::fs::write(file.path(), body).unwrap();
    let path = file.path().to_path_buf();
    let rel = path.strip_prefix(&cwd).unwrap_or(&path).to_path_buf();
    (file, rel)
}

#[test]
#[serial]
fn t012_sensitive_file_valid_path_reads_content() {
    let (_file, rel) = write_secret_file("tok-123\n");
    unsafe { std::env::set_var("API_KEY_FILE", &rel) }; // pragma: allowlist secret
    let cfg = SensitiveFileProbe::load_sync().expect("valid _FILE must load");
    unsafe { std::env::remove_var("API_KEY_FILE") }; // pragma: allowlist secret
    assert_eq!(cfg.api_key.expose(), "tok-123");
    assert_eq!(cfg.host, "localhost");
    let _ = PathValidator::new(); // re-export sanity
}

#[test]
#[serial]
fn t012_sensitive_file_missing_path_is_hard_error() {
    // 指向不存在的文件:必须报错,不得静默跳过。
    unsafe { std::env::set_var("API_KEY_FILE", "t012_definitely_missing.txt") }; // pragma: allowlist secret
    let result = SensitiveFileProbe::load_sync();
    unsafe { std::env::remove_var("API_KEY_FILE") }; // pragma: allowlist secret
    assert!(result.is_err(), "invalid _FILE path must be a hard error");
}

#[test]
#[serial]
fn t012_sensitive_plain_env_still_works() {
    // 无 _FILE 变量时回退普通变量(与 EnvSource 语义一致)。
    unsafe { std::env::set_var("API_KEY", "plain-tok") }; // pragma: allowlist secret
    let cfg = SensitiveFileProbe::load_sync().expect("plain env fallback must load");
    unsafe { std::env::remove_var("API_KEY") }; // pragma: allowlist secret
    assert_eq!(cfg.api_key.expose(), "plain-tok");
    assert_eq!(cfg.host, "localhost");
}
