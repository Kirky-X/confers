// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use crate::providers::{ConfigProvider, ProviderMetadata, ProviderType};
use figment::providers::{Format, Json, Toml, Yaml};
use figment::Figment;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FileConfigProvider {
    paths: Vec<PathBuf>,
    name: String,
    priority: u8,
    format_detection: String,
    allowed_dirs: Vec<PathBuf>,
}

impl FileConfigProvider {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self {
            paths,
            name: "file".to_string(),
            priority: 20,
            format_detection: "Auto".to_string(),
            allowed_dirs: Vec::new(),
        }
    }

    pub fn from_search_paths(search_paths: Vec<PathBuf>) -> Self {
        let mut paths = Vec::new();
        for search_path in &search_paths {
            if search_path.is_dir() {
                let base_name = search_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("config");
                let base_path = search_path.join(base_name);
                paths.push(base_path);
            } else {
                paths.push(search_path.clone());
            }
        }
        Self::new(paths)
    }

    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_format_detection(mut self, mode: impl Into<String>) -> Self {
        self.format_detection = mode.into();
        self
    }

    pub fn with_allowed_dirs(mut self, dirs: Vec<PathBuf>) -> Self {
        self.allowed_dirs = dirs;
        self
    }

    pub fn single_file(path: impl AsRef<Path>) -> Self {
        Self::new(vec![path.as_ref().to_path_buf()])
    }

    fn is_path_safe(&self, path: &Path) -> bool {
        if self.allowed_dirs.is_empty() {
            return true;
        }

        let canonical_path = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => return false,
        };

        self.allowed_dirs
            .iter()
            .all(|dir| match dir.canonicalize() {
                Ok(canonical_dir) => canonical_path.starts_with(&canonical_dir),
                Err(_) => false,
            })
    }

    pub fn detect_format(&self, path: &Path) -> Option<String> {
        match self.format_detection.as_str() {
            "ByExtension" => self.detect_by_extension(path),
            "ByContent" => self.detect_by_content(path),
            _ => {
                // Auto: try extension first, then content
                self.detect_by_extension(path)
                    .or_else(|| self.detect_by_content(path))
            }
        }
    }

    fn detect_by_extension(&self, path: &Path) -> Option<String> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .and_then(|ext| match ext.as_str() {
                "toml" => Some("toml".to_string()),
                "json" => Some("json".to_string()),
                "yaml" | "yml" => Some("yaml".to_string()),
                _ => None,
            })
    }

    fn detect_by_content(&self, path: &Path) -> Option<String> {
        let content = std::fs::read_to_string(path).ok()?;
        let trimmed = content.trim();

        if trimmed.is_empty() {
            return None;
        }

        // Check for JSON
        if ((trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']')))
            && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
        {
            return Some("json".to_string());
        }

        // Check for TOML
        if trimmed.contains('=') && toml::from_str::<toml::Value>(trimmed).is_ok() {
            return Some("toml".to_string());
        }

        // Check for YAML
        if (trimmed.starts_with("---") || trimmed.contains(':'))
            && serde_yaml::from_str::<serde_yaml::Value>(trimmed).is_ok()
        {
            return Some("yaml".to_string());
        }

        None
    }

    fn load_file(&self, path: &Path) -> Result<Figment, ConfigError> {
        if !path.exists() {
            return Ok(Figment::new());
        }

        if !self.is_path_safe(path) {
            return Err(ConfigError::UnsafePath(path.to_path_buf()));
        }

        // Skip editor temporary files
        if crate::core::loader::is_editor_temp_file(path) {
            return Ok(Figment::new());
        }

        let path_str = path.to_string_lossy();
        let mut figment = Figment::new();

        let format = self.detect_format(path);

        match format.as_deref() {
            Some("toml") => figment = figment.merge(Toml::file(path_str.as_ref())),
            Some("json") => figment = figment.merge(Json::file(path_str.as_ref())),
            Some("yaml") => figment = figment.merge(Yaml::file(path_str.as_ref())),
            _ => {
                // Fallback: try extension as last resort, or default to JSON
                if let Some(ext_fmt) = self.detect_by_extension(path) {
                    match ext_fmt.as_str() {
                        "toml" => figment = figment.merge(Toml::file(path_str.as_ref())),
                        "yaml" => figment = figment.merge(Yaml::file(path_str.as_ref())),
                        _ => figment = figment.merge(Json::file(path_str.as_ref())),
                    }
                } else {
                    figment = figment.merge(Json::file(path_str.as_ref()));
                }
            }
        }

        Ok(figment)
    }
}

