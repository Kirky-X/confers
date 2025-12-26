use figment::providers::Env;
use figment::Figment;

pub struct EnvProvider {
    prefix: String,
}

impl EnvProvider {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }

    pub fn load(&self) -> Figment {
        Figment::from(Env::prefixed(&self.prefix).split("__"))
    }
}
