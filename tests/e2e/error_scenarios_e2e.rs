// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! E2E: 异常路径场景(第 1 轮,25 场景中的 23 条)
//!
//! 覆盖(docs/TEST_SCENARIOS.md §2.1-2.4 对应的异常场景):
//! - FMT-07/08/09  TOML/JSON/YAML 语法错误的错误形态与位置信息
//! - FMT-11/12/13/14/15/16  大小上限 / 文件缺失 / 路径穿越 / 符号链接 / 绝对路径 / 未知格式
//! - BLD-14/15/16/17/18/19/20/22/24  构建期限制与错误(大小 / 嵌套 / 键数 / 数组 / 字符串 /
//!   必填字段 / 类型不匹配 / fallback / 必选文件缺失)
//! - VAL-02/04/06/07  校验错误(garde Report → ConfigError、规则串解析、多字段聚合)
//! - IPL-03/04  插值未定义变量与循环引用
//!
//! 错误码说明:`ConfigError::code()` 走 `ErrorCode`(1-999,error.rs);
//! `ConfigErrorCode`(2001-2999,config_error.rs)仅由 `ConfigConfigError` 携带,
//! loader/builder/validator/interpolation 路径均不产生该类型 —— 各场景断言
//! 记录实测错误码,规格中的 2xxx 码以「码族偏差」形式在输出中呈报。

use confers::loader::{PathTraversalError, normalize_and_validate_path};
use confers::types::{AnnotatedValue, ConfigValue};
use confers::{
    ConfigBuilder, ConfigError, ConfigErrorCode, ConfigLimits, ConfigResult, ErrorCode, FileSource,
    Format, LoaderConfig, SourceChainBuilder, SourceId, detect_format_from_content,
    detect_format_from_path, load_file,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

// garde 的 `validate()` 需要该 trait 在作用域内(派生只生成 impl)。
use garde::Validate as _;

fn write_temp_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    let mut file = std::fs::File::create(&path).expect("create temp file");
    file.write_all(content.as_bytes()).expect("write temp file");
    file.flush().expect("flush temp file");
    path
}

// ============== FMT-07 / 场景 7:TOML 语法错误报行列 ==============

#[test]
fn s07_toml_syntax_error_carries_line_column() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_file(dir.path(), "bad.toml", "key =\n");

    let err = load_file(&path, &LoaderConfig::new().allow_absolute())
        .expect_err("broken TOML must fail to parse");

    let (line, column) = match &err {
        ConfigError::ParseError {
            format,
            location: Some(loc),
            ..
        } => {
            assert_eq!(format, "TOML", "format must name TOML");
            assert!(
                loc.line >= 1 && loc.column >= 1,
                "1-based location required"
            );
            assert!(
                loc.line <= 2,
                "error must sit on the offending line, got line {}",
                loc.line
            );
            (loc.line, loc.column)
        }
        other => panic!("expected ParseError with ParseLocation, got: {other:?}"),
    };

    let msg = err.user_message();
    assert!(
        msg.contains("bad.toml") && msg.contains(&format!("{line}:{column}")),
        "user_message must render file:line:column, got: {msg}"
    );
    // 实测错误码族:ErrorCode::FileParseError(3);2300 属 ConfigErrorCode::ConfigParseError,
    // 该常量存在(2300/CONFIG_PARSE_ERROR)但 load_file 不产生 ConfigConfigError。
    assert_eq!(err.code(), ErrorCode::FileParseError);
    assert_eq!(ConfigErrorCode::ConfigParseError as u16, 2300);
    println!(
        "s07 evidence: user_message={msg:?}; code={:?}({}); ParseLocation line={line} column={column}",
        err.code(),
        err.code() as u16
    );
}

// ============== FMT-08 / 场景 8:JSON 语法错误含位置 ==============

#[test]
fn s08_json_syntax_error_includes_position() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_temp_file(dir.path(), "trunc.json", "{\"host\": \"h\", \"port\":");

    let err = load_file(&path, &LoaderConfig::new().allow_absolute())
        .expect_err("truncated JSON must fail to parse");

    match &err {
        ConfigError::ParseError {
            format,
            message,
            location,
            ..
        } => {
            assert_eq!(format, "JSON");
            // serde_json 把行号/列号渲染进错误消息(位置信息载体)。
            assert!(
                message.contains("line") && message.contains("column"),
                "JSON error message must carry line/column, got: {message}"
            );
            // 行为固化:JSON 的结构化 location 字段为 None(与 TOML/YAML 不同),
            // 位置信息经由消息传递 —— 在此显式记录。
            assert!(
                location.is_none(),
                "fixated: JSON location is carried by message"
            );
        }
        other => panic!("expected ParseError(json), got: {other:?}"),
    }
    let msg = err.user_message();
    assert!(
        msg.contains("line"),
        "user_message must keep position: {msg}"
    );
    println!(
        "s08 evidence: user_message={msg:?}; code={:?}({})",
        err.code(),
        err.code() as u16
    );
}

