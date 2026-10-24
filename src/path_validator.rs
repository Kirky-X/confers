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

/// Well-known credential directories, matched per path component so they are
/// blocked wherever they appear (e.g. /home/alice/.ssh/...) without denying
/// every file under /home.
///
/// The rejection message names the component that was matched instead of
/// repeating this list, so the list has a single home and cannot drift away
/// from the wording the user sees.
const CREDENTIAL_DIR_COMPONENTS: &[&str] = &[
    ".ssh", ".aws", ".gnupg", ".kube", ".gcloud", ".env", ".docker",
    // Un-dotted spelling of the docker credential home
    // (e.g. ~/.config/docker/config.json).
    "docker",
];

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
            Path::new("/etc/gshadow"),
            Path::new("/etc/sudoers"),
            Path::new("/etc/sudoers.d"),
            Path::new("/etc/ssh"),
            Path::new("/etc/security"),
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

        // The matched component is carried into the message so the rejected
        // directory is named instead of being hidden behind a fixed list
        // that could drift away from the constant.
        let matched_credential_dir = canonical.components().find_map(|component| {
            component
                .as_os_str()
                .to_str()
                .filter(|s| CREDENTIAL_DIR_COMPONENTS.contains(s))
                .map(|s| s.to_string())
        });
        if let Some(name) = matched_credential_dir {
            return Err(ConfigError::InvalidValue {
                key: "file_path".to_string(),
                expected_type: "safe file path".to_string(),
                message: tr_args("error-path-credential-dir-denied", &[("name", name)]),
            });
        }

        // Dot-files without an extension are where per-user credentials live
        // (`.netrc`, `.npmrc`, `.pypirc`, `.git-credentials`, `.pgpass`, …).
        // The extension whitelist below does not apply to them:
        // `Path::extension` reports `None` for such a name, so `.netrc`
        // would otherwise pass every check.
        let file_name = canonical
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        let is_extension_less_dotfile =
            file_name.starts_with('.') && canonical.extension().is_none();
        if is_extension_less_dotfile {
            return Err(ConfigError::InvalidValue {
                key: "file_path".to_string(),
                expected_type: "safe file path".to_string(),
                message: tr_args(
                    "error-path-credential-file-denied",
                    &[("name", file_name.to_string())],
                ),
            });
        }

        // Well-known credential file names that carry no extension and
        // therefore bypass the extension whitelist as well.
        const CREDENTIAL_FILE_NAMES: &[&str] = &[
            "credentials",
            "credentials.json",
            "credentials.toml",
            "credentials.yaml",
            "credentials.yml",
            "shadow",
            "gshadow",
            "passwd",
            "master.passwd",
            "sudoers",
            "htpasswd",
            "pgpass",
            "id_rsa",
            "id_dsa",
            "id_ecdsa",
            "id_ed25519",
            "netrc",
            "_netrc",
            "npmrc",
            "pypirc",
            "git-credentials",
            "gitconfig",
            "dockercfg",
        ];
        // Compare with and without a leading dot so that both `.npmrc` and
        // `npmrc` are covered.
        let bare_name = file_name.strip_prefix('.').unwrap_or(file_name);
        if CREDENTIAL_FILE_NAMES
            .iter()
            .any(|n| n.eq_ignore_ascii_case(file_name) || n.eq_ignore_ascii_case(bare_name))
        {
            return Err(ConfigError::InvalidValue {
                key: "file_path".to_string(),
                expected_type: "safe file path".to_string(),
                message: tr_args(
                    "error-path-credential-file-denied",
                    &[("name", file_name.to_string())],
                ),
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

    /// Maximum number of bytes read from a `_FILE`-referenced file.
    ///
    /// Mirrors the crate's default configuration-file size limit: a
    /// `_FILE` target must not be able to make the process allocate an
    /// unbounded buffer.
    pub const MAX_SECRET_FILE_SIZE: usize = 10 * 1024 * 1024;

    /// Validate `file_path` and read the secret it points to.
    ///
    /// This is the entry point callers should use: it applies the full
    /// [`Self::validate_and_resolve`] policy and then reads the file, so a
    /// secret can never be read from a path that was never validated.
    pub fn validate_and_read_secret_file(&self, file_path: &str) -> ConfigResult<String> {
        let canonical = self.validate_and_resolve(file_path)?;
        Self::read_secret_file(&canonical)
    }

    /// Read the content of an already-validated secret file.
    ///
    /// `path` must be the canonical path returned by
    /// [`Self::validate_and_resolve`]: opening the original path again
    /// would re-open the TOCTOU window the validation just closed.
    /// [`Self::validate_and_read_secret_file`] does both steps in one call.
    ///
    /// The read is capped at [`Self::MAX_SECRET_FILE_SIZE`]; a larger file
    /// yields [`ConfigError::SizeLimitExceeded`] instead of being buffered
    /// in full.
    pub fn read_secret_file(path: &Path) -> ConfigResult<String> {
        let file = std::fs::File::open(path).map_err(ConfigError::IoError)?;
        let metadata = file.metadata().map_err(ConfigError::IoError)?;
        if !metadata.is_file() {
            return Err(ConfigError::InvalidValue {
                key: "file_path".to_string(),
                expected_type: "regular file".to_string(),
                message: tr("error-path-not-regular-file"),
            });
        }
        // The open above follows a symlink: between the validation and the
        // open, the last path segment may have been replaced by one. The
        // path must still name the very file that was opened, so the link
        // is rejected and the file identity is compared.
        let current = std::fs::symlink_metadata(path).map_err(ConfigError::IoError)?;
        if !current.is_file() || !same_file(&metadata, &current) {
            return Err(ConfigError::InvalidValue {
                key: "file_path".to_string(),
                expected_type: "regular file".to_string(),
                message: tr("error-path-not-regular-file"),
            });
        }
        if metadata.len() > Self::MAX_SECRET_FILE_SIZE as u64 {
            return Err(ConfigError::SizeLimitExceeded {
                actual: usize::try_from(metadata.len()).unwrap_or(usize::MAX),
                limit: Self::MAX_SECRET_FILE_SIZE,
            });
        }
        use std::io::Read;
        let mut content = String::new();
        // `take(limit + 1)` bounds the allocation even when the file is
        // swapped for a larger one between the metadata check and the read.
        std::io::BufReader::new(file)
            .take(Self::MAX_SECRET_FILE_SIZE as u64 + 1)
            .read_to_string(&mut content)
            .map_err(ConfigError::IoError)?;
        if content.len() > Self::MAX_SECRET_FILE_SIZE {
            return Err(ConfigError::SizeLimitExceeded {
                actual: content.len(),
                limit: Self::MAX_SECRET_FILE_SIZE,
            });
        }
        Ok(content)
    }
}

/// Whether two metadata entries describe the same file on disk.
///
/// Used to detect a path segment that was replaced between the validation
/// and the open. The identity is the device/inode pair on unix; windows has
/// no stable file-id accessor in `std`, so the creation time, the last
/// write time and the size stand in for it.
#[cfg(unix)]
fn same_file(opened: &std::fs::Metadata, current: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    opened.dev() == current.dev() && opened.ino() == current.ino()
}

#[cfg(windows)]
fn same_file(opened: &std::fs::Metadata, current: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    opened.creation_time() == current.creation_time()
        && opened.last_write_time() == current.last_write_time()
        && opened.file_size() == current.file_size()
}

#[cfg(not(any(unix, windows)))]
fn same_file(_opened: &std::fs::Metadata, _current: &std::fs::Metadata) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Write `content` to `dir/name` and return the path.
    fn write_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, content).expect("write temp file");
        path
    }

    #[test]
    fn credential_file_names_are_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let validator = PathValidator::new();

        // The per-user credential files that carry no extension and would
        // therefore slip past the extension whitelist.
        for name in [
            ".netrc",
            ".npmrc",
            ".pypirc",
            ".git-credentials",
            "npmrc",
            "gitconfig",
        ] {
            let path = write_file(dir.path(), name, b"https://user:token@example.com");
            assert!(
                validator
                    .validate_and_resolve(path.to_str().unwrap())
                    .is_err(),
                "{name} must not be readable through a _FILE reference"
            );
        }
    }

    /// Whether a rendered message names `component` as a standalone token.
    ///
    /// A fixed list of directory names would only carry `docker` as part of
    /// `.docker`, so the token split is what distinguishes "names the
    /// rejected component" from "mentions some of them".
    fn names_component(message: &str, component: &str) -> bool {
        message
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '.' && c != '_' && c != '-')
            .any(|token| token == component)
    }

    #[test]
    fn credential_dir_denial_names_the_matched_component() {
        let dir = tempfile::tempdir().expect("tempdir");
        let validator = PathValidator::new();

        let mut messages = Vec::new();
        for &component in CREDENTIAL_DIR_COMPONENTS {
            let sub = dir.path().join(component);
            std::fs::create_dir(&sub).expect("create subdir");
            let path = write_file(&sub, "app.toml", b"value");

            let err = validator
                .validate_and_resolve(path.to_str().unwrap())
                .expect_err("a credential directory must be refused");
            let ConfigError::InvalidValue { message, .. } = &err else {
                panic!("expected InvalidValue, got {err:?}");
            };
            assert!(
                names_component(message, component),
                "the denial must name the matched component '{component}', got {message:?}"
            );
            assert!(
                !message.contains("{ $name }"),
                "the placeholder must be resolved, got {message:?}"
            );
            messages.push(message.clone());
        }

        // The message points at the cause, so two different components
        // cannot produce the same wording.
        assert_ne!(
            messages[0], messages[1],
            "the denial must not be a fixed wording: {messages:?}"
        );

        // Every blocked component must be enforced on its own: a component
        // that only ever appears in the shared list would leave the user
        // unable to tell which directory was refused.
        assert_eq!(
            messages.len(),
            CREDENTIAL_DIR_COMPONENTS.len(),
            "every blocked component must produce its own denial"
        );
    }

    #[test]
    fn dotfile_with_extension_is_allowed() {
        // A dot-file that carries an extension is not the credential-file
        // convention (e.g. `tempfile`'s ".tmpXXXX.txt"); only the
        // extension-less dot-files are refused.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_file(dir.path(), ".tmpsecret.txt", b"value");
        assert!(
            PathValidator::new()
                .validate_and_resolve(path.to_str().unwrap())
                .is_ok()
        );
    }

    #[test]
    fn ordinary_secret_file_is_allowed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_file(dir.path(), "db_password.txt", b"hunter2");
        assert!(
            PathValidator::new()
                .validate_and_resolve(path.to_str().unwrap())
                .is_ok()
        );
    }

    #[test]
    fn read_secret_file_enforces_size_cap() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_file(
            dir.path(),
            "big.txt",
            &vec![b'x'; PathValidator::MAX_SECRET_FILE_SIZE + 1],
        );
        let err = PathValidator::read_secret_file(&path).expect_err("oversized file must fail");
        assert!(
            matches!(err, ConfigError::SizeLimitExceeded { .. }),
            "expected SizeLimitExceeded, got {err:?}"
        );
    }

    #[test]
    fn read_secret_file_returns_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = write_file(dir.path(), "small.txt", b"secret-value");
        let content = PathValidator::read_secret_file(&path).expect("read");
        assert_eq!(content, "secret-value");
    }

    #[test]
    fn validate_and_read_secret_file_applies_the_policy() {
        let dir = tempfile::tempdir().expect("tempdir");
        let validator = PathValidator::new();

        let path = write_file(dir.path(), "db_password.txt", b"hunter2");
        let content = validator
            .validate_and_read_secret_file(path.to_str().unwrap())
            .expect("read");
        assert_eq!(content, "hunter2");

        // The policy is applied before the read: a credential file is
        // refused instead of being read.
        let denied = write_file(dir.path(), ".netrc", b"https://user:token@example.com");
        assert!(
            validator
                .validate_and_read_secret_file(denied.to_str().unwrap())
                .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn read_secret_file_rejects_a_symlinked_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = write_file(dir.path(), "real.txt", b"secret");
        let link = dir.path().join("link.txt");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        let err = PathValidator::read_secret_file(&link)
            .expect_err("a symlinked secret path must be refused");
        assert!(
            matches!(err, ConfigError::InvalidValue { .. }),
            "expected InvalidValue, got {err:?}"
        );
    }
}
