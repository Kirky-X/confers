// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! E2E: `#[config(default)]` 边界写法(tests/e2e/default_attrs_e2e.rs)
//!
//! 回归:裸字形式 `#[config(default)]`(取字段类型 Default)与
//! `#[config(default = None)]`(Option 字段显式 None)此前产生难懂的
//! 编译错误,现在必须生成合法代码并真实生效。

use confers::Config;

#[derive(Debug, Config, serde::Deserialize)]
struct BareDefault {
    #[config(default)]
    pub host: String,
}

#[derive(Debug, Config, serde::Deserialize)]
struct NoneDefault {
    #[config(default = None)]
    pub host: Option<String>,
}

#[test]
fn bare_word_default_uses_type_default() {
    let cfg = BareDefault::load_sync().expect("bare-word default must compile and load");
    assert_eq!(cfg.host, String::default());
}

#[test]
fn none_default_yields_none_for_option_field() {
    let cfg = NoneDefault::load_sync().expect("default = None must compile and load");
    assert_eq!(cfg.host, None);
}
