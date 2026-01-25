// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in project root for full license information.

//! 共享类型定义
//!
//! 提供整个confers项目中使用的共享数据类型

use serde::{Deserialize, Serialize};

/// 配置源类型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigSource {
    File(String),
    Environment,
    Cli,
    Remote(String),
}

/// 配置元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    pub source: ConfigSource,
    pub last_modified: Option<String>, // 简化为字符串类型
    pub version: Option<String>,
}
