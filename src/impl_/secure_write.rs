// Copyright (c) 2026 Kirky.X🌠
// SPDX-License-Identifier: MIT

//! Owner-only file writes for payloads that may carry credentials.

use std::io::Write;
use std::path::{Path, PathBuf};

/// Unix file mode for files that may contain sensitive configuration:
/// owner-only (0600) instead of the process-default 0644.
#[cfg(unix)]
const SENSITIVE_FILE_MODE: u32 = 0o600;

/// Sibling temp-file path for `path` (same directory → same filesystem, so
/// the final rename is atomic).
pub(crate) fn sibling_tmp_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

/// Write `content` to `path` atomically, owner-only on unix.
///
/// The data is written to a sibling temp file, fsynced, and then renamed over
/// the target. A crash mid-write leaves at most a `.tmp` sibling — never a
/// truncated/half-written target file. The temp file is removed when the
/// write itself fails.
///
/// On unix the file is created with mode 0600: the payload may contain
/// credentials, and the process-default 0644 would let every local user
/// read them. The temp file is created exclusively, so a leftover `.tmp`
/// sibling (or a symlink planted in its place) is never written into.
pub(crate) fn write_owner_only(path: &Path, content: &str) -> std::io::Result<()> {
    let tmp_path = sibling_tmp_path(path);

    let write_result: std::io::Result<()> = (|| {
        let mut file = create_temp_file(&tmp_path)?;
        file.write_all(content.as_bytes())?;
        // fsync so the data is on disk before the rename makes it visible.
        file.sync_all()?;
        Ok(())
    })();

    if let Err(e) = write_result {
        // Clean up the temp file so failed writes leave no residue.
        let _ = std::fs::remove_file(&tmp_path);
        return Err(e);
    }

    // The rename can fail too (e.g. a directory in the way) — clean up the
    // temp file so no `*.tmp` residue accumulates.
    match std::fs::rename(&tmp_path, path) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            Err(e)
        }
    }
}

/// Create the temp file the payload is written to.
///
/// The file is created exclusively: `OpenOptions::mode` only applies to a
/// file this call creates, so a pre-existing `.tmp` sibling (left behind by
/// a crash, or planted in a shared directory) would keep its own — possibly
/// world-readable — permissions. A leftover file is removed first, and the
/// exclusive create refuses to follow a symlink planted in its place.
#[cfg(unix)]
fn create_temp_file(tmp_path: &Path) -> std::io::Result<std::fs::File> {
    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;

    let open = || {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(SENSITIVE_FILE_MODE)
            .open(tmp_path)
    };
    match open() {
        Ok(file) => Ok(file),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let _ = std::fs::remove_file(tmp_path);
            open()
        }
        Err(e) => Err(e),
    }
}

#[cfg(not(unix))]
fn create_temp_file(tmp_path: &Path) -> std::io::Result<std::fs::File> {
    use std::fs::OpenOptions;

    let open = || {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(tmp_path)
    };
    match open() {
        Ok(file) => Ok(file),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            let _ = std::fs::remove_file(tmp_path);
            open()
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_the_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("out.json");
        write_owner_only(&target, "{\"a\":1}").expect("write");
        assert_eq!(std::fs::read_to_string(&target).expect("read"), "{\"a\":1}");
        // The temp sibling must not survive a successful write.
        assert!(!sibling_tmp_path(&target).exists());
    }

    #[test]
    fn overwrites_an_existing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("out.json");
        std::fs::write(&target, "old").expect("seed");
        write_owner_only(&target, "new").expect("write");
        assert_eq!(std::fs::read_to_string(&target).expect("read"), "new");
    }

    #[cfg(unix)]
    #[test]
    fn creates_the_file_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("out.json");
        write_owner_only(&target, "secret").expect("write");
        let mode = std::fs::metadata(&target)
            .expect("stat")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "the file must be owner-only");
    }

    #[cfg(unix)]
    #[test]
    fn replaces_a_pre_existing_temp_sibling() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("out.json");
        // A leftover temp sibling (crash residue, or a file planted in a
        // shared directory) must not receive the payload under its own
        // permissions.
        let tmp = sibling_tmp_path(&target);
        std::fs::write(&tmp, "stale").expect("seed");
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o666)).expect("chmod");

        write_owner_only(&target, "secret").expect("write");
        assert_eq!(std::fs::read_to_string(&target).expect("read"), "secret");
        let mode = std::fs::metadata(&target)
            .expect("stat")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, "the file must be owner-only");
    }
}
