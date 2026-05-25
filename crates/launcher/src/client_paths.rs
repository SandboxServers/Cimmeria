//! User-data paths the SGW client writes to outside the install directory.
//!
//! The 2009 client unconditionally writes its mutable runtime state under
//! `%USERPROFILE%\Documents\My Games\Firesky\SGWGame\`. The
//! `Cache.en-US/` subdirectory holds **server-pushed PAK overrides** —
//! and crucially the client consults that cache *before* the bundled
//! PAKs in the install directory ([`docs/architecture/mission-pak-overrides.md`]).
//! That means a stale cache from a previous shard silently shadows the
//! launcher-managed PAKs until it's wiped.
//!
//! The launcher exposes two reset buttons backed by this module:
//!
//! 1. **Reset client cache** — wipes only `Cache.en-US/`. Safe whenever
//!    you switch servers, pull a new manifest, or want the launcher's
//!    PAKs to win for sure.
//! 2. **Reset all client state** — wipes the entire `Firesky/` tree.
//!    This is the recipe from [`docs/client-tools.md`] for recovering
//!    from cache corruption (ASEditor can poison the cooked-data cache).
//!    User-facing confirm dialog gated.
//!
//! Both functions are best-effort: missing directories are treated as
//! "already clean" (`Ok(0)`), not an error. IO failures bubble up so the
//! UI can surface what went wrong.
//!
//! The full path is computed from `USERPROFILE` (Windows) or `HOME`
//! (Unix). The Unix fallback is for unit tests on the CI Ubuntu runner
//! and Linux dev hosts; the real client only ever runs on Windows.

use std::io;
use std::path::{Path, PathBuf};

/// Subpath under `Documents\My Games\` that the client owns. Sourced
/// from the reverse-engineered launcher binary
/// ([`docs/reverse-engineering/binaries/launcher-exe.md`]) — the original
/// CME launcher appends this exact string to `My Documents` before
/// handing the path to SGW.exe.
pub const FIRESKY_SUBPATH: &str = "My Games/Firesky";

/// Sub-sub-path under [`firesky_root`] that holds the writable
/// runtime cache the server pushes PAK overrides into.
const SGWGAME_CACHE_SUBPATH: &str = "SGWGame/Cache.en-US";

/// Resolve `%USERPROFILE%\Documents\My Games\Firesky` (Windows) or
/// `$HOME/Documents/My Games/Firesky` (Unix fallback for tests).
///
/// Returns `None` only when neither `USERPROFILE` nor `HOME` is set —
/// extremely degenerate, but possible in restricted CI environments. The
/// caller should surface "couldn't find your user profile" rather than
/// blindly wipe `Documents/My Games/Firesky` relative to the cwd.
pub fn firesky_root() -> Option<PathBuf> {
    user_documents().map(|d| d.join(FIRESKY_SUBPATH))
}

/// Resolve the writable PAK-override cache directory the server pushes
/// into:
/// `%USERPROFILE%\Documents\My Games\Firesky\SGWGame\Cache.en-US`.
pub fn cache_dir() -> Option<PathBuf> {
    firesky_root().map(|r| r.join(SGWGAME_CACHE_SUBPATH))
}

/// Resolve the per-user `Documents` directory.
///
/// Windows: `%USERPROFILE%\Documents`. We deliberately don't go through
/// `SHGetKnownFolderPath(FOLDERID_Documents)` here because the 2009
/// client itself uses the simpler `USERPROFILE + Documents` path
/// ([`docs/reverse-engineering/binaries/launcher-exe.md`]) — matching
/// that exactly is the whole point of this module, even if the user
/// has redirected their Documents folder elsewhere via the Shell API.
/// If the client is writing to `%USERPROFILE%\Documents\...` and the
/// launcher wipes `<redirected>\...`, we wipe the wrong tree.
fn user_documents() -> Option<PathBuf> {
    if let Ok(profile) = std::env::var("USERPROFILE") {
        return Some(PathBuf::from(profile).join("Documents"));
    }
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join("Documents"));
    }
    None
}

/// Summary of a wipe operation: how many top-level entries were removed
/// and how many bytes those entries occupied. Bytes are best-effort
/// (zero if the dir-walk fails) and exist mainly so the UI status line
/// can say "Cleared 124 MB of cached PAK overrides" instead of just
/// "done."
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WipeReport {
    pub entries_removed: usize,
    pub bytes_freed: u64,
}

/// Remove every entry inside `dir` (preserving the directory itself so a
/// running client doesn't lose its watch handle). Returns `Ok(WipeReport::default())`
/// when `dir` doesn't exist — treating "already clean" as success keeps
/// the calling UI flow trivial.
pub fn wipe_dir_contents(dir: &Path) -> io::Result<WipeReport> {
    if !dir.exists() {
        return Ok(WipeReport::default());
    }
    let mut report = WipeReport::default();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        report.bytes_freed += dir_size_best_effort(&path);
        if entry.file_type()?.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
        report.entries_removed += 1;
    }
    Ok(report)
}

