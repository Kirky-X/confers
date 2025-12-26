#[cfg(test)]
mod test_nested_validation {
    use confers::Config;
    use validator::Validate;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
    struct Details {
        #[config(default = "10", validate = "range(max = 100)")]
        count: u32,
    }

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
    #[config(validate)]
    struct NestedConfig {
        #[config(default = "default_val".to_string())]
        name: String,

        #[serde(flatten)]
        details: Details,
    }

    #[test]
    fn test_nested_validation_direct() {
        // Test with valid nested values
        let config = NestedConfig {
            name: "test".to_string(),
            details: Details { count: 50 },
        };
        match config.validate() {
            Ok(_) => println!("Valid nested config: OK"),
            Err(e) => println!("Valid nested config: Error: {:?}", e),
        }

        // Test with invalid nested values
        let config = NestedConfig {
            name: "test".to_string(),
            details: Details { count: 200 },
        };
        match config.validate() {
            Ok(_) => println!("Invalid nested config: OK (should have failed!)"),
            Err(e) => println!("Invalid nested config: Error: {:?}", e),
        }
    }
}
