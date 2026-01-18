use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct SensitiveConfig {
    #[config(sensitive = true)]
    pub api_key: String,
}

fn main() -> anyhow::Result<()> {
    let _config = SensitiveConfig::load()?;
    println!("Sensitive Config: api_key=[REDACTED]");
    Ok(())
}
