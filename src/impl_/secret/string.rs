use std::fmt::Debug;
use zeroize::{Zeroize, Zeroizing};

#[derive(Clone)]
pub struct SecretString(Zeroizing<String>);

impl Default for SecretString {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl SecretString {
    pub fn new(s: impl Into<String>) -> Self {
        Self(Zeroizing::new(s.into()))
    }

    pub fn expose(&self) -> &str {
        self.0.as_str()
    }

    pub fn expose_clone(&self) -> String {
        self.0.to_string()
    }
}

impl Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl std::ops::Deref for SecretString {
    type Target = str;

    fn deref(&self) -> &str {
        self.0.as_str()
    }
}

impl Zeroize for SecretString {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}
