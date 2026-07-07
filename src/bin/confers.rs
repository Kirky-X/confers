// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Confers CLI entry point
//!
//! Binary entry point for the confers command-line tool.
//! See `src/cli/` for implementation.

use anyhow::Result;

fn main() -> Result<()> {
    confers::cli::run()
}