// ============== FMT-09 / 场景 9:YAML 语法错误 ==============

#[test]
fn s09_yaml_syntax_error_reports_parse_error() {
    let dir = tempfile::tempdir().unwrap();
    // tab 缩进是 YAML 语法错误(libyaml 扫描器拒绝)。
    let path = write_temp_file(dir.path(), "tabs.yaml", "server:\n\thost: localhost\n");

    let err = load_file(&path, &LoaderConfig::new().allow_absolute())
        .expect_err("tab-indented YAML must fail to parse");

    match &err {
        ConfigError::ParseError {
            format, message, ..
        } => {
            assert_eq!(format, "YAML", "format must name YAML, got: {message}");
            assert!(!message.is_empty(), "YAML error must carry a message");
        }
        other => panic!("expected ParseError(yaml), got: {other:?}"),
    }
    println!(
        "s09 evidence: user_message={:?}; code={:?}",
        err.user_message(),
        err.code()
    );
}

// ============== FMT-11 / 场景 11:文件超 max_size 拒绝 ==============

#[test]
fn s11_oversized_file_rejected_before_parse() {
    let dir = tempfile::tempdir().unwrap();
    // 故意写入「超限且非法 TOML」的内容:若实现先读入再解析,将得到 ParseError;
    // 得到 SizeLimitExceeded 即证明大小门禁先于读取/解析生效。
    let content = format!("a = {}", "x".repeat(4096));
    let path = write_temp_file(dir.path(), "big.toml", &content);

    let config = LoaderConfig::new().allow_absolute().max_size(16);
    let err = load_file(&path, &config).expect_err("oversized file must be rejected");

    match &err {
        ConfigError::SizeLimitExceeded { actual, limit } => {
            assert_eq!(*limit, 16);
            assert!(*actual > 16, "actual={} must exceed limit", actual);
        }
        other => panic!("expected SizeLimitExceeded, got: {other:?}"),
    }
    assert_eq!(err.code(), ErrorCode::SizeLimitExceeded);
    let msg = err.user_message();
    assert!(msg.contains("16"), "message must state the limit: {msg}");
    // 不读入内存的证据:实现先按已打开句柄的 metadata 拒绝(src/impl_/loader.rs:625),
    // 读取发生在其后(loader.rs:646-650),本例坏内容从未进入解析器。
    println!(
        "s11 evidence: user_message={msg:?}; code={:?}({})",
        err.code(),
        err.code() as u16
    );
}

// ============== FMT-12 / 场景 12:文件不存在报 FileNotFound ==============

