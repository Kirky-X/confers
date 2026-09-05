// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 编译期验收(MAC-09 / MAC-17 / ENC-23):非法宏属性必须在宏展开期报错。
//! 运行:cargo test -p confers-macros(需要 trybuild dev-dependency)

#[test]
fn ci_compile_fail() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
