use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct IniConfig {
    pub name: String,
    pub port: u16,
}

fn main() -> anyhow::Result<()> {
    let config_content = "name = ini-example\nport = 8080\n";
    std::fs::write("src/03-formats/configs/config.ini", config_content)?;
    let config = IniConfig::load()?;
    println!("INI Config: {:?}", config);
    Ok(())
}
