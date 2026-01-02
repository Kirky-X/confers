// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 统一的测试配置模块
//! 
//! 提供所有测试文件共用的配置结构体

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use validator::Validate;
use confers::ConfigMap;

/// 简单的测试配置结构体
/// 
/// 包含基本的配置字段，用于大多数测试场景
#[derive(Debug, Clone, Serialize, Deserialize, Default, Validate)]
pub struct SimpleTestConfig {
    #[validate(length(min = 1))]
    pub name: String,
    
    #[validate(range(min = 0, max = 1000))]
    pub value: i32,
    
    pub enabled: bool,
}

impl ConfigMap for SimpleTestConfig {
    fn to_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("name".to_string(), serde_json::json!(self.name));
        map.insert("value".to_string(), serde_json::json!(self.value));
        map.insert("enabled".to_string(), serde_json::json!(self.enabled));
        map
    }

    fn env_mapping() -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("name".to_string(), "TEST_NAME".to_string());
        map.insert("value".to_string(), "TEST_VALUE".to_string());
        map.insert("enabled".to_string(), "TEST_ENABLED".to_string());
        map
    }
}

/// 服务器配置结构体
/// 
/// 用于测试服务器相关的配置
#[derive(Debug, Clone, Serialize, Deserialize, Default, Validate)]
pub struct ServerTestConfig {
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,
    
    #[validate(length(min = 1))]
    pub host: String,
}

impl ConfigMap for ServerTestConfig {
    fn to_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("port".to_string(), serde_json::json!(self.port));
        map.insert("host".to_string(), serde_json::json!(self.host));
        map
    }

    fn env_mapping() -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("port".to_string(), "TEST_PORT".to_string());
        map.insert("host".to_string(), "TEST_HOST".to_string());
        map
    }
}

/// 数据库配置结构体
/// 
/// 用于测试数据库相关的配置
#[derive(Debug, Clone, Serialize, Deserialize, Default, Validate)]
pub struct DatabaseTestConfig {
    #[validate(length(min = 1))]
    pub db_url: String,
    
    #[validate(range(min = 1, max = 100))]
    pub pool_size: usize,
}

impl ConfigMap for DatabaseTestConfig {
    fn to_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("db_url".to_string(), serde_json::json!(self.db_url));
        map.insert("pool_size".to_string(), serde_json::json!(self.pool_size));
        map
    }

    fn env_mapping() -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("db_url".to_string(), "TEST_DB_URL".to_string());
        map.insert("pool_size".to_string(), "TEST_POOL_SIZE".to_string());
        map
    }
}

/// 完整的应用配置结构体
/// 
/// 包含所有常见配置字段，用于综合测试
#[derive(Debug, Clone, Serialize, Deserialize, Default, Validate)]
pub struct FullTestConfig {
    #[validate(length(min = 1))]
    pub app_name: String,
    
    #[validate(range(min = 1, max = 65535))]
    pub server_port: u16,
    
    #[validate(length(min = 1))]
    pub server_host: String,
    
    pub debug_mode: bool,
    
    #[validate(range(min = 1, max = 100))]
    pub log_level: i32,
    
    pub db_password: String,
    
    pub db_url: String,
}

impl ConfigMap for FullTestConfig {
    fn to_map(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert("app_name".to_string(), serde_json::json!(self.app_name));
        map.insert("server_port".to_string(), serde_json::json!(self.server_port));
        map.insert("server_host".to_string(), serde_json::json!(self.server_host));
        map.insert("debug_mode".to_string(), serde_json::json!(self.debug_mode));
        map.insert("log_level".to_string(), serde_json::json!(self.log_level));
        map.insert("db_password".to_string(), serde_json::json!(self.db_password));
        map.insert("db_url".to_string(), serde_json::json!(self.db_url));
        map
    }

    fn env_mapping() -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("app_name".to_string(), "TEST_APP_NAME".to_string());
        map.insert("server_port".to_string(), "TEST_SERVER_PORT".to_string());
        map.insert("server_host".to_string(), "TEST_SERVER_HOST".to_string());
        map.insert("debug_mode".to_string(), "TEST_DEBUG_MODE".to_string());
        map.insert("log_level".to_string(), "TEST_LOG_LEVEL".to_string());
        map.insert("db_password".to_string(), "TEST_DB_PASSWORD".to_string());
        map.insert("db_url".to_string(), "TEST_DB_URL".to_string());
        map
    }
}