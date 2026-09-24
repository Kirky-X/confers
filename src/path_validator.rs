// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Secure path validation for `_FILE`-style secret file references.
//!
//! This module is intentionally **not feature-gated**: generated derive-macro
//! code references [`PathValidator`] unconditionally whenever a struct has
//! sensitive (`SecretString`/`encrypt`) fields, so it must exist in every
//! feature configuration.

use crate::error::{ConfigError, ConfigResult};
use crate::i18n::{tr, tr_args};
use std::path::{Path, PathBuf};

/// Validator for secret file paths referenced through `<VAR>_FILE`
/// environment variables (Docker/K8s secrets convention).
///
/// Policy mirrors [`crate::EnvSource`]'s own `_FILE` handling: reject empty
/// paths, refuse paths that cannot be resolved, block access to sensitive
/// system locations, and return the canonical path so callers read exactly
/// the file that was validated.
#[derive(Debug, Default, Clone, Copy)]
pub struct PathValidator {}

impl PathValidator {
    /// Create a new validator with the default policy.
    pub fn new() -> Self {
        Self {}
    }

    /// Validate `file_path` and return its canonical form.
    ///
    /// Errors mirror [`crate::EnvSource`]'s `_FILE` error wording so both
    /// paths report consistently.
    pub fn validate_and_resolve(&self, file_path: &str) -> ConfigResult<PathBuf> {
        // Reject empty paths up front with a clear error instead of letting
        // them through and failing later with a confusing read error.
        if file_path.is_empty() {
            return Err(ConfigError::InvalidValue {
                key: "file_path".to_string(),
                expected_type: "non-empty file path".to_string(),
                message: tr("error-path-empty"),
            });
        }

        let path = Path::new(file_path);

        // Check for path traversal attempts
        let canonical = std::fs::canonicalize(path).map_err(|_| ConfigError::FileNotFound {
            filename: path.to_path_buf(),
            source: Some(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                tr("error-path-cannot-resolve"),
            )),
        })?;

        // Block access to sensitive system paths.
        let sensitive_prefixes = [
            Path::new("/etc/shadow"),
            Path::new("/etc/passwd"),
            Path::new("/root"),
        ];

        for prefix in &sensitive_prefixes {
            // Path::starts_with checks path components (not string prefix),
            // so "/rootkit" does NOT match "/root" — this is correct.
            if canonical.starts_with(prefix) {
                return Err(ConfigError::InvalidValue {
                    key: "file_path".to_string(),
                    expected_type: "safe file path".to_string(),
                    message: tr_args(
                        "error-path-access-denied",
                        &[("path", format!("{prefix:?}"))],
                    ),
                });
            }
        }

        // Well-known credential directories, matched per path component so
        // they are blocked wherever they appear (e.g. /home/alice/.ssh/...)
        // without denying every file under /home.
        const CREDENTIAL_DIR_COMPONENTS: &[&str] =
            &[".ssh", ".aws", ".gnupg", ".kube", ".gcloud", ".env"];
        let hits_credential_dir = canonical.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .is_some_and(|s| CREDENTIAL_DIR_COMPONENTS.contains(&s))
        });
        if hits_credential_dir {
            return Err(ConfigError::InvalidValue {
                key: "file_path".to_string(),
                expected_type: "safe file path".to_string(),
                message: tr("error-path-credential-dir-denied"),
            });
        }

        // Only allow reading regular files
        if !canonical.is_file() {
            return Err(ConfigError::InvalidValue {
                key: "file_path".to_string(),
                expected_type: "regular file".to_string(),
                message: tr("error-path-not-regular-file"),
            });
        }

        // Only allow specific extensions for security
        if let Some(ext) = canonical.extension() {
            let allowed = [
                "txt", "json", "yaml", "yml", "toml", "ini", "env", "secret", "key", "pem", "crt",
            ];
            if !allowed
                .iter()
                .any(|&e| ext.to_str().is_some_and(|s| s.eq_ignore_ascii_case(e)))
            {
                return Err(ConfigError::InvalidValue {
                    key: "file_path".to_string(),
                    expected_type: "allowed extension".to_string(),
                    message: tr_args(
                        "error-path-extension-denied",
                        &[("ext", format!("{ext:?}"))],
                    ),
                });
            }
        }

        Ok(canonical)
    }
}
