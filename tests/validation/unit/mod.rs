// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 单元测试模块
//!
//! 包含配置验证的基础功能测试

pub mod validation_basic;
pub mod validation_nested;

#[cfg(feature = "parallel")]
pub mod validation_parallel;