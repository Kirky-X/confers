// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use tempfile::TempDir;

pub fn create_temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp directory")
}

pub fn create_test_config(content: &str) -> String {
    content.to_string()
}
