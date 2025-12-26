use confers::Config;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Config)]
pub struct TestConfig {
    pub name: String,
}

fn main() {
    println!("Testing simple macro");
}
