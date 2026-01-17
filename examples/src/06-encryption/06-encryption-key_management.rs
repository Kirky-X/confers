// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    // 注意：密钥管理功能需要 encryption 特性
    // 示例代码（需要启用 encryption 特性）：
    // let mut km = KeyManager::new(PathBuf::from("src/06-encryption/configs/keys"))?;
    // println!("Key Manager created");

    println!("Key management example requires the 'encryption' feature.");
    println!("Run with: cargo run --bin 06-encryption-key_management --features encryption");

    Ok(())
}
