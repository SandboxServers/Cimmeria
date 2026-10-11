//! User-data paths the SGW client writes to outside the install directory.
//!
//! The 2009 client unconditionally writes its mutable runtime state under
//! `<Documents>\My Games\Firesky\SGWGame\`. The
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
//! "already clean" (`Ok(WipeReport::default())`), not an error. IO
//! failures bubble up so the UI can surface what went wrong.
//!
//! `Documents` is the folder the client itself asks the shell for
//! (`SHGetFolderPathW(CSIDL_PERSONAL)`), so a redirected (OneDrive)
//! Documents is followed. `USERPROFILE\Documents` and then
//! `$HOME/Documents` are fallbacks only; off Windows nothing else exists,
//! and the desktop launcher refuses to act there (the Wine client writes
//! inside its prefix).

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

/// Resolve `<Documents>\My Games\Firesky`, `<Documents>` being the
/// folder [`user_documents`] finds.
///
/// Returns `None` only when no Documents folder can be found — extremely
/// degenerate, but possible in restricted CI environments. The caller
/// should surface "couldn't find your user profile" rather than blindly
/// wipe `Documents/My Games/Firesky` relative to the cwd.
pub fn firesky_root() -> Option<PathBuf> {
    user_documents().map(|d| d.join(FIRESKY_SUBPATH))
}

/// Resolve the writable PAK-override cache directory the server pushes
/// into: `<Documents>\My Games\Firesky\SGWGame\Cache.en-US`.
pub fn cache_dir() -> Option<PathBuf> {
    firesky_root().map(|r| r.join(SGWGAME_CACHE_SUBPATH))
}

/// Resolve the per-user `Documents` directory the client writes under.
///
/// SGW.exe asks the shell: `SHGetFolderPathW(CSIDL_PERSONAL)`
/// ([`docs/reverse-engineering/findings/multi-client-lab.md`]), which
/// follows a Documents folder redirected elsewhere (OneDrive does this).
/// Joining `%USERPROFILE%\Documents` instead would wipe a tree the client
/// never reads on such a machine, so that join is only the fallback for
/// when the shell call fails.
fn user_documents() -> Option<PathBuf> {
    #[cfg(any(test, feature = "test-fixtures"))]
    if let Some(documents) = test_documents::current() {
        return documents;
    }
    resolve_documents(shell_documents(), |name| std::env::var(name).ok())
}

/// [`user_documents`] with its two sources passed in: the shell's answer
/// (`None` when it failed, or off Windows) and the environment.
fn resolve_documents(
    shell: Option<PathBuf>,
    env: impl Fn(&str) -> Option<String>,
) -> Option<PathBuf> {
    if let Some(documents) = shell.filter(|path| !path.as_os_str().is_empty()) {
        return Some(documents);
    }
    // Treat empty values as unset — an empty USERPROFILE would join
    // to "Documents" relative to cwd and the wipe commands would
    // target the wrong tree.
    ["USERPROFILE", "HOME"]
        .into_iter()
        .find_map(|name| env(name).filter(|value| !value.is_empty()))
        .map(|profile| PathBuf::from(profile).join("Documents"))
}

/// The client's own lookup: `SHGetFolderPathW(CSIDL_PERSONAL,
/// SHGFP_TYPE_CURRENT)`.
#[cfg(windows)]
fn shell_documents() -> Option<PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::Foundation::MAX_PATH;
    use windows_sys::Win32::UI::Shell::{SHGetFolderPathW, CSIDL_PERSONAL, SHGFP_TYPE_CURRENT};
    let mut buffer = [0u16; MAX_PATH as usize];
    // SAFETY: `buffer` holds MAX_PATH UTF-16 units, the size the call
    // requires; the window and token handles may be null.
    let hr = unsafe {
        SHGetFolderPathW(
            std::ptr::null_mut(),
            CSIDL_PERSONAL as i32,
            std::ptr::null_mut(),
            SHGFP_TYPE_CURRENT as u32,
            buffer.as_mut_ptr(),
        )
    };
    if hr < 0 {
        tracing::warn!(
            hr,
            "SHGetFolderPathW(CSIDL_PERSONAL) failed; using USERPROFILE"
        );
        return None;
    }
    let len = buffer
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(buffer.len());
    Some(PathBuf::from(std::ffi::OsString::from_wide(&buffer[..len])))
}

#[cfg(not(windows))]
fn shell_documents() -> Option<PathBuf> {
    None
}

