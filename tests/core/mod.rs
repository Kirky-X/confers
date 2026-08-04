// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

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
mod load;
mod merge;
mod migration;
mod modules;
mod nested_deserialize;
mod progressive;
