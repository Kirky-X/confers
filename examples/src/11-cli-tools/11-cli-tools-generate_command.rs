// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! CLI 生成命令示例
//!
//! 展示如何使用 confers CLI 的生成命令。
//!
//! ## 前提条件
//!
//! - 需要启用的特性：`cli`, `derive`
//! - 可选特性：`schema` (用于增强功能)
//!
//! ## 运行方式
//!
//! ```bash
//! cargo run --example 11-cli-tools-generate_command --features "cli,derive"
//! ```
//!
//! ## 或者直接使用 CLI
//!
//! ```bash
//! cargo install confers --features "cli,derive"
//! confers generate --output config.toml
//! ```

fn main() {
    println!("Run: confers generate --struct \"AppConfig\" --output config.toml");
}
