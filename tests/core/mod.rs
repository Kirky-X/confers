// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Core integration tests — configuration loading, merging, derive macros,
//! dynamic fields, context-aware config, migration, modules, coverage,
//! nested deserialization, env type inference, and progressive reload.

#[path = "../common.rs"]
pub mod common;

mod context;
mod coverage;
mod derive;
mod dynamic;
mod env_types;
mod error;
mod load;
mod merge;
mod migration;
mod modules;
mod nested_deserialize;
mod progressive;
mod toggle;
