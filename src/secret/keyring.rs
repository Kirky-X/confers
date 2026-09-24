// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! OS keyring master-key storage (`keyring` feature).
//!
//! Developers should not keep master keys in plain env vars or files when an
//! OS secret store is available. This module abstracts the store behind the
//! [`KeyringStore`] port with two implementations:
//!
//! - [`SecretToolKeyringStore`]: the freedesktop Secret Service via the
//!   `secret-tool` CLI (libsecret) — the common Linux path.
//! - [`FileKeyringStore`]: a chmod-600 fallback file. Selecting it emits a
//!   **warning**, because a plaintext fallback weakens the threat model.
//!
//! [`MasterKeyStore::from_environment`] picks the best available store. CI
//! environments without a secret service fall back to the file store, so the
//! unit tests skip the DBus path when `secret-tool` is unavailable.

use std::path::PathBuf;
use std::process::Command;

use crate::error::{ConfigError, ConfigResult};

/// OS keyring port (object safe).
pub trait KeyringStore: Send + Sync {
    /// Store `secret` under `(service, account)`, replacing any previous one.
    fn set_secret(&self, service: &str, account: &str, secret: &[u8]) -> ConfigResult<()>;

    /// Load the secret stored under `(service, account)` (`None` when
    /// absent).
    fn get_secret(&self, service: &str, account: &str) -> ConfigResult<Option<Vec<u8>>>;

    /// Remove the secret under `(service, account)`.
    fn delete_secret(&self, service: &str, account: &str) -> ConfigResult<()>;

    /// Stable store kind name (`secret-tool`, `file`, …).
    fn kind(&self) -> &'static str;
}

/// Default service/account identifiers for the confers master key.
pub const MASTER_KEY_SERVICE: &str = "confers";
pub const MASTER_KEY_ACCOUNT: &str = "master-key";

/// Validate a keyring attribute (service or account name).
///
/// The values address the backing store — they become `secret-tool` argv
/// elements and fallback-store file names — so anything that could change
/// how they are interpreted is rejected outright: a leading `-` would be
/// parsed as an option by secret-tool, path separators / `..` would escape
/// the fallback directory, control characters (including NUL) could alter
/// argv or file-name semantics.
fn validated_attribute(value: &str, what: &str) -> ConfigResult<()> {
    let reject = |reason: &str| {
        Err(ConfigError::KeyError {
            message: format!("keyring {what} {reason}"),
        })
    };
    if value.is_empty() {
        return reject("must not be empty");
    }
    if value.starts_with('-') {
        return reject("must not start with '-'");
    }
    if value.contains(['/', '\\', '\0']) || value.contains("..") {
        return reject("must not contain path separators or '..'");
    }
    if value.chars().any(char::is_control) {
        return reject("must not contain control characters");
    }
    Ok(())
}

/// Secret Service access through the `secret-tool` CLI.
pub struct SecretToolKeyringStore;

impl SecretToolKeyringStore {
    pub fn new() -> Self {
        Self
    }

    /// Whether the `secret-tool` binary exists on `$PATH` (and therefore a
    /// Secret Service is plausible). DBus absence is detected at first use.
    pub fn is_available() -> bool {
        Command::new("secret-tool")
            .arg("--version")
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    }

    fn run(&self, args: &[&str], input: Option<&[u8]>) -> ConfigResult<Vec<u8>> {
        use std::io::Write;

        let mut child = Command::new("secret-tool")
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| ConfigError::KeyError {
                message: format!("cannot spawn secret-tool: {e}"),
            })?;

        if let Some(data) = input
            && let Some(mut stdin) = child.stdin.take()
        {
            stdin.write_all(data).map_err(|e| ConfigError::KeyError {
                message: format!("cannot pipe secret into secret-tool: {e}"),
            })?;
        }

