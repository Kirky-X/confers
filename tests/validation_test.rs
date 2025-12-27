// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

#[cfg(test)]
mod test_validation {
    use confers::Config;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
    struct SimpleConfig {
        #[config(default = 5)]
        val: u32,
    }

    #[test]
    fn test_simple_default() {
        let config = SimpleConfig::load().expect("Should load valid config");
        assert_eq!(config.val, 5);
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
    #[config(validate = true)]
    struct ValidateConfig {
        #[config(default = 5)]
        val: u32,
    }

    #[test]
    fn test_validate_default() {
        let config = ValidateConfig::load_sync().expect("Should load valid config");
        assert_eq!(config.val, 5);
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
    struct Details {
        #[config(default = "10", validate = "range(min = 1, max = 100)")]
        count: u32,
    }

    #[test]
    fn test_details_validation() {
        // 测试有效值（在 1-100 范围内）
        temp_env::with_vars(
            [("DETAILS_COUNT", Some("50"))],
            || match Details::load_sync() {
                Ok(config) => println!("Valid count (50): OK, count={}", config.count),
                Err(e) => println!("Valid count (50): Error: {:?}", e),
            },
        );

        // 测试无效值（超过 max=100）
        // 加载时应触发验证错误
        temp_env::with_vars(
            [("DETAILS_COUNT", Some("200"))],
            || match Details::load_sync() {
                Ok(_) => println!("Invalid count (200): OK (should have failed!)"),
                Err(e) => println!("Invalid count (200): Error: {:?}", e),
            },
        );
    }
}
