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

    // 从环境变量获取主密钥（实际应用中应从安全的密钥管理系统获取）
    let master_key_hex = std::env::var("CONFERS_MASTER_KEY").unwrap_or_else(|_| {
        // 示例用途：使用默认密钥（仅用于演示，生产环境必须使用安全密钥）
        "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20".to_string()
    });

    let master_key = hex::decode(master_key_hex)
        .map_err(|_| anyhow::anyhow!("无效的主密钥格式，应为 64 字符的十六进制字符串"))?;
    if master_key.len() != 32 {
        return Err(anyhow::anyhow!("主密钥长度必须为 32 字节"));
    }
    let master_key: [u8; 32] = master_key.try_into().unwrap();
    println!("✓ 主密钥已加载（从环境变量 CONFERS_MASTER_KEY）");

    // 初始化密钥环
    println!("\n--- 初始化密钥环 ---");
    let version = key_manager.initialize(
        &master_key,
        "production".to_string(),
        "security-team".to_string(),
    )?;
    println!("✓ 密钥环已初始化，版本: {:?}", version);

    // 生成加密密钥
    println!("\n--- 生成加密密钥 ---");
    let encryption_key = key_manager.generate_key()?;
    println!("✓ 加密密钥已生成: {:?}", &encryption_key[..8]);

    // 轮换密钥
    println!("\n--- 轮换密钥 ---");
    let rotation_result = key_manager.rotate_key(
        &master_key,
        Some("production".to_string()),
        "security-team".to_string(),
        Some("Scheduled rotation".to_string()),
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
        Some("Application encryption key ring".to_string()),
    )?;
    println!("✓ 密钥环已创建:");
    println!("  - 密钥 ID: {}", key_ring.id);
    println!("  - 版本: {:?}", key_ring.version);

    println!("\n=== 密钥管理示例完成 ===");
    println!("\n安全提示:");
    println!("⚠️  主密钥应从环境变量 CONFERS_MASTER_KEY 或安全的密钥管理系统获取");
    println!("⚠️  不要将密钥硬编码在代码中或提交到版本控制系统");
    println!("⚠️  生产环境必须使用真实的密钥管理系统（如 AWS KMS、HashiCorp Vault）");
    println!("⚠️  定期轮换密钥（建议每 90 天）");
    println!("⚠️  为不同环境使用不同的密钥");
    println!("⚠️  确保密钥存储位置有适当的访问控制");

    Ok(())
}
