// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in project root for full license information.

//! 通用错误类型定义
//!
//! 提供整个confers项目中使用的共享错误类型

use thiserror::Error;

/// 通用配置错误类型
#[derive(Error, Debug)]
pub enum CommonError {
    #[error("IO错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("序列化错误: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("验证错误: {0}")]
    Validation(String),

    #[error("解析错误: {0}")]
    Parse(String),
}

/// 通用结果类型
pub type CommonResult<T> = Result<T, CommonError>;
