// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! 密钥轮转示例 - 加密密钥的安全轮转
//!
//! 本示例展示如何实现加密密钥的安全轮转：
//! - 两阶段轮转策略
//! - 旧密钥解密支持
//! - 检查点恢复机制
//! - 零停机轮转

use confers::secret::{EnvKeyProvider, SecretKeyProvider, XChaCha20Crypto, derive_field_key};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedData {
    version: String,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
    previous_version: Option<String>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    tracing::info!("Key rotation example starting");

    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { env::set_var("APP_ENCRYPTION_KEY", "12345678901234567890123456789012") };

    demonstrate_key_version_derivation();
    demonstrate_encryption_with_version();
    demonstrate_key_rotation();
    demonstrate_data_migration();
    demonstrate_env_key_provider_with_version();
    demonstrate_rollback_mechanism(); // added: rollback mechanism demo

    tracing::info!("All example demos completed");
}

fn demonstrate_key_version_derivation() {
    tracing::info!("=== Example 1: key version derivation ===");

    let master_key = [
        0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99,
    ];

    let db_password_v1 = derive_field_key(&master_key, "database.password", "v1")
        .expect("failed to derive the v1 key");
    let db_password_v2 = derive_field_key(&master_key, "database.password", "v2")
        .expect("failed to derive the v2 key");

    tracing::info!("Database password v1 key: {:02x?}", &db_password_v1[..8]);
    tracing::info!("Database password v2 key: {:02x?}", &db_password_v2[..8]);

    assert_ne!(db_password_v1, db_password_v2);
    tracing::info!("Different versions produce different keys: validation passed");

    let api_key_v1 =
        derive_field_key(&master_key, "api.key", "v1").expect("failed to derive the API key");
    assert_ne!(db_password_v1, api_key_v1);
    tracing::info!("Different fields produce different keys: validation passed");

    let db_password_v1_again = derive_field_key(&master_key, "database.password", "v1")
        .expect("failed to re-derive the v1 key");
    assert_eq!(db_password_v1, db_password_v1_again);
    tracing::info!("The same version produces the same key (deterministic): validation passed");
}

fn demonstrate_encryption_with_version() {
    tracing::info!("\n=== Example 2: encryption with versioned keys ===");

    let master_key = [
        0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99,
    ];

    let crypto = XChaCha20Crypto::new();
    let plaintext = b"my-secret-database-password";

    let key_v1 =
        derive_field_key(&master_key, "database.password", "v1").expect("failed to derive the key");
    let (nonce_v1, ciphertext_v1) = crypto
        .encrypt(plaintext, key_v1.as_slice())
        .expect("v1 encryption failed");

    let key_v2 =
        derive_field_key(&master_key, "database.password", "v2").expect("failed to derive the key");
    let (nonce_v2, ciphertext_v2) = crypto
        .encrypt(plaintext, key_v2.as_slice())
        .expect("v2 encryption failed");

    tracing::info!("v1 encryption - nonce: {:02x?}", &nonce_v1[..8]);
    tracing::info!("v1 encryption - ciphertext: {:02x?}", &ciphertext_v1[..16]);
    tracing::info!("v2 encryption - nonce: {:02x?}", &nonce_v2[..8]);
    tracing::info!("v2 encryption - ciphertext: {:02x?}", &ciphertext_v2[..16]);

    assert_ne!(ciphertext_v1, ciphertext_v2);
    tracing::info!("Different version keys produce different ciphertexts: validation passed");

    let decrypted_v1 = crypto
        .decrypt(&nonce_v1, &ciphertext_v1, key_v1.as_slice())
        .expect("v1 decryption failed");
    let decrypted_v2 = crypto
        .decrypt(&nonce_v2, &ciphertext_v2, key_v2.as_slice())
        .expect("v2 decryption failed");

    assert_eq!(decrypted_v1.as_slice(), plaintext.as_slice());
    assert_eq!(decrypted_v2.as_slice(), plaintext.as_slice());
    tracing::info!("Each version's key decrypts correctly: validation passed");

    let cross_fail = crypto.decrypt(&nonce_v1, &ciphertext_v1, key_v2.as_slice());
    assert!(cross_fail.is_err());
    tracing::info!("Cross-version keys cannot decrypt: validation passed");
}

