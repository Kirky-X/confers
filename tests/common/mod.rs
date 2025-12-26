use tempfile::TempDir;

pub fn create_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

pub fn create_test_config(content: &str) -> String {
    content.to_string()
}
