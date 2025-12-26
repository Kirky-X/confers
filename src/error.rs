use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum ConfigError {
    #[error("配置文件未找到: {path}")]
    FileNotFound { path: PathBuf },

    #[error("格式检测失败: {0}")]
    FormatDetectionFailed(String),

    #[error("解析失败: {0}")]
    ParseError(String),

    #[error("验证失败: {0}")]
    ValidationError(String),

    #[error("路径不安全: {0}")]
    UnsafePath(PathBuf),

    #[error("远程配置加载失败: {0}")]
    RemoteError(String),

    #[error("加载配置失败")]
    LoadError,

    #[error("运行时错误: {0}")]
    RuntimeError(String),

    #[error("序列化错误: {0}")]
    SerializationError(String),

    #[error("IO 错误: {0}")]
    IoError(String),

    #[error("内存限制超出: 限制 {limit}MB, 当前 {current}MB")]
    MemoryLimitExceeded { limit: usize, current: usize },

    #[error("密钥错误: {0}")]
    KeyError(String),

    #[error("密钥未找到: {key_id}")]
    KeyNotFound { key_id: String },

    #[error("密钥版本不匹配: 期望 {expected}, 实际 {actual}")]
    KeyVersionMismatch { expected: u32, actual: u32 },

    #[error("密钥轮换失败: {0}")]
    KeyRotationFailed(String),

    #[error("密钥存储错误: {0}")]
    KeyStorageError(String),

    #[error("密钥验证失败: 校验和不匹配")]
    KeyChecksumMismatch,

    #[error("密钥已过期: {key_id}, 版本 {version}")]
    KeyExpired { key_id: String, version: u32 },

    #[error("密钥已废弃: {key_id}, 版本 {version}")]
    KeyDeprecated { key_id: String, version: u32 },

    #[error("主密钥无效: {0}")]
    InvalidMasterKey(String),

    #[error("密钥策略错误: {0}")]
    KeyPolicyError(String),

    #[error("环境变量安全验证失败: {0}")]
    EnvSecurityError(String),

    #[error("其他错误: {0}")]
    Other(String),
}

impl From<validator::ValidationErrors> for ConfigError {
    fn from(_err: validator::ValidationErrors) -> Self {
        ConfigError::ValidationError("验证失败".to_string())
    }
}

impl From<figment::Error> for ConfigError {
    fn from(_err: figment::Error) -> Self {
        ConfigError::LoadError
    }
}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::IoError(err.to_string())
    }
}

impl From<String> for ConfigError {
    fn from(s: String) -> Self {
        ConfigError::FormatDetectionFailed(s)
    }
}

impl From<crate::security::EnvSecurityError> for ConfigError {
    fn from(err: crate::security::EnvSecurityError) -> Self {
        ConfigError::EnvSecurityError(err.to_string())
    }
}
