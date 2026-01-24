// Copyright (c) 2025 Kirky.X
//
// Licensed under MIT License
// See LICENSE file in the project root for full license information.

//! Remote configuration support
//!
//! This module contains RemoteConfig and related types for remote configuration
//! management including etcd, Consul, and HTTP providers.

#[cfg(feature = "encryption")]
use crate::security::secure_string::{SecureString, SensitivityLevel};
#[cfg(feature = "remote")]
use crate::security::SecureString as RemoteSecureString;
#[allow(unused_imports)]
use std::path::PathBuf;

/// Remote configuration settings
#[cfg(feature = "remote")]
#[derive(Clone, Debug)]
pub struct RemoteConfig {
    pub enabled: bool,
    pub url: Option<String>,
    #[cfg(feature = "encryption")]
    pub token: Option<std::sync::Arc<SecureString>>,
    #[cfg(not(feature = "encryption"))]
    pub token: Option<String>,
    pub username: Option<String>,
    #[cfg(feature = "encryption")]
    pub password: Option<std::sync::Arc<SecureString>>,
    #[cfg(not(feature = "encryption"))]
    pub password: Option<String>,
    pub ca_cert: Option<PathBuf>,
    pub client_cert: Option<PathBuf>,
    pub client_key: Option<PathBuf>,
    pub timeout: Option<String>,
    pub fallback: bool,
}

#[cfg(feature = "remote")]
impl Default for RemoteConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: None,
            token: None,
            username: None,
            password: None,
            ca_cert: None,
            client_cert: None,
            client_key: None,
            timeout: None,
            fallback: true,
        }
    }
}

#[cfg(feature = "remote")]
impl RemoteConfig {
    /// Create a new remote config instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable/disable remote configuration (matches loader.rs API)
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set remote configuration URL
    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set authentication token (matches loader.rs API - takes String)
    #[cfg(feature = "encryption")]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(std::sync::Arc::new(SecureString::new(
            token.into(),
            SensitivityLevel::High,
        )));
        self
    }

    #[cfg(not(feature = "encryption"))]
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }

    /// Set username for authentication
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set password for authentication (matches loader.rs API - takes String)
    #[cfg(feature = "encryption")]
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(std::sync::Arc::new(SecureString::new(
            password.into(),
            SensitivityLevel::Critical,
        )));
        self
    }

    #[cfg(not(feature = "encryption"))]
    pub fn with_password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    /// Set request timeout
    pub fn with_timeout(mut self, timeout: impl Into<String>) -> Self {
        self.timeout = Some(timeout.into());
        self
    }

    /// Enable/disable fallback to local config
    pub fn with_fallback(mut self, fallback: bool) -> Self {
        self.fallback = fallback;
        self
    }

    /// Get URL reference
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// Get username reference
    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }
}

/// Audit configuration for config access tracking
#[cfg(feature = "audit")]
#[derive(Clone, Debug)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_file: Option<PathBuf>,
    pub sensitive_fields: Vec<String>,
}

#[cfg(feature = "audit")]
impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            log_file: None,
            sensitive_fields: Vec::new(),
        }
    }
}

#[cfg(feature = "audit")]
impl AuditConfig {
    /// Create a new audit config instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable audit logging
    pub fn enable(mut self) -> Self {
        self.enabled = true;
        self
    }

    /// Set log file path
    pub fn with_log_file(mut self, log_file: impl Into<PathBuf>) -> Self {
        self.log_file = Some(log_file.into());
        self
    }

    /// Add sensitive field to track
    pub fn add_sensitive_field(mut self, field: impl Into<String>) -> Self {
        self.sensitive_fields.push(field.into());
        self
    }

    /// Enable sensitive field tracking
    pub fn enable_sensitive_field_tracking(mut self) -> Self {
        self.sensitive_fields = vec![
            "password".to_string(),
            "secret".to_string(),
            "token".to_string(),
            "api_key".to_string(),
            "private_key".to_string(),
        ];
        self
    }
}