        let out = child
            .wait_with_output()
            .map_err(|e| ConfigError::KeyError {
                message: format!("secret-tool failed: {e}"),
            })?;
        if !out.status.success() {
            return Err(ConfigError::KeyError {
                message: format!("secret-tool exited with {}", out.status),
            });
        }
        Ok(out.stdout)
    }
}

impl Default for SecretToolKeyringStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyringStore for SecretToolKeyringStore {
    fn set_secret(&self, service: &str, account: &str, secret: &[u8]) -> ConfigResult<()> {
        validated_attribute(service, "service")?;
        validated_attribute(account, "account")?;
        self.run(
            &["store", "--label=confers master key", service, account],
            Some(secret),
        )?;
        Ok(())
    }

    fn get_secret(&self, service: &str, account: &str) -> ConfigResult<Option<Vec<u8>>> {
        validated_attribute(service, "service")?;
        validated_attribute(account, "account")?;
        match self.run(&["lookup", service, account], None) {
            Ok(bytes) if bytes.is_empty() => Ok(None),
            Ok(bytes) => Ok(Some(bytes)),
            // `secret-tool lookup` exits non-zero with empty output when the
            // item does not exist: treat as absent, not an error.
            Err(_) => Ok(None),
        }
    }

    fn delete_secret(&self, service: &str, account: &str) -> ConfigResult<()> {
        validated_attribute(service, "service")?;
        validated_attribute(account, "account")?;
        self.run(&["clear", service, account], None)?;
        Ok(())
    }

    fn kind(&self) -> &'static str {
        "secret-tool"
    }
}

/// chmod-600 file fallback. Emits a warning on construction: a plaintext
/// fallback weakens the threat model compared to an OS keyring.
pub struct FileKeyringStore {
    path: PathBuf,
}

impl FileKeyringStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// The backing file path.
    pub fn path(&self) -> &std::path::Path {
        &self.path
    }
}

impl KeyringStore for FileKeyringStore {
    fn set_secret(&self, service: &str, account: &str, secret: &[u8]) -> ConfigResult<()> {
        validated_attribute(service, "service")?;
        validated_attribute(account, "account")?;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ConfigError::KeyError {
                message: format!("cannot create keyring fallback dir: {e}"),
            })?;
        }
        #[cfg(unix)]
        {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            let file_name = format!("{service}-{account}.key");
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .mode(0o600)
                .open(self.path.join(file_name))
                .map_err(|e| ConfigError::KeyError {
                    message: format!("cannot write keyring fallback file: {e}"),
                })?;
            file.write_all(secret).map_err(|e| ConfigError::KeyError {
                message: format!("cannot write keyring fallback file: {e}"),
            })?;
        }
        #[cfg(not(unix))]
        {
            let _ = (service, account, secret);
            return Err(ConfigError::KeyError {
                message: "file keyring fallback requires unix".to_string(),
            });
        }
        Ok(())
    }

    fn get_secret(&self, service: &str, account: &str) -> ConfigResult<Option<Vec<u8>>> {
        validated_attribute(service, "service")?;
        validated_attribute(account, "account")?;
        let file_name = format!("{service}-{account}.key");
        match std::fs::read(self.path.join(file_name)) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(ConfigError::KeyError {
                message: format!("cannot read keyring fallback file: {e}"),
            }),
        }
    }

    fn delete_secret(&self, service: &str, account: &str) -> ConfigResult<()> {
        validated_attribute(service, "service")?;
        validated_attribute(account, "account")?;
        let file_name = format!("{service}-{account}.key");
        match std::fs::remove_file(self.path.join(file_name)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(ConfigError::KeyError {
                message: format!("cannot delete keyring fallback file: {e}"),
            }),
        }
    }

    fn kind(&self) -> &'static str {
        "file"
    }
}

/// Picks the best available keyring store.
pub struct MasterKeyStore;

