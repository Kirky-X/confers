use std::fmt::Debug;

#[derive(Clone, Default)]
pub struct SecretString(String);

impl Drop for SecretString {
    fn drop(&mut self) {
        let bytes = unsafe { self.0.as_bytes_mut() };
        for byte in bytes.iter_mut() {
            unsafe { std::ptr::write_volatile(byte, 0) };
        }
        self.0.clear();
    }
}

impl SecretString {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn expose_clone(&self) -> String {
        self.0.clone()
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
        &self.0
    }
}
