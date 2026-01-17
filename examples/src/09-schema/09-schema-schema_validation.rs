use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct SchemaValidationConfig {
    pub name: String,
}

fn main() -> anyhow::Result<()> {
    let config = SchemaValidationConfig::load()?;
    println!("Schema Validation Config: {:?}", config);
    Ok(())
}
