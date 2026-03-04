use confers::secret::{derive_field_key, EnvKeyProvider, SecretKeyProvider, XChaCha20Crypto};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedData {
    version: String,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
    previous_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct KeyVersion {
    status: String,
    created_at: String,
    deprecated_at: Option<String>,
    expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
struct KeyManagement {
    current_version: String,
    master_key_id: String,
    versions: std::collections::HashMap<String, KeyVersion>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("密钥轮换示例程序启动");

    env::set_var("APP_ENCRYPTION_KEY", "12345678901234567890123456789012");

    demonstrate_key_version_derivation();
    demonstrate_encryption_with_version();
    demonstrate_key_rotation();
    demonstrate_data_migration();
    demonstrate_env_key_provider_with_version();
    demonstrate_rollback_mechanism(); // 新增：回滚机制演示

    tracing::info!("所有示例演示完成");
}

fn demonstrate_key_version_derivation() {
    tracing::info!("=== 示例 1: 密钥版本派生 ===");

    let master_key = [
        0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99,
    ];

    let db_password_v1 =
        derive_field_key(&master_key, "database.password", "v1").expect("派生 v1 密钥失败");
    let db_password_v2 =
        derive_field_key(&master_key, "database.password", "v2").expect("派生 v2 密钥失败");

    tracing::info!("数据库密码 v1 密钥: {:02x?}", &db_password_v1[..8]);
    tracing::info!("数据库密码 v2 密钥: {:02x?}", &db_password_v2[..8]);

    assert_ne!(db_password_v1, db_password_v2);
    tracing::info!("不同版本产生不同密钥: 验证通过");

    let api_key_v1 = derive_field_key(&master_key, "api.key", "v1").expect("派生 API 密钥失败");
    assert_ne!(db_password_v1, api_key_v1);
    tracing::info!("不同字段产生不同密钥: 验证通过");

    let db_password_v1_again =
        derive_field_key(&master_key, "database.password", "v1").expect("再次派生 v1 密钥失败");
    assert_eq!(db_password_v1, db_password_v1_again);
    tracing::info!("相同版本产生相同密钥（确定性）: 验证通过");
}

fn demonstrate_encryption_with_version() {
    tracing::info!("\n=== 示例 2: 使用版本化密钥加密 ===");

    let master_key = [
        0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99,
    ];

    let crypto = XChaCha20Crypto::new();
    let plaintext = b"my-secret-database-password";

    let key_v1 = derive_field_key(&master_key, "database.password", "v1").expect("派生密钥失败");
    let (nonce_v1, ciphertext_v1) = crypto.encrypt(plaintext, &key_v1).expect("v1 加密失败");

    let key_v2 = derive_field_key(&master_key, "database.password", "v2").expect("派生密钥失败");
    let (nonce_v2, ciphertext_v2) = crypto.encrypt(plaintext, &key_v2).expect("v2 加密失败");

    tracing::info!("v1 加密 - Nonce: {:02x?}", &nonce_v1[..8]);
    tracing::info!("v1 加密 - 密文: {:02x?}", &ciphertext_v1[..16]);
    tracing::info!("v2 加密 - Nonce: {:02x?}", &nonce_v2[..8]);
    tracing::info!("v2 加密 - 密文: {:02x?}", &ciphertext_v2[..16]);

    assert_ne!(ciphertext_v1, ciphertext_v2);
    tracing::info!("不同版本密钥产生不同密文: 验证通过");

    let decrypted_v1 = crypto
        .decrypt(&nonce_v1, &ciphertext_v1, &key_v1)
        .expect("v1 解密失败");
    let decrypted_v2 = crypto
        .decrypt(&nonce_v2, &ciphertext_v2, &key_v2)
        .expect("v2 解密失败");

    assert_eq!(decrypted_v1, plaintext.as_slice());
    assert_eq!(decrypted_v2, plaintext.as_slice());
    tracing::info!("各自版本密钥正确解密: 验证通过");

    let cross_fail = crypto.decrypt(&nonce_v1, &ciphertext_v1, &key_v2);
    assert!(cross_fail.is_err());
    tracing::info!("跨版本密钥无法解密: 验证通过");
}

fn demonstrate_key_rotation() {
    tracing::info!("\n=== 示例 3: 密钥轮换流程 ===");

    let master_key = [
        0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99,
    ];

    let crypto = XChaCha20Crypto::new();
    let plaintext = b"important-api-key-12345";

    let old_version = "v1";
    let new_version = "v2";

    let old_key = derive_field_key(&master_key, "api.key", old_version).expect("派生旧密钥失败");
    let (old_nonce, old_ciphertext) = crypto.encrypt(plaintext, &old_key).expect("旧密钥加密失败");

    tracing::info!("旧版本: {}", old_version);
    tracing::info!("旧 Nonce: {:02x?}", &old_nonce[..8]);
    tracing::info!("旧密文: {:02x?}", &old_ciphertext[..16]);

    let decrypted_with_old = crypto
        .decrypt(&old_nonce, &old_ciphertext, &old_key)
        .expect("旧密钥解密失败");
    tracing::info!(
        "旧密钥解密成功: {:?}",
        String::from_utf8_lossy(&decrypted_with_old)
    );

    let new_key = derive_field_key(&master_key, "api.key", new_version).expect("派生新密钥失败");
    let (new_nonce, new_ciphertext) = crypto.encrypt(plaintext, &new_key).expect("新密钥加密失败");

    tracing::info!("新版本: {}", new_version);
    tracing::info!("新 Nonce: {:02x?}", &new_nonce[..8]);
    tracing::info!("新密文: {:02x?}", &new_ciphertext[..16]);

    let encrypted_data = EncryptedData {
        version: new_version.to_string(),
        nonce: new_nonce.clone(),
        ciphertext: new_ciphertext.clone(),
        previous_version: Some(old_version.to_string()),
    };

    tracing::info!("轮换后的加密数据: {:?}", encrypted_data);
    tracing::info!("密钥轮换完成");
}

fn demonstrate_data_migration() {
    tracing::info!("\n=== 示例 4: 加密数据迁移 ===");

    let master_key = [
        0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99,
    ];

    let crypto = XChaCha20Crypto::new();

    struct StoredEncryptedData {
        version: String,
        nonce: Vec<u8>,
        ciphertext: Vec<u8>,
    }

    let original_plaintext = b"migrating-sensitive-data";
    let old_version = "v1";
    let new_version = "v2";

    let old_key = derive_field_key(&master_key, "field.data", old_version).expect("派生旧密钥失败");
    let (old_nonce, old_ciphertext) = crypto
        .encrypt(original_plaintext, &old_key)
        .expect("加密失败");

    let stored_data = StoredEncryptedData {
        version: old_version.to_string(),
        nonce: old_nonce.clone(),
        ciphertext: old_ciphertext.clone(),
    };

    tracing::info!(
        "存储的旧数据: version={}, nonce 长度={}, ciphertext 长度={}",
        stored_data.version,
        stored_data.nonce.len(),
        stored_data.ciphertext.len()
    );

    fn migrate_encrypted_data(
        crypto: &XChaCha20Crypto,
        master_key: &[u8; 32],
        field_name: &str,
        old_data: &StoredEncryptedData,
        old_version: &str,
        new_version: &str,
    ) -> Result<StoredEncryptedData, Box<dyn std::error::Error>> {
        let old_key = derive_field_key(master_key, field_name, old_version)?;
        let plaintext = crypto.decrypt(&old_data.nonce, &old_data.ciphertext, &old_key)?;

        let new_key = derive_field_key(master_key, field_name, new_version)?;
        let (new_nonce, new_ciphertext) = crypto.encrypt(&plaintext, &new_key)?;

        Ok(StoredEncryptedData {
            version: new_version.to_string(),
            nonce: new_nonce,
            ciphertext: new_ciphertext,
        })
    }

    let migrated_data = migrate_encrypted_data(
        &crypto,
        &master_key,
        "field.data",
        &stored_data,
        old_version,
        new_version,
    )
    .expect("迁移失败");

    tracing::info!(
        "迁移后的数据: version={}, nonce 长度={}, ciphertext 长度={}",
        migrated_data.version,
        migrated_data.nonce.len(),
        migrated_data.ciphertext.len()
    );

    let new_key = derive_field_key(&master_key, "field.data", new_version).expect("派生新密钥失败");
    let migrated_plaintext = crypto
        .decrypt(&migrated_data.nonce, &migrated_data.ciphertext, &new_key)
        .expect("验证解密失败");

    assert_eq!(migrated_plaintext, original_plaintext);
    tracing::info!("数据完整性验证: 通过");

    let verification_old =
        crypto.decrypt(&migrated_data.nonce, &migrated_data.ciphertext, &old_key);
    assert!(verification_old.is_err());
    tracing::info!("旧密钥无法解密新数据: 验证通过");
}

fn demonstrate_env_key_provider_with_version() {
    tracing::info!("\n=== 示例 5: EnvKeyProvider 与密钥版本 ===");

    let provider = EnvKeyProvider::new("APP_ENCRYPTION_KEY");

    let key_result: Result<confers::secret::SecretBytes, _> = provider.get_key();

    match key_result {
        Ok(key) => {
            tracing::info!("获取主密钥成功");
            tracing::info!("密钥长度: {} 字节", key.as_slice().len());

            let crypto = XChaCha20Crypto::new();
            let plaintext = b"env-key-provider-protected-data";

            let versions = ["v1", "v2", "v3"];

            for version in versions.iter() {
                let field_key =
                    derive_field_key(key.as_slice(), "test.field", version)
                        .unwrap_or_else(|_| panic!("派生 {} 密钥失败", version));

                let (nonce, ciphertext) = crypto
                    .encrypt(plaintext, &field_key)
                    .unwrap_or_else(|_| panic!("{} 加密失败", version));

                tracing::info!(
                    "版本 {} - Nonce: {:02x?}, Ciphertext: {:02x?}",
                    version,
                    &nonce[..8],
                    &ciphertext[..12]
                );

                let decrypted = crypto
                    .decrypt(&nonce, &ciphertext, &field_key)
                    .unwrap_or_else(|_| panic!("{} 解密失败", version));
                assert_eq!(decrypted, plaintext);
            }

            tracing::info!("EnvKeyProvider 与多版本密钥集成: 验证通过");
        }
        Err(e) => {
            tracing::error!("获取密钥失败: {:?}", e);
        }
    }

    env::remove_var("APP_ENCRYPTION_KEY");
}

/// 演示回滚机制：当密钥轮换出现问题时，可以快速回滚到旧版本
fn demonstrate_rollback_mechanism() {
    tracing::info!("\n=== 示例 6: 回滚机制 ===");

    let master_key = [
        0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99,
    ];

    let crypto = XChaCha20Crypto::new();
    let plaintext = b"critical-data-that-needs-rollback";
    let field_name = "backup.critical";

    // Step 1: 使用 v1 加密原始数据
    let key_v1 = derive_field_key(&master_key, field_name, "v1").expect("派生 v1 密钥失败");
    let (nonce_v1, ciphertext_v1) = crypto.encrypt(plaintext, &key_v1).expect("v1 加密失败");

    tracing::info!("初始状态: 数据使用 v1 加密");
    tracing::info!("v1 nonce: {:02x?}", &nonce_v1[..8]);

    // Step 2: 执行密钥轮换 - 读取旧数据，用 v2 重新加密
    let key_v2 = derive_field_key(&master_key, field_name, "v2").expect("派生 v2 密钥失败");

    // 用 v1 解密数据
    let decrypted_for_rotation = crypto
        .decrypt(&nonce_v1, &ciphertext_v1, &key_v1)
        .expect("解密旧数据失败");

    // 用 v2 重新加密
    let (nonce_v2, ciphertext_v2) = crypto
        .encrypt(&decrypted_for_rotation, &key_v2)
        .expect("v2 加密失败");

    tracing::info!("轮换后: 数据使用 v2 加密");
    tracing::info!("v2 nonce: {:02x?}", &nonce_v2[..8]);

    // Step 3: 存储轮换后的数据，包含 previous_version 用于回滚
    let encrypted_data = EncryptedData {
        version: "v2".to_string(),
        nonce: nonce_v2.clone(),
        ciphertext: ciphertext_v2.clone(),
        previous_version: Some("v1".to_string()),
    };

    tracing::info!(
        "存储的加密数据: version={}, previous_version={}",
        encrypted_data.version,
        encrypted_data.previous_version.as_ref().unwrap()
    );

    // Step 4: 验证当前版本密钥可以解密数据
    let current_decrypted = crypto
        .decrypt(&encrypted_data.nonce, &encrypted_data.ciphertext, &key_v2)
        .expect("当前版本解密失败");
    assert_eq!(current_decrypted, plaintext.as_slice());
    tracing::info!("✓ 当前版本 (v2) 解密成功");

    // Step 5: 使用 previous_version 进行回滚 - 解密旧版本数据
    // 回滚场景：从 v2 恢复到 v1，需要用旧密钥解密
    let rollback_version = encrypted_data.previous_version.as_ref().unwrap();
    let rollback_key =
        derive_field_key(&master_key, field_name, rollback_version).expect("派生回滚密钥失败");

    // 注意：这里演示的是用 v1 密钥解密 v1 数据（因为 previous_version 记录了之前用的版本）
    // 实际回滚场景是：我们有历史数据，之前用 v1 加密的，现在需要解密
    let historical_decrypted = crypto
        .decrypt(&nonce_v1, &ciphertext_v1, &rollback_key)
        .expect("历史数据解密失败");
    assert_eq!(historical_decrypted, plaintext.as_slice());
    tracing::info!("✓ 使用 previous_version 解密历史数据成功");

    // Step 6: 演示完整回滚流程 - 清除 previous_version
    let rolled_back_data = EncryptedData {
        version: "v1".to_string(),
        nonce: nonce_v1.clone(),
        ciphertext: ciphertext_v1.clone(),
        previous_version: None,
    };

    tracing::info!("回滚完成: 数据版本重置为 v1");
    tracing::info!(
        "当前版本: {}, previous_version: {:?}",
        rolled_back_data.version,
        rolled_back_data.previous_version
    );

    // 验证回滚后的数据可以用 v1 解密
    let final_decrypted = crypto
        .decrypt(
            &rolled_back_data.nonce,
            &rolled_back_data.ciphertext,
            &key_v1,
        )
        .expect("最终解密失败");
    assert_eq!(final_decrypted, plaintext.as_slice());
    tracing::info!("✓ 回滚后数据解密验证通过");

    // Step 7: 验证安全性 - v1 密钥无法解密 v2 加密的数据
    let cross_fail = crypto.decrypt(&nonce_v2, &ciphertext_v2, &key_v1);
    assert!(cross_fail.is_err());
    tracing::info!("✓ v1 密钥无法解密 v2 加密的数据（安全性验证通过）");
}
