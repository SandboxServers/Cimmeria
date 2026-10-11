//! Where the game's files sit inside the install directory.
//!
//! The 2009 installer lays the client out as
//!
//! ```text
//! <install>\Common\...
//! <install>\Resources\...
//! <install>\Working\Binaries\SGW.exe      (the cabinets spell it `binaries`)
//! <install>\Working\SGWGame\...
//! ```
//!
//! `SGW.exe`, its DLLs, `SGWDebugLog.log` and the `sessions\` telemetry
//! directory all live in `Working\Binaries`, and the client must be started
//! with that directory as its working directory. Older launcher configs point
//! the install path straight at `Binaries` (or at `Working`), so
//! [`binaries_dir`] accepts all three.

use std::path::{Path, PathBuf};

/// The directory holding `SGW.exe` for the install rooted at `install_dir`.
///
/// Resolution order:
/// 1. `install_dir` itself when it contains `SGW.exe` (the path points
///    straight at the binaries directory).
/// 2. `install_dir\Working\Binaries` when it exists (a full install).
/// 3. `install_dir\Binaries` when it exists (the path points at `Working`).
/// 4. Otherwise `install_dir\Working\Binaries`, where a fresh install puts it.
///
/// `Binaries` is matched case-blind, since the cabinets store `binaries`.
pub fn binaries_dir(install_dir: &Path) -> PathBuf {
    if install_dir.join("SGW.exe").is_file() {
        return install_dir.to_path_buf();
    }
    let working = install_dir.join("Working");
    if let Some(dir) = existing_binaries(&working) {
        return dir;
    }
    if let Some(dir) = existing_binaries(install_dir) {
        return dir;
    }
    working.join("Binaries")
}

/// The client's `SGWGame` directory, the sibling of the binaries directory
/// (`<install>\Working\SGWGame`).
pub fn sgwgame_dir(install_dir: &Path) -> PathBuf {
    let bin = binaries_dir(install_dir);
    bin.parent().unwrap_or(install_dir).join("SGWGame")
}

/// Path of `SGW.exe` for the install rooted at `install_dir`.
pub fn sgw_exe(install_dir: &Path) -> PathBuf {
    binaries_dir(install_dir).join("SGW.exe")
}

/// Move the installer's bundled cooked-data PAKs to where the client reads
/// them. Returns true when it moved them.
///
/// The client keeps two cooked-data tiers (`LaunchMisc.cpp`, see
/// `docs/reverse-engineering/findings/cooked-data-pipeline.md`):
/// `CachePath` is the writable tier the server's version push rewrites
/// (it lands in `Documents\My Games\Firesky\SGWGame\Cache.en-US`), and
/// `SourceCachePath=..\SGWGame\SourceCache` plus the locale is the
/// read-only bundled tier, `Working\SGWGame\SourceCache.en-us`. The 2009
/// cabinets put the bundled PAKs in `Working\SGWGame\Cache.en-US`, which
/// is neither. A stock install logs, once per cooked-data category,
/// `WARN common - Non-existent source archive directory:
/// <install>\Working\SGWGame\SourceCache.en-US`; creating that directory
/// with the shipped PAKs silences it. This does that rename. It does
/// nothing when the source tier already exists, so it never overwrites a
/// patched `SourceCache.en-us`.
pub fn place_bundled_cooked_data(install_dir: &Path) -> std::io::Result<bool> {
    let sgwgame = install_dir.join("Working").join("SGWGame");
    let shipped = sgwgame.join("Cache.en-US");
    let source = sgwgame.join("SourceCache.en-us");
    if !shipped.is_dir() || source.exists() {
        return Ok(false);
    }
    std::fs::rename(&shipped, &source)?;
    Ok(true)
}

/// The `Binaries` directory under `parent`, in whatever case it has on
/// disk (the cabinets store `binaries`; a case-sensitive file system would
/// miss it under the other spelling).
fn existing_binaries(parent: &Path) -> Option<PathBuf> {
    std::fs::read_dir(parent)
        .ok()?
        .flatten()
        .find(|e| {
            e.file_name().eq_ignore_ascii_case("binaries")
                && e.file_type().is_ok_and(|t| t.is_dir())
        })
        .map(|e| e.path())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_install_resolves_to_working_binaries() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("Working").join("binaries");
        std::fs::create_dir_all(&bin).unwrap();
        std::fs::write(bin.join("SGW.exe"), b"").unwrap();
        assert_eq!(binaries_dir(dir.path()), bin);
        assert_eq!(sgw_exe(dir.path()), bin.join("SGW.exe"));
    }

    #[test]
    fn path_pointing_at_binaries_resolves_to_itself() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), b"").unwrap();
        assert_eq!(binaries_dir(dir.path()), dir.path());
    }

    #[test]
    fn path_pointing_at_working_resolves_to_its_binaries() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Binaries")).unwrap();
        assert_eq!(binaries_dir(dir.path()), dir.path().join("Binaries"));
    }

    // Bug shape: a raw cabinet expansion leaves the bundled PAKs in
    // Working\SGWGame\Cache.en-US, a folder the client never reads.
    #[test]
    fn bundled_paks_move_to_the_source_cache() {
        let dir = tempfile::tempdir().unwrap();
        let sgwgame = dir.path().join("Working").join("SGWGame");
        std::fs::create_dir_all(sgwgame.join("Cache.en-US")).unwrap();
        std::fs::write(sgwgame.join("Cache.en-US").join("TextStrings.pak"), b"pk").unwrap();
        assert!(place_bundled_cooked_data(dir.path()).unwrap());
        assert_eq!(
            std::fs::read(sgwgame.join("SourceCache.en-us").join("TextStrings.pak")).unwrap(),
            b"pk"
        );
        assert!(!sgwgame.join("Cache.en-US").exists());
    }

    // An install that already has a source tier (possibly patched) keeps it.
    #[test]
    fn existing_source_cache_is_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let sgwgame = dir.path().join("Working").join("SGWGame");
        std::fs::create_dir_all(sgwgame.join("Cache.en-US")).unwrap();
        std::fs::create_dir_all(sgwgame.join("SourceCache.en-us")).unwrap();
        std::fs::write(sgwgame.join("SourceCache.en-us").join("a.pak"), b"patched").unwrap();
        assert!(!place_bundled_cooked_data(dir.path()).unwrap());
        assert_eq!(
            std::fs::read(sgwgame.join("SourceCache.en-us").join("a.pak")).unwrap(),
            b"patched"
        );
        assert!(!place_bundled_cooked_data(tempfile::tempdir().unwrap().path()).unwrap());
    }

    #[test]
    fn empty_install_defaults_to_working_binaries() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            binaries_dir(dir.path()),
            dir.path().join("Working").join("Binaries")
        );
    }
}
