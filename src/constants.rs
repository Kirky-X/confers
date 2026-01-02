// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

/// Security-related constants
pub mod security {
    /// Maximum length for environment variable names (256 characters)
    pub const MAX_ENV_NAME_LENGTH: usize = 256;

    /// Maximum length for environment variable values (4096 characters)
    pub const MAX_ENV_VALUE_LENGTH: usize = 4096;

    /// Maximum length for environment variable names in strict mode (512 characters)
    pub const MAX_ENV_NAME_LENGTH_STRICT: usize = 512;

    /// Maximum length for environment variable values in strict mode (8192 characters)
    pub const MAX_ENV_VALUE_LENGTH_STRICT: usize = 8192;

    /// Maximum length for environment variable values in very strict mode (1024 characters)
    pub const MAX_ENV_VALUE_LENGTH_VERY_STRICT: usize = 1024;
}

/// Encryption-related constants
pub mod encryption {
    /// AES key size in bytes (256 bits)
    pub const AES_KEY_SIZE: usize = 32;

    /// AES key size in bits
    pub const AES_KEY_SIZE_BITS: usize = 256;
}

/// Network-related constants
pub mod network {
    /// Default HTTP request timeout in seconds
    pub const HTTP_TIMEOUT_SECS: u64 = 30;

    /// HTTP connection pool idle timeout in seconds
    pub const HTTP_POOL_IDLE_TIMEOUT_SECS: u64 = 90;

    /// Default provider priority
    pub const DEFAULT_PROVIDER_PRIORITY: u8 = 30;

    /// Default remote config poll interval in seconds
    pub const DEFAULT_POLL_INTERVAL_SECS: u64 = 60;
}

/// Key management constants
pub mod keys {
    /// Key rotation interval in days
    pub const KEY_ROTATION_INTERVAL_DAYS: u64 = 90;

    /// Days before key expiry to notify
    pub const KEY_EXPIRY_NOTIFY_DAYS: u64 = 30;

    /// Number of keys to rotate
    pub const KEY_ROTATION_COUNT: usize = 5;
}

/// Memory-related constants
pub mod memory {
    /// Bytes in a kilobyte
    pub const BYTES_PER_KB: f64 = 1024.0;

    /// Kilobytes in a megabyte
    pub const KB_PER_MB: f64 = 1024.0;
}

/// UI-related constants
pub mod ui {
    /// Width for separator lines in CLI output
    pub const SEPARATOR_WIDTH: usize = 60;
}