impl ConfigProvider for FileConfigProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        let mut figment = Figment::new();

        for path in &self.paths {
            match self.load_file(path) {
                Ok(file_figment) => {
                    figment = figment.merge(file_figment);
                }
                Err(e) => {
                    // Log error but continue with other files
                    eprintln!("Warning: Failed to load file {}: {}", path.display(), e);
                }
            }
        }

        Ok(figment)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn is_available(&self) -> bool {
        self.paths.iter().any(|path| path.exists())
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: self.name.clone(),
            description: format!(
                "File-based configuration provider with {} paths",
                self.paths.len()
            ),
            source_type: ProviderType::File,
            requires_network: false,
            supports_watch: true,
            priority: self.priority,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[deprecated(since = "0.4.0", note = "Use FileConfigProvider instead")]
pub type FileProvider = FileConfigProvider;

/// Standard file configuration provider that follows common conventions
pub struct StandardFileProvider {
    app_name: String,
    run_env: Option<String>,
    search_paths: Vec<PathBuf>,
}

impl StandardFileProvider {
    pub fn new(app_name: impl Into<String>) -> Self {
        let app_name = app_name.into();
        let run_env = std::env::var("RUN_ENV").ok();

        let mut search_paths = vec![std::path::PathBuf::from(".")];

        if let Some(config_dir) = dirs::config_dir() {
            search_paths.push(config_dir.join(&app_name));
            search_paths.push(config_dir);
        }

        if let Some(home) = dirs::home_dir() {
            search_paths.push(home);
        }

        #[cfg(unix)]
        search_paths.push(std::path::PathBuf::from(format!("/etc/{}", app_name)));

        Self {
            app_name,
            run_env,
            search_paths,
        }
    }

    pub fn with_env(mut self, env: impl Into<String>) -> Self {
        self.run_env = Some(env.into());
        self
    }
}

impl ConfigProvider for StandardFileProvider {
    fn load(&self) -> Result<Figment, ConfigError> {
        let mut base_paths = Vec::new();
        let mut env_paths = Vec::new();
        let formats = ["toml", "json", "yaml", "yml"];

        for search_path in &self.search_paths {
            let base_path_no_ext = search_path.join(&self.app_name);

            // 1. Collect base config files
            let mut found_base_for_path = Vec::new();
            for fmt in &formats {
                let config_path = base_path_no_ext.with_extension(fmt);
                if config_path.exists() {
                    found_base_for_path.push(config_path);
                }
            }

            // Check for multiple formats in the same directory (base config)
            if found_base_for_path.len() > 1 {
                let paths_str = found_base_for_path
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                eprintln!(
                    "Warning: Multiple configuration formats found for base config in {}: {}. Using {} as priority.",
                    search_path.display(),
                    paths_str,
                    found_base_for_path[0].display()
                );
            }
            if let Some(first) = found_base_for_path.first() {
                base_paths.push(first.clone());
            }

            // 2. Collect environment-specific config files
            if let Some(ref env) = self.run_env {
                let mut found_env_for_path = Vec::new();
                for fmt in &formats {
                    let env_config_path = search_path
                        .join(format!("{}.{}", self.app_name, env))
                        .with_extension(fmt);
                    if env_config_path.exists() {
                        found_env_for_path.push(env_config_path);
                    }
                }

                // Check for multiple formats in the same directory (env config)
                if found_env_for_path.len() > 1 {
                    let paths_str = found_env_for_path
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    eprintln!(
                        "Warning: Multiple configuration formats found for environment '{}' in {}: {}. Using {} as priority.",
                        env,
                        search_path.display(),
                        paths_str,
                        found_env_for_path[0].display()
                    );
                }
                if let Some(first) = found_env_for_path.first() {
                    env_paths.push(first.clone());
                }
            }
        }

        if base_paths.is_empty() && env_paths.is_empty() {
            return Ok(Figment::new());
        }

        // Combine paths: base paths first, then env paths (which will override base)
        let mut all_paths = base_paths;
        all_paths.extend(env_paths);

        let file_provider = FileConfigProvider::new(all_paths)
            .with_name("standard_file")
            .with_priority(10);

        file_provider.load()
    }

    fn name(&self) -> &str {
        "standard_file"
    }

    fn is_available(&self) -> bool {
        let formats = ["toml", "json", "yaml", "yml"];

        for search_path in &self.search_paths {
            let base_path = search_path.join(&self.app_name);

            for fmt in &formats {
                if base_path.with_extension(fmt).exists() {
                    return true;
                }

                if let Some(ref env) = self.run_env {
                    if search_path
                        .join(format!("{}.{}", self.app_name, env))
                        .with_extension(fmt)
                        .exists()
                    {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn priority(&self) -> u8 {
        10 // Medium priority
    }

    fn metadata(&self) -> ProviderMetadata {
        ProviderMetadata {
            name: "standard_file".to_string(),
            description: format!(
                "Standard file provider for app '{}' with {} search paths",
                self.app_name,
                self.search_paths.len()
            ),
            source_type: ProviderType::File,
            requires_network: false,
            supports_watch: true,
            priority: self.priority(),
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
