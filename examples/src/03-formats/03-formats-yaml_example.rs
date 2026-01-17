use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct YamlConfig {
    pub name: String,
    pub port: u16,
}

fn main() -> anyhow::Result<()> {
    let config_content = "name: yaml-example\nport: 8080\n";
    std::fs::write("src/03-formats/configs/config.yaml", config_content)?;
    let config = YamlConfig::load()?;
    println!("YAML Config: {:?}", config);
    Ok(())
}
