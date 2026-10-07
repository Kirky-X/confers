// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Internal implementation module (not exposed externally).
//!
//! This directory contains all concrete implementations following BrickArchitecture.
//! Public traits are defined in `src/interface.rs`.
//!
//! ## Core Modules
//!
//! - `memory` - InMemoryConfig using moka cache
//! - `loader` - Format loading
//! - `merger` - Merge engine
//! - `format` - Format detection and parsing
//! - `config` - ConfigBuilder, SourceChain, sources, limits
//
#[cfg(feature = "audit")]
pub(crate) mod audit;
pub(crate) mod config;
#[cfg(feature = "context-aware")]
pub(crate) mod context;
pub(crate) mod convert;
#[cfg(feature = "dynamic")]
pub(crate) mod dynamic;
pub(crate) mod format;
#[cfg(feature = "interpolation")]
pub(crate) mod interpolation;
pub(crate) mod lifecycle;
pub(crate) mod loader;
pub(crate) mod memory;
pub(crate) mod merger;
#[cfg(feature = "migration")]
pub(crate) mod migration;
#[cfg(feature = "modules")]
pub(crate) mod modules;
#[cfg(feature = "json-schema")]
pub(crate) mod schema;
#[cfg(feature = "snapshot")]
pub(crate) mod snapshot;
#[cfg(feature = "validation")]
pub(crate) mod validator;