fn demonstrate_key_rotation() {
    tracing::info!("\n=== Example 3: key rotation flow ===");

    let master_key = [
        0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99,
    ];

    let crypto = XChaCha20Crypto::new();
    let plaintext = b"important-api-key-12345";

    let old_version = "v1";
    let new_version = "v2";

    let old_key = derive_field_key(&master_key, "api.key", old_version)
        .expect("failed to derive the old key");
    let (old_nonce, old_ciphertext) = crypto
        .encrypt(plaintext, old_key.as_slice())
        .expect("old key encryption failed");

    tracing::info!("Old version: {}", old_version);
    tracing::info!("Old nonce: {:02x?}", &old_nonce[..8]);
    tracing::info!("Old ciphertext: {:02x?}", &old_ciphertext[..16]);

    let decrypted_with_old = crypto
        .decrypt(&old_nonce, &old_ciphertext, old_key.as_slice())
        .expect("old key decryption failed");
    tracing::info!(
        "Old key decryption succeeded: {:?}",
        String::from_utf8_lossy(&decrypted_with_old)
    );

    let new_key = derive_field_key(&master_key, "api.key", new_version)
        .expect("failed to derive the new key");
    let (new_nonce, new_ciphertext) = crypto
        .encrypt(plaintext, new_key.as_slice())
        .expect("new key encryption failed");

    tracing::info!("New version: {}", new_version);
    tracing::info!("New nonce: {:02x?}", &new_nonce[..8]);
    tracing::info!("New ciphertext: {:02x?}", &new_ciphertext[..16]);

    let encrypted_data = EncryptedData {
        version: new_version.to_string(),
        nonce: new_nonce.clone(),
        ciphertext: new_ciphertext.clone(),
        previous_version: Some(old_version.to_string()),
    };

    tracing::info!("Encrypted data after rotation: {:?}", encrypted_data);
    tracing::info!("Key rotation completed");
}