#[test]
fn s12_missing_file_reports_not_found() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("no_such_config.toml");

    // 入口 1a:load_file(不存在的绝对路径, allow_absolute) → FileNotFound,
    // source 携带底层 io 错误(NotFound)。
    let direct = load_file(&missing, &LoaderConfig::new().allow_absolute())
        .expect_err("missing file must be reported as FileNotFound");
    match &direct {
        ConfigError::FileNotFound { filename, source } => {
            assert!(
                filename.ends_with("no_such_config.toml"),
                "error must name the file, got {:?}",
                filename
            );
            let io_err = source.as_ref().expect("FileNotFound must carry a source");
            assert_eq!(io_err.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected FileNotFound with source, got: {other:?}"),
    }
    assert_eq!(direct.code(), ErrorCode::FileNotFound);
    assert!(direct.to_string().contains("not found"));
    println!(
        "s12 evidence(abs path): display={:?}; code={:?}({}); 2200 常量存在={} 但属 ConfigErrorCode 族,loader 链路实测码 1",
        direct.to_string(),
        direct.code(),
        direct.code() as u16,
        ConfigErrorCode::ConfigFileNotFound as u16 == 2200
    );

    // 入口 1b:相对路径(symlink 检查分支)同样报 FileNotFound —— 两条校验
    // 分支的缺失文件语义一致。tempdir 建在 cwd 下以构成合法相对路径。
    let cwd = std::env::current_dir().unwrap();
    let rel_dir = tempfile::tempdir_in(&cwd).unwrap();
    let rel_missing = rel_dir.path().join("absent_rel.toml");
    let rel = rel_missing
        .strip_prefix(&cwd)
        .expect("tempdir_in(cwd) yields a relative path")
        .to_path_buf();
    let rel_err = load_file(&rel, &LoaderConfig::new())
        .expect_err("missing relative path must also be FileNotFound");
    match &rel_err {
        ConfigError::FileNotFound { source, .. } => {
            let io_err = source
                .as_ref()
                .expect("relative branch also carries source");
            assert_eq!(io_err.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected FileNotFound (relative branch), got: {other:?}"),
    }

    // 对照:真正的路径穿越仍报 InvalidValue(安全语义不变)。
    let traversal = load_file(
        Path::new("config/../../etc/passwd.toml"),
        &LoaderConfig::new(),
    );
    assert!(
        matches!(&traversal, Err(ConfigError::InvalidValue { .. })),
        "traversal must stay InvalidValue, got: {traversal:?}"
    );

    // 入口 2:builder 的 FileSource —— 缺失文件 → ConfigError::FileNotFound
    // (source 为 None:该入口在 exists() 预检即失败,不经过 open)。
    let err = SourceChainBuilder::new()
        .source(Box::new(FileSource::new(&missing)))
        .build()
        .collect()
        .expect_err("missing required file must fail the chain");

    match &err {
        ConfigError::FileNotFound { filename, source } => {
            assert!(
                filename.ends_with("no_such_config.toml"),
                "error must name the file, got {:?}",
                filename
            );
            assert!(
                source.is_none(),
                "FileSource pre-check carries no io source"
            );
        }
        other => panic!("expected FileNotFound, got: {other:?}"),
    }
    assert_eq!(err.code(), ErrorCode::FileNotFound);
    println!("s12 evidence(FileSource): display={:?}", err.to_string());
}

// ============== FMT-13 / 场景 13:路径穿越拒绝 ==============

#[test]
fn s13_path_traversal_rejected() {
    // 直接断言 normalize_and_validate_path 拒绝。
    let verdict = normalize_and_validate_path(
        Path::new("config/../../etc/passwd.toml"),
        &[PathBuf::from(".")],
        false,
        true,
    );
    match verdict {
        Err(PathTraversalError::ParentDirectoryReference) => {}
        other => panic!("expected ParentDirectoryReference, got: {other:?}"),
    }

    // 经 load_file 的用户路径同样被拒(包装为 InvalidValue 并说明原因)。
    let err = load_file(
        Path::new("config/../../etc/passwd.toml"),
        &LoaderConfig::new(),
    )
    .expect_err("traversal path must be rejected");
    match &err {
        ConfigError::InvalidValue { message, .. } => {
            assert!(
                message.contains("Parent directory references"),
                "rejection must explain the violation, got: {message}"
            );
            println!("s13 evidence: InvalidValue message={message:?}");
        }
        other => panic!("expected InvalidValue(path validation), got: {other:?}"),
    }
}

// ============== FMT-14 / 场景 14:符号链接逃逸拒绝 ==============

#[cfg(unix)]
#[test]
fn s14_symlink_escape_rejected_then_allowed_without_check() {
    let outside = tempfile::tempdir().unwrap();
    write_temp_file(outside.path(), "outside.toml", "a = 1\n");
    // 符号链接检查只作用于相对路径分支(绝对路径走 allow_absolute 分支,不查目录归属),
    // 因此 allowed 目录建在 cwd 之下,以相对路径引用链接。
    let cwd = std::env::current_dir().unwrap();
    let allowed = tempfile::tempdir_in(&cwd).unwrap();
    let link = allowed.path().join("link.toml");
    std::os::unix::fs::symlink(outside.path().join("outside.toml"), &link).expect("create symlink");
    let relative_link = link.strip_prefix(&cwd).unwrap().to_path_buf();

    // 默认:check_symlinks=true → 解析出真实路径在 allowed 目录之外 → 拒绝。
    let denied = load_file(
        &relative_link,
        &LoaderConfig::new().allowed_dirs(vec![allowed.path()]),
    )
    .expect_err("symlink escaping the allowed dir must be denied by default");
    match &denied {
        ConfigError::InvalidValue { message, .. } => assert!(
            message.contains("Symlink resolves outside"),
            "denial must explain symlink traversal, got: {message}"
        ),
        other => panic!("expected InvalidValue(symlink traversal), got: {other:?}"),
    }
    println!("s14 evidence(denied): {:?}", denied.to_string());

    // no_symlink_check() 后放行:内容经由符号链接读取成功。
    let value = load_file(
        &relative_link,
        &LoaderConfig::new()
            .allowed_dirs(vec![allowed.path()])
            .no_symlink_check(),
    )
    .expect("no_symlink_check must allow the link");
    assert_eq!(value.to_json()["a"], 1, "linked content must load");
    println!("s14 evidence(allowed): value={:?}", value.to_json());
}

// ============== FMT-15 / 场景 15:绝对路径默认拒绝 ==============

#[test]
fn s15_absolute_path_denied_by_default_allowed_option() {
    let dir = tempfile::tempdir().unwrap();
    let abs = write_temp_file(dir.path(), "abs.toml", "k = \"v\"\n");

    // 默认拒绝。
    let denied =
        load_file(&abs, &LoaderConfig::new()).expect_err("absolute path must be denied by default");
    match &denied {
        ConfigError::InvalidValue { message, .. } => assert!(
            message.contains("Absolute paths are not allowed"),
            "got: {message}"
        ),
        other => panic!("expected InvalidValue(absolute path), got: {other:?}"),
    }

    // LoaderConfig::allow_absolute() 放行。
    let value = load_file(&abs, &LoaderConfig::new().allow_absolute())
        .expect("allow_absolute must permit the absolute path");
    assert_eq!(value.to_json()["k"], "v");

    // ConfigBuilder::allow_absolute_paths() 后 .file(绝对路径) 构建成功。
    let built: serde_json::Value = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&abs)
        .build()
        .expect("builder must honor allow_absolute_paths");
    assert_eq!(built["k"], "v");
    println!("s15 evidence: default denied; allow_absolute ok; builder ok: {built}");
}

// ============== FMT-16 / 场景 16:未知格式报错 ==============

#[test]
fn s16_unknown_format_reports_error() {
    let dir = tempfile::tempdir().unwrap();
    // 无扩展名 + 内容不可嗅探(无 {、---、key =、key:、[section] 等特征)。
    let path = write_temp_file(dir.path(), "blobfile", "\u{0}\u{1}gibberish\u{7}\n");

    assert_eq!(
        detect_format_from_path(&path),
        None,
        "no-extension path must not resolve to a format"
    );
    assert_eq!(
        detect_format_from_content("\u{0}\u{1}gibberish\u{7}\n"),
        None
    );
    assert!(
        Format::try_parse("gibberish").is_none(),
        "unknown name must not parse"
    );

    let err = load_file(&path, &LoaderConfig::new().allow_absolute())
        .expect_err("unknown format must fail loudly");
    match &err {
        ConfigError::ParseError {
            format, message, ..
        } => {
            assert_eq!(format, "unknown", "format must be reported as unknown");
            assert!(message.contains("Unknown"), "got: {message}");
        }
        other => panic!("expected ParseError(unknown), got: {other:?}"),
    }
    println!("s16 evidence: {:?}", err.to_string());
}

// ============== BLD-14 / 场景 36:ConfigLimits::max_file_size_bytes 超限 ==============

#[test]
fn s36_limits_max_file_size_bytes_rejected_at_build() {
    let dir = tempfile::tempdir().unwrap();
    let content = format!("name = \"{}\"\nport = 1\n", "x".repeat(64 * 1024));
    let big = write_temp_file(dir.path(), "big.toml", &content);
    let file_len = std::fs::metadata(&big).unwrap().len();

    let limits = ConfigLimits::default().with_max_file_size_bytes(16);
    assert!(
        !limits.is_file_size_ok(file_len),
        "precondition: is_file_size_ok must flag the oversized file"
    );

    // build 在读取任何源之前拒绝超限文件 → SizeLimitExceeded(actual/limit)。
    let err: ConfigError = ConfigBuilder::<serde_json::Value>::new()
        .allow_absolute_paths()
        .file(&big)
        .limits(limits)
        .build()
        .expect_err("oversized file must be rejected at build");
    match &err {
        ConfigError::SizeLimitExceeded { actual, limit } => {
            assert_eq!(*limit, 16, "limit must come from ConfigLimits");
            assert_eq!(*actual as u64, file_len, "actual must be the file size");
        }
        other => panic!("expected SizeLimitExceeded, got: {other:?}"),
    }
    assert_eq!(err.code(), ErrorCode::SizeLimitExceeded);
    let msg = err.to_string();
    assert!(msg.contains("16"), "message must state the limit: {msg}");
    // 码族说明:2400(CONFIG_SIZE_LIMIT_EXCEEDED)属 ConfigErrorCode 族,
    // build 链路实测码为 ErrorCode::SizeLimitExceeded(500)。
    assert_eq!(ConfigErrorCode::ConfigSizeLimitExceeded as u16, 2400);
    println!(
        "s36 evidence(rejected): file_len={file_len}; display={msg:?}; code={}({})",
        err.code(),
        err.code() as u16
    );

    // 对照:文件在限制内 → 正常构建,值完整。
    let small = write_temp_file(dir.path(), "small.toml", "port = 2\n");
    let ok: serde_json::Value = ConfigBuilder::<serde_json::Value>::new()
        .allow_absolute_paths()
        .file(&small)
        .limits(ConfigLimits::default().with_max_file_size_bytes(16))
        .build()
        .expect("file within limit must build");
    assert_eq!(ok["port"], 2);
    println!("s36 evidence(within limit): {ok}");
}

// ============== BLD-15/16/17/18 / 场景 37-40:结构限制 ==============

fn annotated(value: ConfigValue) -> AnnotatedValue {
    AnnotatedValue::new(value, SourceId::new("mem"), "")
}

fn nested(depth: usize) -> ConfigValue {
    let mut v = ConfigValue::string("leaf");
    for _ in 0..depth {
        v = ConfigValue::map(vec![("n", annotated(v))]);
    }
    v
}

fn build_memory(
    values: HashMap<String, ConfigValue>,
    limits: ConfigLimits,
) -> ConfigResult<serde_json::Value> {
    ConfigBuilder::new().memory(values).limits(limits).build()
}

#[test]
fn s37_nesting_depth_exceeded_clear_error() {
    let mut values = HashMap::new();
    values.insert("a".into(), nested(6));
    let err = build_memory(values, ConfigLimits::default().with_max_nesting_depth(3))
        .expect_err("depth beyond max_nesting_depth must be rejected");

    match &err {
        ConfigError::InvalidValue { message, .. } => {
            assert!(
                message.contains("nesting depth") && message.contains("exceeds configured limit 3"),
                "got: {message}"
            );
        }
        other => panic!("expected InvalidValue(nesting depth), got: {other:?}"),
    }
    println!("s37 evidence: {:?}", err.to_string());
}

#[test]
fn s38_total_fields_exceeded() {
    let mut values = HashMap::new();
    for k in ["k1", "k2", "k3"] {
        values.insert(k.into(), ConfigValue::integer(1));
    }
    let err = build_memory(values, ConfigLimits::default().with_max_total_fields(2))
        .expect_err("key count beyond max_total_fields must be rejected");

    match &err {
        ConfigError::InvalidValue { message, .. } => assert!(
            message.contains("total field count") && message.contains("exceeds configured limit 2"),
            "got: {message}"
        ),
        other => panic!("expected InvalidValue(total field count), got: {other:?}"),
    }
    println!("s38 evidence: {:?}", err.to_string());
}

#[test]
fn s39_array_length_exceeded() {
    let mut values = HashMap::new();
    values.insert(
        "items".into(),
        ConfigValue::array(vec![
            annotated(ConfigValue::integer(1)),
            annotated(ConfigValue::integer(2)),
        ]),
    );
    let err = build_memory(values, ConfigLimits::default().with_max_array_length(1))
        .expect_err("array beyond max_array_length must be rejected");

    match &err {
        ConfigError::InvalidValue { message, .. } => assert!(
            message.contains("array length") && message.contains("exceeds configured limit 1"),
            "got: {message}"
        ),
        other => panic!("expected InvalidValue(array length), got: {other:?}"),
    }
    println!("s39 evidence: {:?}", err.to_string());
}

#[test]
fn s40_string_length_exceeded() {
    let mut values = HashMap::new();
    values.insert(
        "s".into(),
        ConfigValue::string("this-string-is-far-too-long"),
    );
    let err = build_memory(values, ConfigLimits::default().with_max_string_length(4))
        .expect_err("string beyond max_string_length must be rejected");

    match &err {
        ConfigError::InvalidValue { message, .. } => assert!(
            message.contains("string length") && message.contains("exceeds configured limit 4"),
            "got: {message}"
        ),
        other => panic!("expected InvalidValue(string length), got: {other:?}"),
    }
    println!("s40 evidence: {:?}", err.to_string());
}

// ============== BLD-19 / 场景 41:必填字段缺失 ==============

#[expect(dead_code)] // 反序列化载体：字段由 serde 填充，测试只断言错误路径
#[derive(Debug, Default, Deserialize)]
struct RequiredCfg41 {
    host: String,
    port: u16,
}

#[test]
fn s41_missing_required_field_reports_error() {
    let dir = tempfile::tempdir().unwrap();
    // port 无 default、非 Option:源中缺失 → 构建必须明确报错(非 panic)。
    let file = write_temp_file(dir.path(), "partial.toml", "host = \"h\"\n");

    let result: Result<RequiredCfg41, ConfigError> = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&file)
        .build();

    let err = result.expect_err("missing required field must fail the build");
    match &err {
        ConfigError::InvalidValue { key, message, .. } => {
            // serde 的 missing field 错误经 serde_path_to_error 保留字段路径。
            assert!(
                message.contains("missing field") && message.contains("port"),
                "error must name the missing field, got: {message}"
            );
            let _ = key;
        }
        other => panic!("expected InvalidValue(missing field), got: {other:?}"),
    }
    // 规格标注 MissingField(2001):该常量存在(ConfigErrorCode::MissingField),
    // 但 build 链路报 serde missing-field(实测码 102/INVALID_VALUE);2001 无生产方。
    assert_eq!(ConfigErrorCode::MissingField as u16, 2001);
    println!(
        "s41 evidence: {:?}; code={}({})",
        err,
        err.code() as u16,
        err.code()
    );
}

// ============== BLD-20 / 场景 42:类型不匹配 ==============

#[expect(dead_code)] // 反序列化载体：字段由 serde 填充，测试只断言错误路径
#[derive(Debug, Default, Deserialize)]
struct PortCfg42 {
    port: u16,
}

#[expect(dead_code)] // 反序列化载体：字段由 serde 填充，测试只断言错误路径
#[derive(Debug, Default, Deserialize)]
struct NameCfg42 {
    name: String,
}

#[test]
fn s42_type_mismatch_errors_not_panic() {
    let dir = tempfile::tempdir().unwrap();

    // string "abc" → u16:类型错误。
    let bad = write_temp_file(dir.path(), "str_port.toml", "port = \"abc\"\n");
    let result: Result<PortCfg42, ConfigError> = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&bad)
        .build();
    let err = result.expect_err("string port must fail typed build");
    match &err {
        ConfigError::InvalidValue { key, message, .. } => {
            assert!(
                key.contains("port"),
                "field path must be tracked, got key={key:?}"
            );
            assert!(message.contains("invalid type"), "got: {message}");
        }
        other => panic!("expected InvalidValue(type mismatch), got: {other:?}"),
    }

    // 反向边界:integer 123 → String 字段:同样明确报错。
    let rev = write_temp_file(dir.path(), "int_name.toml", "name = 123\n");
    let result2: Result<NameCfg42, ConfigError> = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&rev)
        .build();
    let err2 = result2.expect_err("integer into String must fail typed build");
    match &err2 {
        ConfigError::InvalidValue { key, message, .. } => {
            assert!(key.contains("name"), "got key={key:?}");
            assert!(message.contains("invalid type"), "got: {message}");
        }
        other => panic!("expected InvalidValue(int→string), got: {other:?}"),
    }
    println!("s42 evidence: str→u16 {err:?}; int→String {err2:?}");
}

