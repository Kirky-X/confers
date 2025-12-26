use confers::Config;
use confers::ConfigMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(env_prefix = "TEST")]
struct SimpleConfig {
    #[config(default = 5)]
    val: u32,
}

#[test]
fn test_debug_cli_args() {
    println!("Testing CLI args generation...");

    let default_config = SimpleConfig::default();
    println!("Default config: {:?}", default_config);

    let no_args: Vec<String> = vec![];
    match <SimpleConfigClapShadow as confers::clap::Parser>::try_parse_from(no_args) {
        Ok(clap_shadow) => {
            println!("Parsed ClapShadow: {:?}", clap_shadow);

            let cli_args = clap_shadow.to_cli_args();
            println!("CLI args: {:?}", cli_args);
        }
        Err(e) => {
            println!("Error parsing ClapShadow: {:?}", e);
        }
    }

    let empty_args: Vec<String> = vec![];
    let result = SimpleConfig::load_from_args(empty_args);
    match result {
        Ok(config) => {
            println!("Loaded config: {:?}", config);
            assert_eq!(config.val, 5);
        }
        Err(e) => {
            println!("Error loading config: {:?}", e);
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
#[config(env_prefix = "ATTR")]
struct AttributeConfig {
    #[config(name_env = "CUSTOM_VAR_NAME")]
    #[config(default = "1234")]
    custom_field: u32,

    #[config(default = "\"default\".to_string()")]
    normal_field: String,
}

#[test]
fn debug_name_env() {
    temp_env::with_vars(
        [
            ("CUSTOM_VAR_NAME", Some("8888")),
            ("ATTR_CUSTOM_FIELD", Some("1111")),
        ],
        || {
            println!("Environment variables set:");
            println!(
                "  CUSTOM_VAR_NAME = {}",
                std::env::var("CUSTOM_VAR_NAME").unwrap()
            );
            println!(
                "  ATTR_CUSTOM_FIELD = {}",
                std::env::var("ATTR_CUSTOM_FIELD").unwrap()
            );

            let mapping = AttributeConfig::env_mapping();
            println!("Env mapping: {:?}", mapping);

            let config = AttributeConfig::load().expect("Failed to load config");
            println!("Loaded config: {:?}", config);
            println!("custom_field = {}", config.custom_field);

            assert_eq!(config.custom_field, 8888);
        },
    );
}

#[test]
fn debug_figment_test() {
    use figment::{
        providers::{Env, Format, Toml},
        Figment,
    };
    use serde::Deserialize;
    use std::fs;

    #[derive(Debug, Deserialize)]
    struct NestedConfig {
        name: String,
        #[serde(flatten)]
        details: Details,
    }

    #[derive(Debug, Deserialize)]
    struct Details {
        count: u32,
    }

    let file_path = "/tmp/config.toml";
    fs::write(
        file_path,
        r#"
            name = "file_val"
            [details]
            count = 50
        "#,
    )
    .unwrap();

    temp_env::with_vars(
        [
            ("TEST_NAME", Some("env_val")),
            ("TEST_DETAILS__COUNT", Some("75")),
        ],
        || {
            let figment = Figment::new()
                .merge(Toml::file(file_path))
                .merge(Env::prefixed("TEST_").split("__"));

            match figment.extract::<NestedConfig>() {
                Ok(config) => {
                    println!("Success: {:?}", config);
                    println!("name: {}, count: {}", config.name, config.details.count);
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                }
            }
        },
    );
}

#[test]
fn test_figment_flatten() {
    use figment::{providers::Env, Figment};
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Details {
        count: u32,
    }

    #[derive(Debug, Deserialize)]
    struct ConfigWithoutFlatten {
        name: String,
        details: Details,
    }

    #[derive(Debug, Deserialize)]
    struct ConfigWithFlatten {
        name: String,
        #[serde(flatten)]
        details: Details,
    }

    temp_env::with_vars(
        [
            ("TEST_NAME", Some("test_name")),
            ("TEST_DETAILS__COUNT", Some("75")),
        ],
        || {
            println!("=== Testing without flatten ===");
            let figment = Figment::new().merge(Env::prefixed("TEST_").split("__"));

            match figment.extract::<ConfigWithoutFlatten>() {
                Ok(config) => println!("Success: {:?}", config),
                Err(e) => println!("Error: {}", e),
            }

            println!("\n=== Testing with flatten ===");
            let figment = Figment::new().merge(Env::prefixed("TEST_").split("__"));

            match figment.extract::<ConfigWithFlatten>() {
                Ok(config) => println!("Success: {:?}", config),
                Err(e) => println!("Error: {}", e),
            }
        },
    );
}
