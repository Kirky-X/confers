// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

use crate::error::ConfigError;
use figment::providers::{Format, Json, Toml, Yaml};
use figment::Figment;
use std::path::PathBuf;

pub struct FileProvider {
    search_paths: Vec<PathBuf>,
}

impl FileProvider {
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self { search_paths }
    }

    pub fn load(&self) -> Result<Figment, ConfigError> {
        let mut figment = Figment::new();

        for path in &self.search_paths {
            // Try to load with different extensions if the path is a directory or base name
            // For simplicity, we assume `path` might be a file or a base name.
            // If it's a base name like "config", we try extensions.

            // This is a naive implementation. A robust one would check file existence.
            // Figment providers usually fail gracefully if file not found, or we can check existence.

            // Priorities: TOML > JSON > YAML

            // If path has extension, just load it.
            if let Some(ext) = path.extension() {
                if ext == "toml" {
                    figment = figment.merge(Toml::file(path));
                } else if ext == "json" {
                    figment = figment.merge(Json::file(path));
                } else if ext == "yaml" || ext == "yml" {
                    figment = figment.merge(Yaml::file(path));
                } else if ext == "ini" {
                    // figment doesn't have built-in ini provider, but we can use serde_ini
                    if let Ok(content) = std::fs::read_to_string(path) {
                        if let Ok(val) = serde_ini::from_str::<serde_json::Value>(&content) {
                            figment = figment.merge(figment::providers::Serialized::defaults(val));
                        }
                    }
                }
            } else {
                // Try extensions
                figment = figment.merge(Toml::file(path.with_extension("toml")));
                figment = figment.merge(Json::file(path.with_extension("json")));
                figment = figment.merge(Yaml::file(path.with_extension("yaml")));
                // Try INI
                let ini_path = path.with_extension("ini");
                if ini_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(ini_path) {
                        if let Ok(val) = serde_ini::from_str::<serde_json::Value>(&content) {
                            figment = figment.merge(figment::providers::Serialized::defaults(val));
                        }
                    }
                }
            }
        }

        Ok(figment)
    }
}
