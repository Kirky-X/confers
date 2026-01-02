// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! 验证测试模块
//! 
//! 包含所有配置验证相关的测试

// 基础验证测试
#[path = "../validation_test.rs"]
mod basic_validation;

// 嵌套验证测试
#[path = "../nested_validation_test.rs"]
mod nested_validation;

// 并行验证测试
#[cfg(feature = "parallel")]
#[path = "../parallel_validation_test.rs"]
mod parallel_validation;