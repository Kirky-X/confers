// Copyright (c) 2025 Kirky.X
//
// Licensed under the MIT License
// See LICENSE file in the project root for full license information.

//! Polling interval presets for remote sources.

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::error::{ConfigError, ConfigResult};

/// Polling interval presets for remote sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum PollInterval {
    /// Fast polling (10 seconds) - for critical configs
    Fast,
    /// Normal polling (30 seconds) - default for etcd/consul
    #[default]
    Normal,
    /// Slow polling (60 seconds) - default for HTTP
    Slow,
    /// Custom interval
    Custom(u64),
}

impl PollInterval {
    /// Get the duration
    pub fn as_duration(&self) -> Duration {
        match self {
            Self::Fast => Duration::from_secs(10),
            Self::Normal => Duration::from_secs(30),
            Self::Slow => Duration::from_secs(60),
            Self::Custom(secs) => Duration::from_secs(*secs),
        }
    }

    /// Create a custom interval (with validation)
    pub fn custom(secs: u64) -> ConfigResult<Self> {
        if secs < 1 {
            return Err(ConfigError::InvalidValue {
                key: "poll_interval".to_string(),
                expected_type: "u64 >= 1".to_string(),
                message: "Poll interval must be at least 1 second".to_string(),
            });
        }
        if secs > 3600 {
            return Err(ConfigError::InvalidValue {
                key: "poll_interval".to_string(),
                expected_type: "u64 <= 3600".to_string(),
                message: "Poll interval too large (max 1 hour)".to_string(),
            });
        }
        Ok(Self::Custom(secs))
    }

    /// Default for etcd
    pub fn etcd_default() -> Self {
        Self::Normal
    }

    /// Default for Consul
    pub fn consul_default() -> Self {
        Self::Normal
    }

    /// Default for HTTP polling
    pub fn http_default() -> Self {
        Self::Slow
    }
}

impl std::fmt::Display for PollInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fast => write!(f, "10s"),
            Self::Normal => write!(f, "30s"),
            Self::Slow => write!(f, "60s"),
            Self::Custom(secs) => write!(f, "{}s", secs),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poll_interval_as_duration() {
        assert_eq!(PollInterval::Fast.as_duration(), Duration::from_secs(10));
        assert_eq!(PollInterval::Normal.as_duration(), Duration::from_secs(30));
        assert_eq!(PollInterval::Slow.as_duration(), Duration::from_secs(60));
        assert_eq!(
            PollInterval::Custom(45).as_duration(),
            Duration::from_secs(45)
        );
    }

    #[test]
    fn test_poll_interval_custom_valid() {
        assert!(PollInterval::custom(1).is_ok());
        assert!(PollInterval::custom(3600).is_ok());
    }

    #[test]
    fn test_poll_interval_custom_invalid() {
        assert!(PollInterval::custom(0).is_err());
        assert!(PollInterval::custom(3601).is_err());
    }

    #[test]
    fn test_poll_interval_defaults() {
        assert_eq!(PollInterval::etcd_default(), PollInterval::Normal);
        assert_eq!(PollInterval::consul_default(), PollInterval::Normal);
        assert_eq!(PollInterval::http_default(), PollInterval::Slow);
    }

    #[test]
    fn test_poll_interval_display() {
        assert_eq!(format!("{}", PollInterval::Fast), "10s");
        assert_eq!(format!("{}", PollInterval::Normal), "30s");
        assert_eq!(format!("{}", PollInterval::Slow), "60s");
        assert_eq!(format!("{}", PollInterval::Custom(45)), "45s");
    }
}
