use confers::Config;
use serde::{Deserialize, Serialize};
// use confers::schema::typescript::generate_typescript; // TypeScript generation not available

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct TsConfig {
    pub name: String,
    pub port: u16,
}

fn main() -> anyhow::Result<()> {
    let config = TsConfig::load()?;
    // let ts_types = generate_typescript(&config)?;
    // println!("TypeScript Types: {}", ts_types);
    Ok(())
}