fn demonstrate_data_migration() {
    tracing::info!("\n=== Example 4: encrypted data migration ===");

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

    let old_key = derive_field_key(&master_key, "field.data", old_version)
        .expect("failed to derive the old key");
    let (old_nonce, old_ciphertext) = crypto
        .encrypt(original_plaintext, old_key.as_slice())
        .expect("encryption failed");

    let stored_data = StoredEncryptedData {
        version: old_version.to_string(),
        nonce: old_nonce.clone(),
        ciphertext: old_ciphertext.clone(),
    };

    tracing::info!(
        "Stored old data: version={}, nonce length={}, ciphertext length={}",
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
        let plaintext =
            crypto.decrypt(&old_data.nonce, &old_data.ciphertext, old_key.as_slice())?;

        let new_key = derive_field_key(master_key, field_name, new_version)?;
        let (new_nonce, new_ciphertext) = crypto.encrypt(&plaintext, new_key.as_slice())?;

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
    .expect("migration failed");

    tracing::info!(
        "Migrated data: version={}, nonce length={}, ciphertext length={}",
        migrated_data.version,
        migrated_data.nonce.len(),
        migrated_data.ciphertext.len()
    );

    let new_key = derive_field_key(&master_key, "field.data", new_version)
        .expect("failed to derive the new key");
    let migrated_plaintext = crypto
        .decrypt(
            &migrated_data.nonce,
            &migrated_data.ciphertext,
            new_key.as_slice(),
        )
        .expect("failed to verify decryption");

    assert_eq!(migrated_plaintext.as_slice(), original_plaintext);
    tracing::info!("Data integrity check: passed");

    let verification_old = crypto.decrypt(
        &migrated_data.nonce,
        &migrated_data.ciphertext,
        old_key.as_slice(),
    );
    assert!(verification_old.is_err());
    tracing::info!("Old keys cannot decrypt new data: validation passed");
}

fn demonstrate_env_key_provider_with_version() {
    tracing::info!("\n=== Example 5: EnvKeyProvider and key versions ===");

    let provider = EnvKeyProvider::new("APP_ENCRYPTION_KEY");

    let key_result: Result<confers::secret::SecretBytes, _> = provider.get_key();

    match key_result {
        Ok(key) => {
            tracing::info!("Got the master key successfully");
            tracing::info!("Key length: {} bytes", key.as_slice().len());

            let crypto = XChaCha20Crypto::new();
            let plaintext = b"env-key-provider-protected-data";

            let versions = ["v1", "v2", "v3"];

            for version in versions.iter() {
                let field_key = derive_field_key(key.as_slice(), "test.field", version)
                    .unwrap_or_else(|_| panic!("failed to derive the {} key", version));

                let (nonce, ciphertext) = crypto
                    .encrypt(plaintext, field_key.as_slice())
                    .unwrap_or_else(|_| panic!("{} encryption failed", version));

                tracing::info!(
                    "Version {} - nonce: {:02x?}, ciphertext: {:02x?}",
                    version,
                    &nonce[..8],
                    &ciphertext[..12]
                );

                let decrypted = crypto
                    .decrypt(&nonce, &ciphertext, field_key.as_slice())
                    .unwrap_or_else(|_| panic!("{} decryption failed", version));
                assert_eq!(decrypted.as_slice(), plaintext);
            }

            tracing::info!("EnvKeyProvider integrates with multi-version keys: validation passed");
        }
        Err(e) => {
            tracing::error!("Failed to get the key: {:?}", e);
        }
    }

    // FIXME: Audit that the environment access only happens in single-threaded code.
    unsafe { env::remove_var("APP_ENCRYPTION_KEY") };
}

/// 演示回滚机制：当密钥轮换出现问题时，可以快速回滚到旧版本
fn demonstrate_rollback_mechanism() {
    tracing::info!("\n=== Example 6: rollback mechanism ===");

    let master_key = [
        0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88,
        0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99,
    ];

    let crypto = XChaCha20Crypto::new();
    let plaintext = b"critical-data-that-needs-rollback";
    let field_name = "backup.critical";

    // Step 1: 使用 v1 加密原始数据
    let key_v1 =
        derive_field_key(&master_key, field_name, "v1").expect("failed to derive the v1 key");
    let (nonce_v1, ciphertext_v1) = crypto
        .encrypt(plaintext, key_v1.as_slice())
        .expect("v1 encryption failed");

    tracing::info!("Initial state: data encrypted with v1");
    tracing::info!("v1 nonce: {:02x?}", &nonce_v1[..8]);

    // Step 2: 执行密钥轮换 - 读取旧数据，用 v2 重新加密
    let key_v2 =
        derive_field_key(&master_key, field_name, "v2").expect("failed to derive the v2 key");

    // 用 v1 解密数据
    let decrypted_for_rotation = crypto
        .decrypt(&nonce_v1, &ciphertext_v1, key_v1.as_slice())
        .expect("failed to decrypt old data");

    // 用 v2 重新加密
    let (nonce_v2, ciphertext_v2) = crypto
        .encrypt(&decrypted_for_rotation, key_v2.as_slice())
        .expect("v2 encryption failed");

    tracing::info!("After rotation: data encrypted with v2");
    tracing::info!("v2 nonce: {:02x?}", &nonce_v2[..8]);

    // Step 3: 存储轮换后的数据，包含 previous_version 用于回滚
    let encrypted_data = EncryptedData {
        version: "v2".to_string(),
        nonce: nonce_v2.clone(),
        ciphertext: ciphertext_v2.clone(),
        previous_version: Some("v1".to_string()),
    };

    tracing::info!(
        "Stored encrypted data: version={}, previous_version={}",
        encrypted_data.version,
        encrypted_data.previous_version.as_ref().unwrap()
    );

    // Step 4: 验证当前版本密钥可以解密数据
    let current_decrypted = crypto
        .decrypt(
            &encrypted_data.nonce,
            &encrypted_data.ciphertext,
            key_v2.as_slice(),
        )
        .expect("failed to decrypt the current version");
    assert_eq!(current_decrypted.as_slice(), plaintext.as_slice());
    tracing::info!("✓ current version (v2) decrypts successfully");

    // Step 5: 使用 previous_version 进行回滚 - 解密旧版本数据
    // 回滚场景：从 v2 恢复到 v1，需要用旧密钥解密
    let rollback_version = encrypted_data.previous_version.as_ref().unwrap();
    let rollback_key = derive_field_key(&master_key, field_name, rollback_version)
        .expect("failed to derive the rollback key");

    // 注意：这里演示的是用 v1 密钥解密 v1 数据（因为 previous_version 记录了之前用的版本）
    // 实际回滚场景是：我们有历史数据，之前用 v1 加密的，现在需要解密
    let historical_decrypted = crypto
        .decrypt(&nonce_v1, &ciphertext_v1, rollback_key.as_slice())
        .expect("failed to decrypt historical data");
    assert_eq!(historical_decrypted.as_slice(), plaintext.as_slice());
    tracing::info!("✓ decrypted historical data with previous_version successfully");

    // Step 6: 演示完整回滚流程 - 清除 previous_version
    let rolled_back_data = EncryptedData {
        version: "v1".to_string(),
        nonce: nonce_v1.clone(),
        ciphertext: ciphertext_v1.clone(),
        previous_version: None,
    };

    tracing::info!("Rollback complete: data version reset to v1");
    tracing::info!(
        "Current version: {}, previous_version: {:?}",
        rolled_back_data.version,
        rolled_back_data.previous_version
    );

    // 验证回滚后的数据可以用 v1 解密
    let final_decrypted = crypto
        .decrypt(
            &rolled_back_data.nonce,
            &rolled_back_data.ciphertext,
            key_v1.as_slice(),
        )
        .expect("final decryption failed");
    assert_eq!(final_decrypted.as_slice(), plaintext.as_slice());
    tracing::info!("✓ data decryption verified after rollback");

    // Step 7: 验证安全性 - v1 密钥无法解密 v2 加密的数据
    let cross_fail = crypto.decrypt(&nonce_v2, &ciphertext_v2, key_v1.as_slice());
    assert!(cross_fail.is_err());
    tracing::info!("✓ v1 keys cannot decrypt data encrypted with v2 (security check passed)");
}
