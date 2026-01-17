use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
#[config(remote = "http://localhost:8080/config", remote_fallback = true)]
pub struct FallbackConfig {
    pub api_key: String,
}

fn main() -> anyhow::Result<()> {
    let config = FallbackConfig::load()?;
    println!("Fallback Config: {:?}", config);
    Ok(())
}
