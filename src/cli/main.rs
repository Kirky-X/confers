// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Confers CLI entry point

use anyhow::Result;

fn main() -> Result<()> {
    confers::cli::run::<serde_json::Value>()
}
