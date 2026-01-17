use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct DeepNestedConfig {
    pub server: ServerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls: TlsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert: String,
}

fn main() -> anyhow::Result<()> {
    let config = DeepNestedConfig::load()?;
    println!("Deep Nested Config: {:?}", config);
    Ok(())
}
