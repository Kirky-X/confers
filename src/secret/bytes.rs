// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

use std::fmt::Debug;

use zeroize::Zeroize;

pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }
}

impl Debug for SecretBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl Drop for SecretBytes {
    fn drop(&mut self) {
        // `Vec::zeroize` writes zeros with volatile stores (so the optimizer
        // cannot elide the write) and clears the entire allocation, including
        // spare capacity beyond `len`.
        self.0.zeroize();
    }
}

impl Zeroize for SecretBytes {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

// SecretBytes does not implement Clone to prevent bypassing memory protection.
// Drop zeroizes the buffer through the `zeroize` crate, whose volatile writes
// cannot be optimized away and which zeroes the full capacity of the
// allocation (not just the initialized length), so no secret bytes survive on
// the heap after the value is gone.
// If you need to clone, consider using ZeroizingBytes or explicit copying.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_accessors() {
        let secret = SecretBytes::new(vec![1, 2, 3]);
        assert_eq!(secret.as_slice(), &[1, 2, 3]);
        assert_eq!(secret.len(), 3);
        assert!(!secret.is_empty());

        let empty = SecretBytes::new(Vec::new());
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
    }

    #[test]
    fn test_debug_does_not_leak_secret() {
        let secret = SecretBytes::new(b"top-secret".to_vec());
        let debug_str = format!("{:?}", secret);
        assert_eq!(debug_str, "[REDACTED]");
        assert!(!debug_str.contains("top-secret"));
    }

    #[test]
    fn test_zeroize_clears_len_and_spare_capacity() {
        // Build a Vec whose spare capacity exceeds its length: a plain
        // `fill(0)` would only clear the `len` bytes and leave the tail of
        // the allocation intact.
        let mut inner = Vec::with_capacity(64);
        inner.extend_from_slice(b"top-secret");
        let capacity = inner.capacity();
        assert!(capacity > inner.len());

        let mut secret = SecretBytes::new(inner);
        secret.zeroize();

        // The length is cleared and the entire allocation, including the
        // spare capacity, has been overwritten with zeros.
        assert!(secret.0.is_empty());
        assert_eq!(secret.0.capacity(), capacity);
        assert!(secret.0.iter().all(|&b| b == 0));
        // SAFETY: `Vec::zeroize` writes zeros over the full capacity, so
        // every byte of the spare capacity is initialized (to zero) and
        // safe to read here.
        let spare = secret.0.spare_capacity_mut();
        assert!(spare.iter().all(|b| unsafe { b.assume_init() } == 0));
    }
}
