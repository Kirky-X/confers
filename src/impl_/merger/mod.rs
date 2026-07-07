// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Configuration merge engine.
//!
//! This module provides various merge strategies for combining configuration values
//! from multiple sources with different priorities.

mod engine;
mod strategy;

pub use engine::MergeEngine;
pub use strategy::{CustomMergeFn, MergeStrategy};
