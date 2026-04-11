//! Confers CLI entry point
//!
//! Binary entry point for the confers command-line tool.
//! See `src/cli/` for implementation.

use anyhow::Result;

fn main() -> Result<()> {
    confers::cli::run()
}
