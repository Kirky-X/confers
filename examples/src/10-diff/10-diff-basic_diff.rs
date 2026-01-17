// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

#[cfg(feature = "cli")]
use confers::commands::{DiffCommand, DiffOptions, DiffFormat};

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "cli")]
    {
        let options = DiffOptions {
            format: DiffFormat::Unified,
            ..Default::default()
        };
        DiffCommand::execute("src/10-diff/configs/config_v1.toml", "src/10-diff/configs/config_v2.toml", options)?;
    }
    #[cfg(not(feature = "cli"))]
    {
        println!("This example requires the 'cli' feature. Run with: cargo run --example 10-diff-basic_diff --features cli");
    }
    Ok(())
}