/// Recursively sum file sizes under `path`. Errors are silently treated
/// as 0 — this is presentation-layer fluff, not load-bearing accounting.
fn dir_size_best_effort(path: &Path) -> u64 {
    fn walk(p: &Path) -> u64 {
        let Ok(meta) = std::fs::symlink_metadata(p) else {
            return 0;
        };
        if meta.is_file() {
            return meta.len();
        }
        if !meta.is_dir() {
            return 0;
        }
        let Ok(rd) = std::fs::read_dir(p) else {
            return 0;
        };
        let mut acc = 0u64;
        for e in rd.flatten() {
            acc = acc.saturating_add(walk(&e.path()));
        }
        acc
    }
    walk(path)
}

/// Process-wide serialization for tests that mutate `USERPROFILE` /
/// `HOME` — `std::env::set_var` is process-global and cargo runs tests
/// multi-threaded by default, so without this lock two env-mutating
/// tests can trash each other's expected values mid-assertion. Shared
/// between the unit tests in this file and `worker::tests::spawn_wipe_*`.
///
/// `#[cfg(test)]` so it doesn't leak into release builds, but
/// `pub(crate)` so siblings can lock it.
#[cfg(test)]
pub(crate) fn env_test_lock() -> &'static std::sync::Mutex<()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    // Real wipe + report path: a populated nested tree gets fully removed,
    // and the report counts top-level entries (not recursive file count)
    // because that's what makes sense to surface as "X items removed."
    #[test]
    fn wipe_dir_contents_removes_files_and_subdirs() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        fs::create_dir_all(dir.path().join("nested/sub")).unwrap();
        fs::write(dir.path().join("nested/sub/deep.txt"), b"world!").unwrap();

        let report = wipe_dir_contents(dir.path()).unwrap();
        assert_eq!(report.entries_removed, 2, "a.txt + nested/");
        assert!(report.bytes_freed >= 11, "5 + 6 byte payloads");
        // The directory itself survives; only its contents go.
        assert!(dir.path().exists());
        assert!(fs::read_dir(dir.path()).unwrap().next().is_none());
    }

    // Missing path is "already clean" — important so the UI doesn't
    // error on a fresh install that's never run the client.
    #[test]
    fn wipe_dir_contents_handles_missing_path_as_success() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("never-existed");
        let report = wipe_dir_contents(&missing).unwrap();
        assert_eq!(report, WipeReport::default());
    }

    #[test]
    fn wipe_dir_contents_on_empty_dir_is_zero_report() {
        let dir = tempfile::tempdir().unwrap();
        let report = wipe_dir_contents(dir.path()).unwrap();
        assert_eq!(report, WipeReport::default());
    }

    // Resolution path: with USERPROFILE set, firesky_root + cache_dir
    // must compose to the exact tree the 2009 client writes
    // (Documents\My Games\Firesky\SGWGame\Cache.en-US).
    #[test]
    fn firesky_root_uses_userprofile_when_set() {
        let _g = env_test_lock().lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let prev = std::env::var("USERPROFILE").ok();
        let prev_home = std::env::var("HOME").ok();
        std::env::set_var("USERPROFILE", dir.path());
        // Force the USERPROFILE branch even on Unix CI by removing HOME.
        std::env::remove_var("HOME");

        let root = firesky_root().expect("USERPROFILE was set");
        assert!(
            root.ends_with("Documents/My Games/Firesky")
                || root.ends_with("Documents\\My Games\\Firesky")
        );
        let cache = cache_dir().expect("USERPROFILE was set");
        assert!(cache.ends_with("Cache.en-US"));

        // Restore.
        match prev {
            Some(v) => std::env::set_var("USERPROFILE", v),
            None => std::env::remove_var("USERPROFILE"),
        }
        if let Some(v) = prev_home {
            std::env::set_var("HOME", v);
        }
    }

    #[test]
    fn firesky_root_falls_back_to_home_when_userprofile_unset() {
        let _g = env_test_lock().lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let prev_userprofile = std::env::var("USERPROFILE").ok();
        let prev_home = std::env::var("HOME").ok();
        std::env::remove_var("USERPROFILE");
        std::env::set_var("HOME", dir.path());

        let root = firesky_root().expect("HOME was set");
        assert!(root.starts_with(dir.path()));

        match prev_userprofile {
            Some(v) => std::env::set_var("USERPROFILE", v),
            None => std::env::remove_var("USERPROFILE"),
        }
        match prev_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
    }
}
