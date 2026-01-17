use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct EncryptionConfig {
    pub api_key: String,
}

fn main() -> anyhow::Result<()> {
    let config = EncryptionConfig::load()?;
    println!("Encryption Config: {:?}", config);
    Ok(())
}
