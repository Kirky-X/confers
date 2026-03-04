//! Configuration merge engine.
//!
//! This module provides various merge strategies for combining configuration values
//! from multiple sources with different priorities.

mod engine;
mod strategy;

pub use engine::*;
pub use strategy::*;