/// Replace [`user_documents`]' answer in tests, so a test that wipes the
/// client folder never reaches the player's real Documents (the shell call
/// ignores `USERPROFILE` on a redirected machine).
#[cfg(any(test, feature = "test-fixtures"))]
pub mod test_documents {
    use std::path::PathBuf;
    use std::sync::{Mutex, MutexGuard};

    static DOCUMENTS: Mutex<Option<Option<PathBuf>>> = Mutex::new(None);

    /// While held, [`super::firesky_root`] resolves under `documents`, or to
    /// `None` when `documents` is `None`. Holds [`super::env_test_lock`], so
    /// two such tests never overlap.
    pub struct Override {
        _lock: MutexGuard<'static, ()>,
    }

    pub fn set(documents: Option<PathBuf>) -> Override {
        let lock = super::env_test_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *DOCUMENTS.lock().unwrap_or_else(|p| p.into_inner()) = Some(documents);
        Override { _lock: lock }
    }

    impl Drop for Override {
        fn drop(&mut self) {
            *DOCUMENTS.lock().unwrap_or_else(|p| p.into_inner()) = None;
        }
    }

    pub(super) fn current() -> Option<Option<PathBuf>> {
        DOCUMENTS.lock().unwrap_or_else(|p| p.into_inner()).clone()
    }
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
/// tests can trash each other's expected values mid-assertion.
/// [`test_documents::set`] holds it for as long as its override lasts.
///
/// Test-only so it doesn't leak into release builds; the `test-fixtures`
/// feature exposes it to other crates' tests, which take it too.
#[cfg(any(test, feature = "test-fixtures"))]
pub fn env_test_lock() -> &'static std::sync::Mutex<()> {
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

    /// An environment holding only the given variables.
    fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name| {
            vars.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.to_string())
        }
    }

    // A redirected Documents (OneDrive) is where the client writes, so the
    // shell's answer wins over USERPROFILE\Documents.
    #[test]
    fn a_redirected_documents_folder_wins_over_userprofile() {
        let redirected = PathBuf::from("D:/OneDrive/Documents");
        assert_eq!(
            resolve_documents(
                Some(redirected.clone()),
                env(&[("USERPROFILE", "C:/Users/player"), ("HOME", "/home/player")]),
            ),
            Some(redirected)
        );
    }

    // Read-only: the FFI call answers with a real folder. Nothing is
    // deleted under it.
    #[cfg(windows)]
    #[test]
    fn the_shell_names_an_absolute_documents_folder() {
        let documents = shell_documents().expect("SHGetFolderPathW(CSIDL_PERSONAL)");
        assert!(documents.is_absolute(), "{documents:?}");
    }

    // The shell call failing (or off Windows) falls back to USERPROFILE,
    // then HOME.
    #[test]
    fn without_the_shell_answer_userprofile_then_home_is_used() {
        assert_eq!(
            resolve_documents(
                None,
                env(&[("USERPROFILE", "C:/Users/player"), ("HOME", "/h")])
            ),
            Some(PathBuf::from("C:/Users/player").join("Documents"))
        );
        assert_eq!(
            resolve_documents(None, env(&[("USERPROFILE", ""), ("HOME", "/h")])),
            Some(PathBuf::from("/h").join("Documents"))
        );
        assert_eq!(
            resolve_documents(Some(PathBuf::new()), env(&[("HOME", "/h")])),
            Some(PathBuf::from("/h").join("Documents"))
        );
    }

    // Empty USERPROFILE / HOME must be treated as unset — otherwise
    // user_documents would join "Documents" relative to cwd and the
    // wipe commands would target the wrong tree.
    #[test]
    fn no_documents_when_nothing_names_one() {
        assert_eq!(
            resolve_documents(None, env(&[("USERPROFILE", ""), ("HOME", "")])),
            None
        );
        assert_eq!(resolve_documents(None, env(&[])), None);
    }

    // firesky_root + cache_dir compose to the exact tree the 2009 client
    // writes (Documents\My Games\Firesky\SGWGame\Cache.en-US).
    #[test]
    fn firesky_root_and_cache_dir_compose_under_documents() {
        let dir = tempfile::tempdir().unwrap();
        let documents = test_documents::set(Some(dir.path().join("Documents")));
        assert_eq!(
            firesky_root(),
            Some(dir.path().join("Documents").join(FIRESKY_SUBPATH))
        );
        assert_eq!(
            cache_dir(),
            Some(
                dir.path()
                    .join("Documents")
                    .join(FIRESKY_SUBPATH)
                    .join(SGWGAME_CACHE_SUBPATH)
            )
        );
        drop(documents);
        let _none = test_documents::set(None);
        assert_eq!(firesky_root(), None);
    }
}