// ============== BLD-22 / 场景 44:全源失败回退 fallback ==============

#[derive(Debug, Default, Deserialize, Clone, PartialEq)]
struct FallbackCfg {
    host: String,
    port: u16,
}

#[test]
fn s44_build_with_fallback_when_all_sources_fail() {
    let fallback = FallbackCfg {
        host: "fallback-host".into(),
        port: 9,
    };

    let result = ConfigBuilder::new()
        .allow_absolute_paths()
        .file("/nonexistent/a.toml")
        .file("/nonexistent/b.toml")
        .build_with_fallback(fallback.clone());

    // 回退默认实例。
    assert_eq!(
        result.config, fallback,
        "fallback instance must be returned"
    );
    assert!(result.degraded, "build must be flagged degraded");
    assert!(
        result
            .degraded_reason
            .as_deref()
            .unwrap_or_default()
            .contains("not found"),
        "degraded reason must explain the failure, got: {:?}",
        result.degraded_reason
    );
    // 标记 warning。
    assert!(result.has_warnings(), "fallback must carry a warning");
    let w = &result.warnings[0];
    assert_eq!(w.code, confers::error::WarningCode::RemoteFallback);
    assert!(
        w.message.contains("fallback"),
        "warning message must mention the fallback, got: {:?}",
        w.message
    );
    println!(
        "s44 evidence: config={:?}; degraded_reason={:?}; warning={:?}",
        result.config, result.degraded_reason, w.message
    );
}

