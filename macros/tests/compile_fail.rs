// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 编译期验收(ENC-23):非法宏属性必须在宏展开期报错。
//! 运行:cargo test -p confers-macros(需要 trybuild dev-dependency)

#[test]
fn ci_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