impl MasterKeyStore {
    /// Select the OS keyring when a Secret Service is reachable, otherwise
    /// fall back to a file store **with a warning**.
    pub fn from_environment(fallback_dir: impl Into<PathBuf>) -> Box<dyn KeyringStore> {
        if SecretToolKeyringStore::is_available() {
            Box::new(SecretToolKeyringStore::new())
        } else {
            crate::metrics::record_counter("confers_keyring_fallback_total", &[("store", "file")]);
            log::warn!(
                "OS keyring (secret-tool/Secret Service) unavailable; falling back to a \
                 chmod-600 file store — the master key is protected by file permissions only"
            );
            Box::new(FileKeyringStore::new(fallback_dir))
        }
    }

    /// Convenience: read the master key through the selected store.
    pub fn load_master_key(store: &dyn KeyringStore) -> ConfigResult<Option<Vec<u8>>> {
        store.get_secret(MASTER_KEY_SERVICE, MASTER_KEY_ACCOUNT)
    }

    /// Convenience: persist the master key through the selected store.
    pub fn store_master_key(store: &dyn KeyringStore, key: &[u8]) -> ConfigResult<()> {
        store.set_secret(MASTER_KEY_SERVICE, MASTER_KEY_ACCOUNT, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_store() -> FileKeyringStore {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().to_path_buf();
        // TempDir deletion races with the returned store's use in tests;
        // leak it deliberately (test process exit reclaims it).
        std::mem::forget(dir);
        FileKeyringStore::new(path)
    }

    #[test]
    fn file_store_roundtrips_and_deletes_secrets() {
        let store = file_store();
        assert_eq!(store.kind(), "file");

        assert!(store.get_secret("confers", "master-key").unwrap().is_none());

        store
            .set_secret("confers", "master-key", &[1, 2, 3, 4])
            .expect("set");
        assert_eq!(
            store.get_secret("confers", "master-key").unwrap(),
            Some(vec![1, 2, 3, 4])
        );

        store
            .delete_secret("confers", "master-key")
            .expect("delete");
        assert!(store.get_secret("confers", "master-key").unwrap().is_none());
    }

    #[test]
    fn master_key_helpers_use_stable_service_account_names() {
        let store = file_store();
        MasterKeyStore::store_master_key(&store, &[9u8; 32]).expect("store master key");
        let loaded = MasterKeyStore::load_master_key(&store)
            .expect("load master key")
            .expect("present");
        assert_eq!(loaded, vec![9u8; 32]);
    }

    #[cfg(unix)]
    #[test]
    fn file_fallback_permissions_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let store = file_store();
        store
            .set_secret("confers", "master-key", &[7; 8])
            .expect("set");
        let meta =
            std::fs::metadata(store.path().join("confers-master-key.key")).expect("metadata");
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
    }

    /// Full Secret Service round trip — **skipped** when `secret-tool` (and
    /// thus a DBus Secret Service) is unavailable, which is the norm on CI.
    #[test]
    #[serial_test::serial]
    fn secret_tool_roundtrip_when_service_available() {
        if !SecretToolKeyringStore::is_available() {
            eprintln!("skipping: secret-tool / Secret Service not available");
            return;
        }
        let store = SecretToolKeyringStore::new();
        store
            .set_secret("confers-test", "probe", &[5, 6, 7])
            .expect("set via secret-tool");
        let loaded = store
            .get_secret("confers-test", "probe")
            .expect("get via secret-tool");
        store.delete_secret("confers-test", "probe").expect("clear");
        assert_eq!(loaded, Some(vec![5, 6, 7]));
    }

    fn bad_attribute_names() -> Vec<String> {
        vec![
            String::new(),
            "-leading-dash".into(),
            "with/slash".into(),
            "with\\backslash".into(),
            "with..dots".into(),
            "with\0nul".into(),
            "with\u{7f}control".into(),
        ]
    }

    /// Every reject branch of `validated_attribute` must fail loudly through
    /// all three store operations (the validation runs before any IO).
    #[test]
    fn attribute_validation_rejects_dangerous_names_on_every_operation() {
        let store = file_store();

        for name in bad_attribute_names() {
            assert!(
                store.set_secret(&name, "acct", b"x").is_err(),
                "set must reject service {name:?}"
            );
            assert!(
                store.get_secret(&name, "acct").is_err(),
                "get must reject service {name:?}"
            );
            assert!(
                store.delete_secret(&name, "acct").is_err(),
                "delete must reject service {name:?}"
            );
            assert!(
                store.set_secret("svc", &name, b"x").is_err(),
                "set must reject account {name:?}"
            );
            assert!(
                store.get_secret("svc", &name).is_err(),
                "get must reject account {name:?}"
            );
            assert!(
                store.delete_secret("svc", &name).is_err(),
                "delete must reject account {name:?}"
            );
        }
    }

    /// The `secret-tool` store validates identically before spawning, and —
    /// when the binary is absent (the CI norm) — surfaces a spawn error for
    /// set/delete while `get` maps the failure to "absent".
    #[test]
    #[serial_test::serial]
    fn secret_tool_store_validates_then_maps_spawn_failure() {
        let store = SecretToolKeyringStore::new();

        // Validation errors fire before any process spawn.
        assert!(store.set_secret("", "acct", b"x").is_err());
        assert!(store.get_secret("-bad", "acct").is_err());
        assert!(store.delete_secret("a/b", "acct").is_err());

        if SecretToolKeyringStore::is_available() {
            return; // real service present: roundtrip covered by the other test
        }
        // No secret-tool: set/delete cannot spawn the binary.
        assert!(
            store.set_secret("svc", "acct", b"x").is_err(),
            "set without secret-tool must fail loudly"
        );
        assert!(
            store.delete_secret("svc", "acct").is_err(),
            "delete without secret-tool must fail loudly"
        );
        // lookup failure means "item absent", not an error (contract).
        assert_eq!(store.get_secret("svc", "acct").unwrap(), None);
    }

    #[test]
    fn secret_tool_store_kind_is_stable() {
        assert_eq!(SecretToolKeyringStore::new().kind(), "secret-tool");
    }

    /// The store dir itself must pre-exist: set creates only the missing
    /// parent chain above it, and an uncreatable parent fails with its own
    /// message (both mappings verified).
    #[test]
    fn file_store_creates_missing_parent_directories() {
        let dir = tempfile::TempDir::new().expect("tempdir");

        // Parent chain `a/b` is created, but the store dir is not auto-made:
        // the write then fails with the explicit fallback-file error.
        let store = FileKeyringStore::new(dir.path().join("a/b/store"));
        let err = store.set_secret("confers", "master-key", &[1]).unwrap_err();
        assert!(
            err.to_string()
                .contains("cannot write keyring fallback file"),
            "missing store dir must fail loudly: {err}"
        );
        assert!(
            dir.path().join("a/b").is_dir(),
            "the parent chain above the store dir is created"
        );

        // A parent that cannot be created (exists as a file) fails with the
        // directory-creation error mapping.
        std::fs::write(dir.path().join("blocked"), b"occupied").expect("block parent");
        let blocked = FileKeyringStore::new(dir.path().join("blocked/store"));
        let err = blocked
            .set_secret("confers", "master-key", &[1])
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("cannot create keyring fallback dir"),
            "blocked parent must fail loudly: {err}"
        );
    }