// ============== BLD-24 / 场景 46:必选文件缺失报错 ==============

#[test]
fn s46_required_file_missing_reports_file_not_found() {
    let missing = Path::new("definitely_absent_46.toml");
    let result: Result<FallbackCfg, ConfigError> = ConfigBuilder::new().file(missing).build();
    let err = result.expect_err("required missing file must fail the build");

    match &err {
        ConfigError::FileNotFound { filename, .. } => {
            assert!(filename.ends_with("definitely_absent_46.toml"));
        }
        other => panic!("expected FileNotFound, got: {other:?}"),
    }
    assert_eq!(err.code(), ErrorCode::FileNotFound);
    assert!(err.to_string().contains("not found"));
    println!(
        "s46 evidence: display={:?}; code={}({}); 2200 属 ConfigErrorCode 族(build 链路实测码 1)",
        err.to_string(),
        err.code() as u16,
        err.code()
    );
}

// ============== VAL-02 / 场景 52:非法值报 ValidationFailed ==============

#[derive(Debug, Default, Deserialize, garde::Validate)]
struct ContactCfg {
    #[garde(email)]
    contact: String,
    #[garde(range(min = 1, max = 10000))]
    port: u16,
}

#[test]
fn s52_invalid_values_fail_validation_with_field_path() {
    let dir = tempfile::tempdir().unwrap();
    // 类型均合法(可构建),值非法:email 格式错 + range 越界。
    let file = write_temp_file(
        dir.path(),
        "bad_values.toml",
        "contact = \"not-an-email\"\nport = 65535\n",
    );

    // 行为固化:#[config(validate)] 不自动挂进加载管线 —— build 成功,
    // 校验由使用方显式触发(见 tests/e2e/validation_e2e.rs 头注)。
    let cfg: ContactCfg = ConfigBuilder::new()
        .allow_absolute_paths()
        .file(&file)
        .build()
        .expect("type-valid values must build; validation is an explicit step");

    let report = cfg.validate().expect_err("both fields violate their rules");
    let mut saw_contact = false;
    let mut saw_port = false;
    for (path, _) in report.iter() {
        let p = path.to_string();
        saw_contact |= p.contains("contact");
        saw_port |= p.contains("port");
    }
    assert!(
        saw_contact && saw_port,
        "both offending fields must be referenced"
    );

    // 报告转换为 ConfigError 后保留字段路径。
    let err = ConfigError::validation_error("validation failed", report);
    match &err {
        ConfigError::ValidationFailed { field, .. } => {
            assert!(
                field.contains("contact") || field.contains("port"),
                "field path must survive conversion, got: {field:?}"
            );
        }
        other => panic!("expected ValidationFailed, got: {other:?}"),
    }
    assert_eq!(err.code(), ErrorCode::ValidationFailed);
    println!(
        "s52 evidence: {:?}; code={}({}); 2500 属 ConfigErrorCode 族",
        err,
        err.code() as u16,
        err.code()
    );
}

