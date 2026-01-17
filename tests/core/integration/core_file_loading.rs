// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 集成测试：核心配置文件加载
//!
//! 测试从文件加载配置和环境变量覆盖功能

use confers::Config;
use serde::{Deserialize, Serialize};

#[test]
fn test_validate_attribute() {
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
    #[config(env_prefix = "APP")]
    struct ValidateConfig {
        #[config(default = 5)]
        val: u32,
    }

    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("test.toml");
    std::fs::write(&file_path, "val = 5").unwrap();

    let config = ValidateConfig::load_file(&file_path)
        .load_sync()
        .expect("Should load valid config");
    assert_eq!(config.val, 5);

    temp_env::with_vars([("APP_VAL", Some("15"))], || {
        let config = ValidateConfig::load().expect("Should load with env override");
        assert_eq!(config.val, 15);
    });
}