use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct DecryptConfig {
    pub api_key: String,
}

fn main() -> anyhow::Result<()> {
    let config = DecryptConfig::load()?;
    println!("Decrypt Config: {:?}", config);
    Ok(())
}
