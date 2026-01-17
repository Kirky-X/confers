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
    /// Minimum allowed port number (well-known ports)
    pub const MIN_PORT: u16 = 1024;

    /// Maximum allowed port number
    pub const MAX_PORT: u16 = 65535;

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

/// Time-related constants
pub mod time {
    /// Seconds in a day
    pub const SECONDS_PER_DAY: u64 = 86400;

    /// Milliseconds in a second
    pub const MILLISECONDS_PER_SECOND: u64 = 1000;

    /// Memory cache duration in milliseconds (1 second)
    /// This balances performance and accuracy for memory monitoring
    pub const MEMORY_CACHE_DURATION_MS: u64 = 1000;

    /// Default wait time for retry operations in milliseconds
    pub const DEFAULT_RETRY_WAIT_MS: u64 = 100;
}

/// Memory-related constants
pub mod memory {
    /// Bytes in a kilobyte
    pub const BYTES_PER_KB: f64 = 1024.0;

    /// Kilobytes in a megabyte
    pub const KB_PER_MB: f64 = 1024.0;

    /// Default memory limit in MB for configuration loading
    pub const DEFAULT_MEMORY_LIMIT_MB: usize = 512;
}

/// Configuration-related constants
pub mod config {
    /// Maximum configuration file size in megabytes
    /// This limit prevents DoS attacks and memory exhaustion
    pub const MAX_CONFIG_SIZE_MB: usize = 10;

    /// Minimum recommended memory limit in MB
    pub const MIN_MEMORY_LIMIT_MB: usize = 100;

    /// Maximum string length for configuration values
    pub const MAX_STRING_LENGTH: usize = 1024;

    /// Maximum array length for configuration values
    pub const MAX_ARRAY_LENGTH: usize = 100;
}

/// UI-related constants
pub mod ui {
    /// Width for separator lines in CLI output
    pub const SEPARATOR_WIDTH: usize = 60;
}

/// Audit-related constants
pub mod audit {
    /// Default maximum audit log file size in megabytes
    pub const DEFAULT_MAX_AUDIT_SIZE_MB: usize = 100;

    /// Default maximum number of archived audit log files
    pub const DEFAULT_MAX_AUDIT_FILES: usize = 10;

    /// Default audit log retention period in days
    pub const DEFAULT_AUDIT_RETENTION_DAYS: usize = 30;

    /// Maximum visible characters for sensitive data masking
    pub const MAX_VISIBLE_LENGTH: usize = 100;

    /// Number of visible characters for sensitive data masking
    pub const VISIBLE_CHARS: usize = 4;
}
