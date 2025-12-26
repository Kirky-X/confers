#[cfg(feature = "remote")]
use confers::providers::remote::http::HttpProvider;

#[cfg(feature = "remote")]
#[test]
fn test_http_provider_creation() {
    let _provider = HttpProvider::new("http://example.com/config.json");
}

#[cfg(feature = "remote")]
#[test]
fn test_http_provider_with_auth() {
    let _provider = HttpProvider::new("http://example.com/config.json").with_auth("user", "pass");
}

#[cfg(feature = "remote")]
#[test]
fn test_http_provider_with_bearer_token() {
    let _provider =
        HttpProvider::new("http://example.com/config.json").with_bearer_token("token123");
}

#[cfg(feature = "remote")]
#[test]
fn test_http_url_parsing() {
    use confers::core::{ConfigLoader, OptionalValidate};

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
    struct TestConfig {
        name: String,
        value: i32,
    }

    impl OptionalValidate for TestConfig {}

    impl confers::ConfigMap for TestConfig {
        fn to_map(&self) -> std::collections::HashMap<String, figment::value::Value> {
            let mut map = std::collections::HashMap::new();
            map.insert(
                "name".to_string(),
                figment::value::Value::String(figment::value::Tag::Default, self.name.clone()),
            );
            map.insert(
                "value".to_string(),
                figment::value::Value::Num(
                    figment::value::Tag::Default,
                    figment::value::Num::I32(self.value),
                ),
            );
            map
        }

        fn env_mapping() -> std::collections::HashMap<String, String> {
            let mut map = std::collections::HashMap::new();
            map.insert("name".to_string(), "TEST_CONFIG_NAME".to_string());
            map.insert("value".to_string(), "TEST_CONFIG_VALUE".to_string());
            map
        }
    }

    let _loader =
        ConfigLoader::<TestConfig>::new().with_remote_config("http://example.com/config.json");
}

#[cfg(feature = "remote")]
#[test]
fn test_https_url_parsing() {
    use confers::core::{ConfigLoader, OptionalValidate};

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
    struct HttpsTestConfig {
        name: String,
        value: i32,
    }

    impl OptionalValidate for HttpsTestConfig {}

    impl confers::ConfigMap for HttpsTestConfig {
        fn to_map(&self) -> std::collections::HashMap<String, figment::value::Value> {
            let mut map = std::collections::HashMap::new();
            map.insert(
                "name".to_string(),
                figment::value::Value::String(figment::value::Tag::Default, self.name.clone()),
            );
            map.insert(
                "value".to_string(),
                figment::value::Value::Num(
                    figment::value::Tag::Default,
                    figment::value::Num::I32(self.value),
                ),
            );
            map
        }

        fn env_mapping() -> std::collections::HashMap<String, String> {
            let mut map = std::collections::HashMap::new();
            map.insert("name".to_string(), "HTTPS_TEST_CONFIG_NAME".to_string());
            map.insert("value".to_string(), "HTTPS_TEST_CONFIG_VALUE".to_string());
            map
        }
    }

    let _loader = ConfigLoader::<HttpsTestConfig>::new()
        .with_remote_config("https://example.com/config.json");
}

#[cfg(feature = "remote")]
#[test]
fn test_http_integration_with_auth() {
    use confers::core::{ConfigLoader, OptionalValidate};

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
    struct AuthTestConfig {
        name: String,
    }

    impl OptionalValidate for AuthTestConfig {}

    impl confers::ConfigMap for AuthTestConfig {
        fn to_map(&self) -> std::collections::HashMap<String, figment::value::Value> {
            let mut map = std::collections::HashMap::new();
            map.insert(
                "name".to_string(),
                figment::value::Value::String(figment::value::Tag::Default, self.name.clone()),
            );
            map
        }

        fn env_mapping() -> std::collections::HashMap<String, String> {
            std::collections::HashMap::new()
        }
    }

    let _loader = ConfigLoader::<AuthTestConfig>::new()
        .with_remote_config("http://example.com/config.json")
        .with_remote_auth("user", "pass");
}
