// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use confers::key::KeyManager;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    println!("=== 密钥管理示例 ===\n");

    // 创建密钥存储目录
    let keys_dir = PathBuf::from("src/06-encryption/configs/keys");
    std::fs::create_dir_all(&keys_dir)?;

    // 创建密钥管理器
    let mut key_manager = KeyManager::new(keys_dir.clone())?;
    println!("✓ 密钥管理器已创建");

    // 生成主密钥（实际应用中应从安全位置获取）
    let master_key = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10,
        0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18,
        0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f, 0x20,
    ];
    println!("✓ 主密钥已生成（示例用途）");

    // 初始化密钥环
    println!("\n--- 初始化密钥环 ---");
    let version = key_manager.initialize(
        &master_key,
        "production".to_string(),
        "security-team".to_string()
    )?;
    println!("✓ 密钥环已初始化，版本: {}", version);

    // 生成加密密钥
    println!("\n--- 生成加密密钥 ---");
    let encryption_key = key_manager.generate_key(&master_key)?;
    println!("✓ 加密密钥已生成: {:?}", &encryption_key[..8]);

    // 轮换密钥
    println!("\n--- 轮换密钥 ---");
    let rotation_result = key_manager.rotate_key(
        &master_key,
        Some("production".to_string()),
        "security-team".to_string(),
        Some("Scheduled rotation".to_string())
    )?;
    println!("✓ 密钥已轮换:");
    println!("  - 旧版本: {}", rotation_result.previous_version);
    println!("  - 新版本: {}", rotation_result.new_version);

    // 创建密钥环
    println!("\n--- 创建密钥环 ---");
    let key_ring = key_manager.create_key_ring(
        &master_key,
        "app-encryption".to_string(),
        "application-team".to_string(),
        Some("Application encryption key ring".to_string())
    )?;
    println!("✓ 密钥环已创建:");
    println!("  - 密钥 ID: {}", key_ring.key_id);
    println!("  - 版本: {}", key_ring.version);

    println!("\n=== 密钥管理示例完成 ===");
    println!("\n安全提示:");
    println!("⚠️  主密钥应存储在安全的密钥管理系统中（如 AWS KMS、HashiCorp Vault）");
    println!("⚠️  不要将密钥提交到版本控制系统");
    println!("⚠️  定期轮换密钥（建议每 90 天）");
    println!("⚠️  为不同环境使用不同的密钥");

    Ok(())
}
