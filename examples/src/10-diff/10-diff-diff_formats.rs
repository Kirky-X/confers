#[cfg(feature = "cli")]
use confers::commands::{DiffCommand, DiffOptions, DiffFormat};

fn main() -> anyhow::Result<()> {
    #[cfg(feature = "cli")]
    {
        let options = DiffOptions {
            format: DiffFormat::SideBySide,
            ..Default::default()
        };
        DiffCommand::execute("src/10-diff/configs/config_v1.toml", "src/10-diff/configs/config_v2.toml", options)?;
    }
    #[cfg(not(feature = "cli"))]
    {
        println!("This example requires the 'cli' feature. Run with: cargo run --example 10-diff-diff_formats --features cli");
    }
    Ok(())
}