// ============== VAL-04 / 场景 54:非法规则字符串 ==============

#[test]
fn s54_invalid_rule_string_is_rejected_not_silently_accepted() {
    use confers::validator::ValidationRule;

    // 非法规则串一律 None(显式拒绝),绝不产出规则或静默吞成默认规则。
    assert!(ValidationRule::from_str("unknown_rule").is_none());
    assert!(ValidationRule::from_str("length").is_none());
    assert!(ValidationRule::from_str("length(min=abc)").is_none());
    assert!(ValidationRule::from_str("length(max=xyz)").is_none());
    assert!(
        ValidationRule::from_str("length(min=5, max=10").is_none(),
        "未闭合括号必须拒绝"
    );
    assert!(
        ValidationRule::from_str("length(min=10, max=1)").is_none(),
        "min>max 必须拒绝"
    );
    assert!(ValidationRule::from_str("range(min=abc)").is_none());
    assert!(ValidationRule::from_str("range(min=100, max=1)").is_none());
    assert!(ValidationRule::from_str("").is_none());

    // 对照:合法规则仍可解析。
    assert!(matches!(
        ValidationRule::from_str("length(min=1, max=10)"),
        Some(ValidationRule::Length { min: 1, max: 10 })
    ));
    println!("s54 evidence: all invalid rule strings rejected via None; valid rule parses");
}

