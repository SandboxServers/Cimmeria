//! 32-bit artifacts the launcher carries inside itself and writes to disk
//! at launch: the `cimmeria-client-patches.dll` it injects, the
//! `cimmeria-client-telemetry.dll` it injects for players who opted in to
//! telemetry, and the `sgw-start32.exe` helper that does the injecting.
//! The release workflow builds each for i686 and embeds it (`build.rs`).
//!
//! - A DLL is loaded by the game for as long as it runs, so it goes to
//!   `<launcher dir>/<subdir>/<sha256 prefix>/` ([`find`]): a new launcher
//!   writes a new directory rather than overwrite a DLL a running game
//!   holds, and older directories are pruned once nothing holds them.
//! - The helper runs only for a moment, and antivirus exclusions want one
//!   stable path, so it is kept at a fixed name beside the launcher
//!   ([`write_stable`]).
//!
//! A dev build embeds nothing and uses a copy beside itself.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Where an artifact was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// Written out from the copy embedded in this launcher.
    Bundled(PathBuf),
    /// `<launcher dir>/<file name>` (dev builds).
    BesideLauncher(PathBuf),
}

// Callers match on the variant; the tests only need the path.
#[cfg(test)]
impl Found {
    pub fn path(&self) -> &Path {
        match self {
            Self::Bundled(p) | Self::BesideLauncher(p) => p,
        }
    }
}

/// Find `file_name`: the embedded copy first (written under
/// `<launcher_dir>/<subdir>/`), else one beside the launcher. `Ok(None)`
/// when neither exists.
pub fn find(
    embedded: &[u8],
    subdir: &str,
    file_name: &str,
    launcher_dir: &Path,
) -> std::io::Result<Option<Found>> {
    if !embedded.is_empty() {
        let path = write_bundled(embedded, &launcher_dir.join(subdir), file_name)?;
        return Ok(Some(Found::Bundled(path)));
    }
    let beside = launcher_dir.join(file_name);
    Ok(beside.is_file().then_some(Found::BesideLauncher(beside)))
}

/// Keep `path` equal to `bytes`: reuse an intact file, else write a temp
/// file beside it and rename it over. For the `sgw-start32` helper, which
/// must keep one stable path beside the launcher (never `%TEMP%`, never a
/// per-version directory) so an antivirus exclusion for it keeps working
/// across updates. Safe to overwrite: the helper only runs for the moment
/// it takes to start a game.
pub fn write_stable(bytes: &[u8], path: &Path) -> std::io::Result<()> {
    if std::fs::read(path).is_ok_and(|on_disk| on_disk == bytes) {
        return Ok(());
    }
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = path.with_file_name(format!("{file_name}.tmp"));
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

/// Write `bytes` to `<root>/<sha256 prefix>/<file_name>`, reusing an intact
/// existing copy, and prune the other versions under `root`.
pub fn write_bundled(bytes: &[u8], root: &Path, file_name: &str) -> std::io::Result<PathBuf> {
    let digest = hex(&Sha256::digest(bytes));
    let version_dir = root.join(&digest[..16]);
    let path = version_dir.join(file_name);
    let intact = std::fs::read(&path).is_ok_and(|on_disk| on_disk == bytes);
    if !intact {
        std::fs::create_dir_all(&version_dir)?;
        // Write then rename, so a crash mid-write never leaves a truncated
        // file at the path the next launch trusts.
        let tmp = version_dir.join(format!("{file_name}.tmp"));
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &path)?;
    }
    prune_other_versions(root, &version_dir);
    Ok(path)
}

/// Remove every version directory but `keep`. Best effort: a file a running
/// game still holds cannot be deleted, and is left for a later launch.
fn prune_other_versions(root: &Path, keep: &Path) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path != keep && path.is_dir() {
            if let Err(e) = std::fs::remove_dir_all(&path) {
                tracing::debug!(
                    path = %path.display(),
                    error = %e,
                    "old bundled artifact not pruned (probably still in use)"
                );
            }
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_bytes_are_written_under_a_content_addressed_dir() {
        let dir = tempfile::tempdir().unwrap();
        let found = find(b"MZv1", "sub", "a.dll", dir.path()).unwrap().unwrap();
        let Found::Bundled(path) = &found else {
            panic!("expected Bundled, got {found:?}");
        };
        assert_eq!(std::fs::read(path).unwrap(), b"MZv1");
        assert_eq!(path.file_name().unwrap(), "a.dll");
        assert!(path.starts_with(dir.path().join("sub")));
        // Same bytes again: same path, file reused.
        assert_eq!(
            find(b"MZv1", "sub", "a.dll", dir.path()).unwrap().unwrap(),
            found
        );
    }

    /// A new launcher version writes a new directory rather than overwrite
    /// a file a running game may hold, and prunes the old one.
    #[test]
    fn a_new_version_gets_a_new_dir_and_prunes_the_old() {
        let dir = tempfile::tempdir().unwrap();
        let v1 = find(b"MZv1", "sub", "a.dll", dir.path()).unwrap().unwrap();
        let v2 = find(b"MZv2", "sub", "a.dll", dir.path()).unwrap().unwrap();
        assert_ne!(v1.path().parent(), v2.path().parent());
        assert!(!v1.path().exists(), "old version should be pruned");
        assert_eq!(std::fs::read(v2.path()).unwrap(), b"MZv2");
    }

    #[test]
    fn a_corrupted_copy_is_rewritten() {
        let dir = tempfile::tempdir().unwrap();
        let found = find(b"MZgood", "sub", "a.dll", dir.path())
            .unwrap()
            .unwrap();
        std::fs::write(found.path(), b"MZbad").unwrap();
        find(b"MZgood", "sub", "a.dll", dir.path()).unwrap();
        assert_eq!(std::fs::read(found.path()).unwrap(), b"MZgood");
    }

    #[test]
    fn a_dev_build_uses_the_copy_beside_the_launcher() {
        let dir = tempfile::tempdir().unwrap();
        let beside = dir.path().join("a.dll");
        std::fs::write(&beside, b"MZ").unwrap();
        assert_eq!(
            find(b"", "sub", "a.dll", dir.path()).unwrap(),
            Some(Found::BesideLauncher(beside))
        );
    }

    #[test]
    fn nothing_embedded_and_nothing_beside_is_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(find(b"", "sub", "a.dll", dir.path()).unwrap(), None);
    }

    #[test]
    fn write_stable_writes_reuses_and_replaces_in_place() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("h.exe");
        write_stable(b"MZv1", &path).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"MZv1");
        write_stable(b"MZv1", &path).unwrap();
        write_stable(b"MZv2", &path).unwrap();
        assert_eq!(
            std::fs::read(&path).unwrap(),
            b"MZv2",
            "same path, new bytes"
        );
        assert!(!dir.path().join("h.exe.tmp").exists());
    }

    /// Two artifacts under different subdirs do not prune each other.
    #[test]
    fn artifacts_in_different_subdirs_coexist() {
        let dir = tempfile::tempdir().unwrap();
        let dll = find(b"MZdll", "client-patches", "p.dll", dir.path())
            .unwrap()
            .unwrap();
        let exe = find(b"MZexe", "start32", "h.exe", dir.path())
            .unwrap()
            .unwrap();
        assert!(dll.path().exists() && exe.path().exists());
    }
}
