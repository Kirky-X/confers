use confers::Config;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Config)]
pub struct ArrayConfig {
    pub name: String,
    pub tags: Vec<String>,
    pub ports: Vec<u16>,
}

fn main() -> anyhow::Result<()> {
    let config_content = r#"
name = "array-example"
tags = ["rust", "config", "example"]
ports = [8080, 8081, 8082]
"#;
    std::fs::write("src/04-nested/configs/nested.toml", config_content)?;
    let config = ArrayConfig::load()?;
    println!("Array Config: {:?}", config);
    Ok(())
}
