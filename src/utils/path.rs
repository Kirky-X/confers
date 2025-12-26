use crate::error::ConfigError;
use path_absolutize::Absolutize;
use std::path::{Path, PathBuf};

pub struct PathUtils;

impl PathUtils {
    /// Normalize path: expand `~` and env vars, resolve absolute path, and security check
    pub fn normalize(path: &str) -> Result<PathBuf, ConfigError> {
        // 1. Expand `~` and env vars
        let expanded = shellexpand::full(path).map_err(|e| {
            ConfigError::FormatDetectionFailed(format!("Path expansion failed: {}", e))
        })?;

        // 2. Absolutize (resolve .. and .)
        let path_buf = PathBuf::from(expanded.as_ref());
        let absolute = path_buf
            .absolutize()
            .map_err(|_| ConfigError::UnsafePath(path_buf.clone()))?;

        // 3. Security Check
        Self::validate_security(&absolute)?;

        Ok(absolute.to_path_buf())
    }

    /// Validate path security
    pub fn validate_security(path: &Path) -> Result<(), ConfigError> {
        let path_str = path.to_string_lossy();

        // Forbidden prefixes
        let forbidden = [
            "/etc/shadow",
            "/etc/passwd",
            "/proc",
            "/sys",
            "C:\\Windows\\System32",
        ];

        for &p in &forbidden {
            if path_str.starts_with(p) {
                return Err(ConfigError::UnsafePath(path.to_path_buf()));
            }
        }

        Ok(())
    }

    /// Convert path to Unix style string (forward slashes)
    pub fn to_unix_string(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "/")
    }
}