// ============== VAL-06 / 场景 56:garde 报告转 ConfigError ==============

#[test]
fn s56_garde_report_conversion_keeps_field_and_readable_message() {
    let cfg = ContactCfg {
        contact: "nope".into(),
        port: 80,
    };
    let report = cfg.validate().expect_err("email violation expected");

    let err = ConfigError::validation_error("validation failed", report);
    match &err {
        ConfigError::ValidationFailed {
            field,
            rule,
            message,
        } => {
            assert!(
                field.contains("contact"),
                "field info must survive, got: {field:?}"
            );
            assert_eq!(rule, "garde", "rule must identify the validator");
            assert!(!message.is_empty(), "message must carry garde's reason");
        }
        other => panic!("expected ValidationFailed, got: {other:?}"),
    }

    let msg = err.user_message();
    assert!(!msg.is_empty(), "user_message must be readable");
    assert!(
        msg.contains("contact"),
        "user_message must name the field: {msg}"
    );
    assert!(
        msg.contains("failed validation"),
        "readable wording expected: {msg}"
    );
    println!("s56 evidence: user_message={msg:?}");
}

// ============== VAL-07 / 场景 57:多字段违规聚合 ==============

#[test]
fn s57_multiple_violations_aggregate_into_one_report() {
    let cfg = ContactCfg {
        contact: "bad".into(),
        port: 65535,
    };
    let report = cfg.validate().expect_err("two violations expected");

    let mut count = 0;
    let mut saw_contact = false;
    let mut saw_port = false;
    for (path, _) in report.iter() {
        count += 1;
        let p = path.to_string();
        saw_contact |= p.contains("contact");
        saw_port |= p.contains("port");
    }
    assert!(
        saw_contact && saw_port,
        "both fields must appear in the one report"
    );
    assert!(count >= 2, "violations must aggregate (got {count})");

    // 行为固化:garde Report 聚合全部字段;ConfigError::validation_error
    // 只保留第一条(error.rs「Extract the first error」)—— 显式记录该边界。
    let err = ConfigError::validation_error("validation failed", report);
    assert!(matches!(err, ConfigError::ValidationFailed { .. }));
    println!(
        "s57 evidence: report aggregated {count} violations (contact+port); converted error keeps first: {:?}",
        err
    );
}

