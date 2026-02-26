//! Configuration merge engine.
//!
//! This module provides various merge strategies for combining configuration values
//! from multiple sources with different priorities.

mod strategy;
mod engine;

pub use strategy::*;
pub use engine::*;
