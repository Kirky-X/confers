use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct DebounceConfig {
    pub message: String,
}

fn main() -> anyhow::Result<()> {
    let config = DebounceConfig::load()?;
    println!("Debounce Config: {:?}", config);
    Ok(())
}