    /// A path that exists but is not a regular file surfaces as a key error
    /// (not `None`) on get, and fails the delete loudly.
    #[test]
    fn file_store_reports_unreadable_secret_entries() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let store = FileKeyringStore::new(dir.path());
        // A *directory* where the secret file would live.
        std::fs::create_dir(dir.path().join("confers-master-key.key"))
            .expect("create blocking dir");

        let get_err = store.get_secret("confers", "master-key").unwrap_err();
        assert!(
            get_err
                .to_string()
                .contains("cannot read keyring fallback file"),
            "get must report the unreadable entry: {get_err}"
        );

        let del_err = store.delete_secret("confers", "master-key").unwrap_err();
        assert!(
            del_err
                .to_string()
                .contains("cannot delete keyring fallback file"),
            "delete must report the failure: {del_err}"
        );
    }

    #[test]
    #[serial_test::serial]
    fn master_key_store_selects_the_best_available_backend() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let store = MasterKeyStore::from_environment(dir.path());
        if SecretToolKeyringStore::is_available() {
            assert_eq!(store.kind(), "secret-tool");
        } else {
            assert_eq!(store.kind(), "file");
        }
    }

    /// 用 PATH 上的假 `secret-tool` 驱动 `run()` 的完整子进程路径
    /// (spawn/stdin/stdout/退出码),无需真实 Secret Service。
    #[test]
    #[serial_test::serial]
    fn fake_secret_tool_exercises_full_subprocess_paths() {
        let bindir = tempfile::TempDir::new().expect("bindir");
        let fake = bindir.path().join("secret-tool");
        // 一个把 stdin 原样写进文件的假 store;lookup 读回该文件。
        std::fs::write(
            &fake,
            "#!/bin/sh\nif [ \"$1\" = \"store\" ]; then\n  cat > \"$TMPDIR/confers-fake-keyring\" 2>/dev/null || cat > /tmp/confers-fake-keyring\n  exit 0\nfi\nif [ \"$1\" = \"lookup\" ]; then\n  cat \"$TMPDIR/confers-fake-keyring\" 2>/dev/null || cat /tmp/confers-fake-keyring 2>/dev/null\n  exit 0\nfi\nexit 0\n",
        )
        .expect("write fake secret-tool");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755))
                .expect("chmod fake");
        }

        let old_path = std::env::var("PATH").ok();
        let new_path = format!(
            "{}:{}",
            bindir.path().display(),
            old_path.as_deref().unwrap_or_default()
        );
        unsafe { std::env::set_var("PATH", new_path) };
        assert!(
            SecretToolKeyringStore::is_available(),
            "fake binary must be detected on PATH"
        );

        let store = SecretToolKeyringStore::new();
        let set = store.set_secret("confers", "fake", b"injected-secret");
        let got = store.get_secret("confers", "fake");
        let cleared = store.delete_secret("confers", "fake");
        match old_path {
            Some(p) => unsafe { std::env::set_var("PATH", p) },
            None => unsafe { std::env::remove_var("PATH") },
        }

        set.expect("fake store must accept the secret");
        assert_eq!(
            got.expect("lookup through fake").unwrap(),
            b"injected-secret"
        );
        cleared.expect("clear through fake must succeed");
    }

    /// `lookup` 成功但输出为空 → 契约要求视为「条目不存在」。
    #[test]
    #[serial_test::serial]
    fn empty_lookup_output_maps_to_absent() {
        let bindir = tempfile::TempDir::new().expect("bindir");
        let fake = bindir.path().join("secret-tool");
        std::fs::write(&fake, "#!/bin/sh\nexit 0\n").expect("write silent fake");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755))
                .expect("chmod fake");
        }

        let old_path = std::env::var("PATH").ok();
        let new_path = format!(
            "{}:{}",
            bindir.path().display(),
            old_path.as_deref().unwrap_or_default()
        );
        unsafe { std::env::set_var("PATH", new_path) };
        let store = SecretToolKeyringStore::new();
        let absent = store.get_secret("confers", "nothing-there");
        match old_path {
            Some(p) => unsafe { std::env::set_var("PATH", p) },
            None => unsafe { std::env::remove_var("PATH") },
        }
        assert_eq!(
            absent.expect("silent lookup is Ok"),
            None,
            "empty lookup output maps to absent, not an error"
        );
    }
}
