// Re-export unified ZeroizingBytes from crate::types (BrickArchitecture: single source of truth).
pub use crate::types::ZeroizingBytes;

/// Convenience constructor for [`ZeroizingBytes`].
pub fn zeroizing_bytes(bytes: Vec<u8>) -> ZeroizingBytes {
    ZeroizingBytes::new(bytes)
}
