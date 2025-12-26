use std::env;
use confers::{Config, ConfigLoader};

#[derive(Config, Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WindowsPathConfig {
    #[config(default = "\"default_value\".to_string()")]
    config_path: String,
    
    #[config(default = "\"/tmp\".to_string()")]
    temp_dir: String,
    
    #[config(default = "\"./logs\".to_string()")]
    log_dir: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[tokio::test]
    async fn test_windows_path_normalization() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = "C:\\Users\\test\\config"
        temp_dir = "C:\\Temp"
        log_dir = ".\\logs"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert_eq!(config.config_path, "C:\\Users\\test\\config");
        assert_eq!(config.temp_dir, "C:\\Temp");
        assert_eq!(config.log_dir, ".\\logs");
    }

    #[tokio::test]
    async fn test_mixed_path_separators() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = "C:/Users/test/config"
        temp_dir = "C:\\Temp\\cache"
        log_dir = "./logs"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert_eq!(config.config_path, "C:/Users/test/config");
        assert_eq!(config.temp_dir, "C:\\Temp\\cache");
        assert_eq!(config.log_dir, "./logs");
    }

    #[tokio::test]
    async fn test_unc_paths() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = "\\\\server\\share\\config"
        temp_dir = "\\\\server\\share\\temp"
        log_dir = "\\\\server\\share\\logs"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert_eq!(config.config_path, "\\\\server\\share\\config");
        assert_eq!(config.temp_dir, "\\\\server\\share\\temp");
        assert_eq!(config.log_dir, "\\\\server\\share\\logs");
    }

    #[tokio::test]
    async fn test_relative_paths() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = "..\\config"
        temp_dir = ".\\temp"
        log_dir = "logs\\app"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert_eq!(config.config_path, "..\\config");
        assert_eq!(config.temp_dir, ".\\temp");
        assert_eq!(config.log_dir, "logs\\app");
    }

    #[tokio::test]
    async fn test_path_with_spaces() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = "C:\\Program Files\\My App\\config"
        temp_dir = "C:\\Users\\Test User\\temp"
        log_dir = ".\\My Logs"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert_eq!(config.config_path, "C:\\Program Files\\My App\\config");
        assert_eq!(config.temp_dir, "C:\\Users\\Test User\\temp");
        assert_eq!(config.log_dir, ".\\My Logs");
    }

    #[tokio::test]
    async fn test_empty_path_handling() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = ""
        temp_dir = ""
        log_dir = ""
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert_eq!(config.config_path, "");
        assert_eq!(config.temp_dir, "");
        assert_eq!(config.log_dir, "");
    }

    #[tokio::test]
    async fn test_path_validation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = "C:\\valid\\path"
        temp_dir = "D:\\another\\valid\\path"
        log_dir = ".\\relative\\path"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert!(config.config_path.contains(':') || config.config_path.starts_with('\\'));
        assert!(config.temp_dir.contains(':') || config.temp_dir.starts_with('\\'));
    }

    #[tokio::test]
    async fn test_path_with_environment_variables() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = "%USERPROFILE%\\config"
        temp_dir = "%TEMP%"
        log_dir = "%APPDATA%\\logs"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert!(config.config_path.contains("%USERPROFILE%"));
        assert!(config.temp_dir.contains("%TEMP%"));
        assert!(config.log_dir.contains("%APPDATA%"));
    }

    #[tokio::test]
    async fn test_long_paths() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let long_path = "C:\\very\\long\\path\\that\\exceeds\\normal\\length\\and\\continues\\for\\many\\directories\\a\\b\\c\\d\\e\\f\\g\\h\\i\\j\\k\\l\\m\\n\\o\\p\\q\\r\\s\\t\\u\\v\\w\\x\\y\\z\\config";
        
        let escaped_path = long_path.replace('\\', "\\\\");
        
        let config_content = format!(r#"
        config_path = "{}"
        temp_dir = "C:\\Temp"
        log_dir = ".\\logs"
        "#, escaped_path);
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert_eq!(config.config_path, long_path);
    }

    #[tokio::test]
    async fn test_path_with_special_characters() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = "C:\\path\\with\\_underscore\\-dash\\.dot"
        temp_dir = "C:\\path\\with\\(parens)\\[brackets]\\{braces}"
        log_dir = ".\\path\\with\\@at\\#hash\\$dollar\\%percent"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert!(config.config_path.contains('_') && config.config_path.contains('-'));
        assert!(config.temp_dir.contains('(') || config.temp_dir.contains('['));
        assert!(config.log_dir.contains('@') || config.log_dir.contains('#'));
    }

    #[tokio::test]
    async fn test_windows_reserved_device_names() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = "C:\\path\\to\\CON\\file.txt"
        temp_dir = "C:\\path\\to\\PRN\\temp"
        log_dir = ".\\path\\to\\AUX\\logs"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert!(config.config_path.contains("CON"));
        assert!(config.temp_dir.contains("PRN"));
        assert!(config.log_dir.contains("AUX"));
    }

    #[tokio::test]
    async fn test_case_sensitivity() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = "C:\\USERS\\TEST\\CONFIG"
        temp_dir = "c:\\users\\test\\temp"
        log_dir = ".\\LOGS\\APP"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert_eq!(config.config_path, "C:\\USERS\\TEST\\CONFIG");
        assert_eq!(config.temp_dir, "c:\\users\\test\\temp");
        assert_eq!(config.log_dir, ".\\LOGS\\APP");
    }

    #[tokio::test]
    async fn test_trailing_separator() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = "C:\\path\\to\\config\\"
        temp_dir = "C:\\temp\\"
        log_dir = ".\\logs\\"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert!(config.config_path.ends_with('\\'));
        assert!(config.temp_dir.ends_with('\\'));
        assert!(config.log_dir.ends_with('\\'));
    }

    #[tokio::test]
    async fn test_network_drive_mapping() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let config_content = r#"
        config_path = "Z:\\mapped\\drive\\config"
        temp_dir = "Y:\\data\\temp"
        log_dir = "X:\\logs\\app"
        "#;
        
        fs::write(&config_path, config_content).unwrap();
        
        let loader = ConfigLoader::new().with_file(config_path.to_str().unwrap());
        let config: WindowsPathConfig = loader.load().await.unwrap();
        
        assert_eq!(config.config_path, "Z:\\mapped\\drive\\config");
        assert_eq!(config.temp_dir, "Y:\\data\\temp");
        assert_eq!(config.log_dir, "X:\\logs\\app");
    }
}

#[cfg(test)]
mod integration_tests {
    use tempfile::TempDir;
    use std::fs;
    use confers::watcher::ConfigWatcher;

    #[tokio::test]
    async fn test_windows_file_watching() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let initial_content = r#"
        config_path = "C:\\initial\\path"
        "#;
        
        fs::write(&config_path, initial_content).unwrap();
        
        let watcher = ConfigWatcher::new(vec![config_path.clone()]);
        let (_debouncer, rx) = watcher.watch().expect("Failed to start watcher");
        
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        let updated_content = r#"
        config_path = "C:\\updated\\path"
        "#;
        
        fs::write(&config_path, updated_content).unwrap();
        
        let event = rx.recv_timeout(std::time::Duration::from_secs(2));
        
        assert!(event.is_ok(), "Should receive watch event");
        let events = event.unwrap();
        assert!(events.is_ok(), "Watch event should be Ok");
    }
}
