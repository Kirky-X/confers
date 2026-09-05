// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! E2E: 格式解析与 Loader(tests/e2e/format_e2e.rs)
//!
//! 场景固化(docs/TEST_SCENARIOS.md §2.1):
//! - FMT-11 文件超过 `LoaderConfig::max_size` → `SizeLimitExceeded`,不读入内存(loader 级 + source 链级)
//! - FMT-19 带 UTF-8 BOM 的 TOML 可解析;带 BOM 的 JSON 按实现明确报解析错误(行为固化)
//! - FMT-20 同一内容 `load_file` 与 `parse_content` 结果等价(值 + source)
//! - FMT-16 未知扩展名 → `detect_format_from_path` 返回 None,`load_file` 报"未知格式"错误
//! - FMT-18 INI 只支持一层 section:[section] 前缀化为 `section.key`(行为固化)
//!
//! FMT-22(feature 关闭态 stub 分支)为编译矩阵场景,固化于 presets_e2e.rs(PRS 组)。

use confers::{
    ConfigError, Format, LoaderConfig, SourceId, detect_format_from_content,
    detect_format_from_path, load_file, parse_content,
};
use std::io::Write;
use std::path::Path;

/// Writes `content` into a uniquely named temp file with the given extension and
/// returns its path. The file lives in a per-test tempdir that is removed on drop.
fn write_temp_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    let mut file = std::fs::File::create(&path).expect("create temp file");
    file.write_all(content.as_bytes()).expect("write temp file");
    file.flush().expect("flush temp file");
    path
}

use std::path::PathBuf;

#[test]
fn fmt11_loader_max_size_rejects_oversized_file() {
    let dir = tempfile::tempdir().unwrap();
    let content = format!("a = {}", "1".repeat(200));
    let path = write_temp_file(dir.path(), "big.toml", &content);

    let config = LoaderConfig::new().allow_absolute().max_size(16);
    let err = load_file(&path, &config).expect_err("oversized file must be rejected");

    assert!(
        matches!(err, ConfigError::SizeLimitExceeded { actual, limit } if actual > 16 && limit == 16),
        "expected SizeLimitExceeded, got: {err:?}"
    );
    assert_eq!(err.code(), confers::ErrorCode::SizeLimitExceeded);
    let msg = err.user_message();
    assert!(
        msg.contains("16"),
        "message should mention the limit: {msg}"
    );
}

#[test]
fn fmt11_loader_max_size_allows_file_within_limit() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_file(dir.path(), "small.toml", "a = 1\n");

    let config = LoaderConfig::new().allow_absolute().max_size(4096);
    let value = load_file(&path, &config).expect("file within limit must load");
    assert_eq!(value.to_json()["a"], 1);
}

/// BLD-14(链路级):FileSource 配置的 LoaderConfig 大小限制在 source 链收集时生效。
#[test]
fn fmt11_source_chain_file_source_size_limit() {
    use confers::{FileSource, SourceChainBuilder};

    let dir = tempfile::tempdir().unwrap();
    let content = format!("a = {}", "1".repeat(200));
    let path = write_temp_file(dir.path(), "big.toml", &content);

    let loader_config = LoaderConfig::new().allow_absolute().max_size(16);
    let source = FileSource::new(&path).with_loader_config(loader_config);
    let err = SourceChainBuilder::new()
        .source(Box::new(source))
        .build()
        .collect()
        .expect_err("oversized source must be rejected");

    assert!(
        matches!(err, ConfigError::SizeLimitExceeded { .. }),
        "expected SizeLimitExceeded, got: {err:?}"
    );
}

/// FMT-19:TOML 解析器接受 UTF-8 BOM(行为固化:BOM 被剥离后正常解析)。
#[test]
fn fmt19_toml_with_utf8_bom_parses() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_file(dir.path(), "bom.toml", "\u{feff}key = \"value\"\n");

    let value = load_file(&path, &LoaderConfig::new().allow_absolute())
        .expect("TOML with BOM must parse (BOM stripped)");
    assert_eq!(value.to_json()["key"], "value");
}

/// FMT-19:JSON 解析器对 UTF-8 BOM 明确报错(行为固化:报 ParseError 而非静默成功)。
#[test]
fn fmt19_json_with_utf8_bom_reports_parse_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_file(dir.path(), "bom.json", "\u{feff}{\"key\": \"value\"}\n");

    let err = load_file(&path, &LoaderConfig::new().allow_absolute())
        .expect_err("JSON with BOM must report a parse error (fixated behavior)");
    assert!(
        matches!(err, ConfigError::ParseError { ref format, .. } if format.eq_ignore_ascii_case("json")),
        "expected ParseError(json), got: {err:?}"
    );
}

/// FMT-20:`load_file(path)` 与 `parse_content(content, detect(path))` 对同一内容等价。
#[test]
fn fmt20_load_file_and_parse_content_are_equivalent() {
    let dir = tempfile::tempdir().unwrap();
    let content = "k = \"v\"\nnum = 3\nflag = true\n";
    let path = write_temp_file(dir.path(), "same.toml", content);

    let loaded = load_file(&path, &LoaderConfig::new().allow_absolute()).unwrap();
    let parsed = parse_content(
        content,
        Format::Toml,
        SourceId::new("same.toml"),
        Some(&path),
    )
    .unwrap();

    assert_eq!(loaded.to_json(), parsed.to_json(), "values must be equal");
    assert_eq!(
        loaded.source.as_str(),
        parsed.source.as_str(),
        "source ids must be equal"
    );
    assert_eq!(
        loaded.all_paths().len(),
        parsed.all_paths().len(),
        "leaf path sets must have equal size"
    );
}

/// FMT-16:未知扩展名 → 路径嗅探返回 None,load_file 报"未知格式"解析错误。
#[test]
fn fmt16_unknown_extension_reports_unknown_format() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_file(dir.path(), "data.xyz", "gibberish");

    assert_eq!(
        detect_format_from_path(&path),
        None,
        "unknown extension must not be detected"
    );
    assert_eq!(detect_format_from_content("gibberish"), None);

    let err = load_file(&path, &LoaderConfig::new().allow_absolute())
        .expect_err("unknown extension must fail loudly");
    assert!(
        matches!(err, ConfigError::ParseError { ref format, .. } if format == "unknown"),
        "expected ParseError(unknown), got: {err:?}"
    );
}

/// FMT-18:INI 只有单层 section 语义,`[a]` 下的键前缀化为 `a.b`。
#[test]
fn fmt18_ini_section_keys_are_flat_prefixed() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_file(
        dir.path(),
        "flat.ini",
        "[server]\nhost = localhost\nport = 8080\n",
    );

    let value = load_file(&path, &LoaderConfig::new().allow_absolute()).unwrap();
    let json = value.to_json();
    assert_eq!(json["server.host"], "localhost");
    // INI 值为字符串语义,不做数值推断。
    assert_eq!(json["server.port"], "8080");
    // 单层语义:不存在嵌套 map,键被展平到顶层。
    assert!(json.get("server").is_none(), "INI must not nest maps");
}
