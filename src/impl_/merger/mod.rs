// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Configuration merge engine.
//!
//! This module provides various merge strategies for combining configuration values
//! from multiple sources with different priorities.

mod engine;
mod strategy;

pub use engine::MergeEngine;
pub use strategy::{CustomMergeFn, MergeStrategy};