// ============== IPL-03 / 场景 61:未定义变量报错 ==============

#[test]
fn s61_undefined_variable_is_an_error() {
    let err = confers::interpolation::interpolate("value=${TOTALLY_MISSING}", &|_| None)
        .expect_err("undefined variable without default must fail");

    match &err {
        ConfigError::InterpolationError { variable, message } => {
            assert_eq!(variable, "TOTALLY_MISSING");
            assert!(
                message.contains("not found"),
                "message must explain, got: {message}"
            );
        }
        other => panic!("expected InterpolationError, got: {other:?}"),
    }
    assert_eq!(err.code(), ErrorCode::InterpolationError);
    // 对照:带默认值时不报错(IPL-02 语义)。
    let ok = confers::interpolation::interpolate("value=${TOTALLY_MISSING:8080}", &|_| None)
        .expect("default value must satisfy");
    assert_eq!(ok, "value=8080");
    println!(
        "s61 evidence: variable=TOTALLY_MISSING code={}({}); 2800 属 ConfigErrorCode 族",
        err.code() as u16,
        err.code()
    );
}

// ============== IPL-04 / 场景 62:循环引用检测 ==============

#[test]
fn s62_circular_reference_detected() {
    let err = confers::interpolation::interpolate("${A}", &|name| match name {
        "A" => Some("${B}".to_string()),
        "B" => Some("${A}".to_string()),
        _ => None,
    })
    .expect_err("A→B→A must be detected as circular");

    match &err {
        ConfigError::CircularReference { path } => {
            assert!(
                !path.is_empty(),
                "path must identify the cycle node, got: {path:?}"
            );
        }
        other => panic!("expected CircularReference, got: {other:?}"),
    }
    assert_eq!(err.code(), ErrorCode::CircularReference);
    println!(
        "s62 evidence: {:?}; code={}({}); 2900 属 ConfigErrorCode 族",
        err,
        err.code() as u16,
        err.code()
    );
}
