// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in project root for full license information.

//! 单元测试：密钥管理
//!
//! 测试配置加密密钥的生成、管理、存储和加载功能

#[cfg(test)]
#[cfg(feature = "encryption")]
mod key_tests {
    use super::super::*;
    use std::fs;
    use tempfile::TempDir;

    /// 测试密钥生成
    #[test]
    fn test_key_generation() {
        let key = crate::key::generate_key();

        assert!(!key.is_empty());
        assert!(key.len() >= 32); // 最小密钥长度
    }

    /// 测试密钥格式验证
    #[test]
    fn test_key_format_validation() {
        let valid_key = crate::key::generate_key();

        // 验证密钥格式
        assert!(valid_key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }

    /// 测试密钥强度检查
    #[test]
    fn test_key_strength() {
        let weak_key = "123456"; // 弱密钥
        let strong_key = crate::key::generate_key(); // 强密钥

        // 验证弱密钥检测
        assert!(crate::key::is_weak_key(&weak_key));
        assert!(!crate::key::is_weak_key(&strong_key));
    }

    /// 测试密钥存储
    #[test]
    fn test_key_storage() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let key_file = temp_dir.path().join("test_key");
        let test_key = "test_secret_key_12345";

        // 测试密钥保存
        crate::key::save_key(&key_file, &test_key)?;

        // 验证文件存在
        assert!(key_file.exists());

        // 验证文件内容
        let loaded_key = std::fs::read_to_string(&key_file)?;
        assert_eq!(loaded_key, test_key);

        Ok(())
    }

    /// 测试密钥加载
    #[test]
    fn test_key_loading() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let key_file = temp_dir.path().join("test_key");
        let test_key = "test_secret_key_12345";

        // 保存测试密钥
        crate::key::save_key(&key_file, &test_key)?;

        // 加载密钥
        let loaded_key = crate::key::load_key(&key_file)?;

        assert_eq!(loaded_key, test_key);

        Ok(())
    }

    /// 测试密钥文件不存在
    #[test]
    fn test_key_file_not_found() {
        let non_existent_file = std::path::PathBuf::from("/non/existent/key");

        let result = crate::key::load_key(&non_existent_file);

        assert!(result.is_err());
    }

    /// 测试密钥权限检查
    #[test]
    fn test_key_file_permissions() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let key_file = temp_dir.path().join("restricted_key");
        let test_key = "test_secret_key";

        // 保存密钥
        crate::key::save_key(&key_file, &test_key)?;

        // 检查文件权限（在支持的系统上）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&key_file)?;
            let permissions = metadata.permissions();

            // 验证文件权限（应该为600，仅所有者可读写）
            assert_eq!(permissions.mode() & 0o777, 0o600);
        }

        Ok(())
    }

    /// 测试密钥环境变量加载
    #[test]
    fn test_key_env_loading() -> anyhow::Result<()> {
        let test_key = "env_secret_key_12345";

        // 设置环境变量
        std::env::set_var("CONFERS_KEY", &test_key);

        // 从环境变量加载
        let loaded_key = crate::key::load_key_from_env()?;

        assert_eq!(loaded_key, test_key);

        // 清理环境变量
        std::env::remove_var("CONFERS_KEY");

        Ok(())
    }

    /// 测试密钥环境变量不存在
    #[test]
    fn test_key_env_not_found() {
        // 确保环境变量不存在
        std::env::remove_var("CONFERS_KEY");

        let result = crate::key::load_key_from_env();

        assert!(result.is_err());
    }

    /// 测试密钥验证功能
    #[test]
    fn test_key_validation() {
        let valid_key = "AbCdEf123456789012345678901234";
        let invalid_key = "short";

        // 验证有效密钥
        assert!(crate::key::validate_key(&valid_key).is_ok());

        // 验证无效密钥
        assert!(crate::key::validate_key(&invalid_key).is_err());
    }

    /// 测试密钥派生功能
    #[test]
    fn test_key_derivation() {
        let base_key = "base_key_12345";
        let salt = "test_salt";

        // 测试密钥派生
        let derived_key1 = crate::key::derive_key(&base_key, &salt);
        let derived_key2 = crate::key::derive_key(&base_key, &salt);

        // 相同输入应该产生相同输出
        assert_eq!(derived_key1, derived_key2);

        // 派生密钥应该不同于基础密钥
        assert_ne!(derived_key1, base_key);
    }

    /// 测试密钥安全擦除
    #[test]
    fn test_key_zeroization() {
        let mut key_bytes = vec![0x41, 0x42, 0x43, 0x44]; // "ABCD"

        // 安全擦除密钥
        crate::key::secure_zeroize(&mut key_bytes);

        // 验证内存被清零
        assert!(key_bytes.iter().all(|&b| b == 0));
    }

    /// 测试密钥缓存功能
    #[test]
    fn test_key_caching() -> anyhow::Result<()> {
        let temp_dir = TempDir::new()?;
        let key_file = temp_dir.path().join("cached_key");
        let test_key = "cached_secret_key_12345";

        // 保存密钥
        crate::key::save_key(&key_file, &test_key)?;

        // 第一次加载（从文件）
        let loaded_key1 = crate::key::load_key_cached(&key_file)?;
        assert_eq!(loaded_key1, test_key);

        // 第二次加载（从缓存）
        let loaded_key2 = crate::key::load_key_cached(&key_file)?;
        assert_eq!(loaded_key2, test_key);

        Ok(())
    }

    /// 测试密钥过期检查
    #[test]
    fn test_key_expiry() -> anyhow::Result<()> {
        let test_key = "expiring_key_12345";
        let expiry_hours = 1;

        // 创建有过期时间的密钥存储
        let key_data = crate::key::create_key_with_expiry(&test_key, expiry_hours)?;

        // 验证过期时间设置
        assert!(key_data.contains("expiry"));

        Ok(())
    }

    /// 测试密钥轮换
    #[test]
    fn test_key_rotation() -> anyhow::Result<()> {
        let old_key = "old_key_12345";
        let new_key = "new_key_67890";

        // 测试密钥轮换
        let rotation_result = crate::key::rotate_key(&old_key, &new_key)?;

        // 验证轮换成功
        assert!(rotation_result.contains("rotated"));

        Ok(())
    }
}
