//! Each named lab instance's own `Documents\My Games\Firesky` folder.
//!
//! `SGW.exe` keeps its writable state under My Documents
//! (`My Games\Firesky\SGWGame\`: `Cache.en-US\*.pak`, `Config\*.ini`,
//! saved vars, logs), and it holds all 22 cooked-data cache archives open
//! for writing with read-only sharing for its whole life. A second client on
//! the same folder cannot open one of them, reports cooked version 0 for
//! every category, and the server answers each login with a full resync
//! (about 59,000 entries) that pins its main thread for a minute or more.
//! Measured 2026-10-10: `docs/reverse-engineering/findings/multi-client-lab.md`.
//!
//! The client finds My Documents in exactly one place,
//! `SHGetFolderPathW(CSIDL_PERSONAL)` (`0x004c6333`), and Windows resolves
//! that from the registry value `%USERPROFILE%\Documents` with the calling
//! process's own `USERPROFILE`. So a named instance launches the game with
//! `USERPROFILE` pointed at `sessions/instances/<name>/profile`, seeded once
//! from the real folder (config, shader cache and a warm cooked-data cache,
//! never the per-account folders). Nothing else in the client reads
//! `USERPROFILE`; its one other folder lookup is `CSIDL_LOCAL_APPDATA`
//! (`0x004935ad`), which moves into the profile too and is created on demand.
//!
//! The redirect only works where `Personal` is `%USERPROFILE%`-relative
//! (the Windows default). A machine whose Documents folder is redirected to
//! an absolute path (OneDrive Known Folder Move, a GPO) gets a warning at
//! launch and the shared folder; the durable fix there is a
//! `SHGetFolderPathW` hook in the lab DLL (#1312).

use std::path::{Path, PathBuf};

use super::session_file::sessions_dir;

/// The variable the game resolves My Documents through.
pub const USER_PROFILE_ENV: &str = "USERPROFILE";

/// `My Games\Firesky\SGWGame` under a Documents folder.
const SGWGAME: [&str; 3] = ["My Games", "Firesky", "SGWGame"];

/// Folders copied from the real `SGWGame` into a new profile. Everything
/// else at the top level that is a folder is a per-account folder
/// (`<account>\<character>\* - Saved Vars.lua`), a log or a dump: left out.
const SEED_DIRS: [&str; 3] = ["Config", "Content", "Cache.en-US"];

/// `sessions/instances/<label>/profile`: the profile root the game sees as
/// `USERPROFILE`. Its `Documents` folder is the instance's My Documents.
pub fn profile_dir(install_dir: &Path, label: &str) -> PathBuf {
    sessions_dir(install_dir)
        .join("instances")
        .join(label)
        .join("profile")
}

/// `<documents>\My Games\Firesky\SGWGame`.
pub fn sgwgame_dir(documents: &Path) -> PathBuf {
    SGWGAME.iter().fold(documents.to_path_buf(), |p, s| p.join(s))
}

/// What [`seed`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seeded {
    /// The profile already had an `SGWGame` folder: left alone, so the
    /// instance keeps the cache its own logins brought up to date.
    AlreadyThere,
    /// Copied this many files from the real folder.
    Copied { files: usize, bytes: u64 },
    /// The real folder does not exist yet (the game never ran on this
    /// account): the instance starts empty and the server fills its cache.
    NoSource,
}

/// Seed `profile`'s `SGWGame` from `source` (the real one), once.
pub fn seed(source: &Path, profile: &Path) -> std::io::Result<Seeded> {
    let dst = sgwgame_dir(&profile.join("Documents"));
    if dst.is_dir() {
        return Ok(Seeded::AlreadyThere);
    }
    if !source.is_dir() {
        std::fs::create_dir_all(&dst)?;
        return Ok(Seeded::NoSource);
    }
    // Copy into a sibling and rename, so a copy cut short by a crash never
    // looks like a finished seed on the next launch.
    let tmp = dst.with_file_name("SGWGame.seeding");
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    std::fs::create_dir_all(&tmp)?;
    let (mut files, mut bytes) = (0usize, 0u64);
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let name = entry.file_name();
        let ty = entry.file_type()?;
        if ty.is_file() {
            bytes += std::fs::copy(entry.path(), tmp.join(&name))?;
            files += 1;
        } else if ty.is_dir() && SEED_DIRS.iter().any(|d| name.eq_ignore_ascii_case(d)) {
            copy_tree(&entry.path(), &tmp.join(&name), &mut files, &mut bytes)?;
        }
    }
    std::fs::rename(&tmp, &dst)?;
    Ok(Seeded::Copied { files, bytes })
}

fn copy_tree(src: &Path, dst: &Path, files: &mut usize, bytes: &mut u64) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_tree(&entry.path(), &to, files, bytes)?;
        } else if ty.is_file() {
            *bytes += std::fs::copy(entry.path(), &to)?;
            *files += 1;
        }
    }
    Ok(())
}

/// The real Documents folder for an unexpanded `Personal` shell-folder
/// value, or why a `USERPROFILE` redirect cannot move it.
///
/// `%USERPROFILE%\Documents` (any case, either slash) is redirectable and
/// resolves against `user_profile`, the supervisor's own (unredirected)
/// profile. Anything else, such as an absolute OneDrive path, is not.
pub fn redirectable_documents(personal_raw: &str, user_profile: &Path) -> Result<PathBuf, String> {
    const VAR: &str = "%USERPROFILE%";
    let raw = personal_raw.trim();
    let head = raw.get(..VAR.len()).unwrap_or_default();
    if !head.eq_ignore_ascii_case(VAR) {
        return Err(format!(
            "My Documents is {raw:?}, not under %USERPROFILE%, so a per-instance USERPROFILE \
             cannot move it; lab clients will share one Firesky folder (#1312)"
        ));
    }
    let rest = raw[VAR.len()..].trim_start_matches(['\\', '/']);
    Ok(rest
        .split(['\\', '/'])
        .filter(|s| !s.is_empty())
        .fold(user_profile.to_path_buf(), |p, s| p.join(s)))
}

/// The current user's unexpanded `Personal` value from
/// `HKCU\...\Explorer\User Shell Folders`.
#[cfg(windows)]
pub fn personal_shell_folder_raw() -> Option<String> {
    use windows_sys::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, RRF_NOEXPAND, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ,
    };
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders");
    let value = wide("Personal");
    let mut buf = vec![0u16; 1024];
    let mut len = (buf.len() * 2) as u32;
    // SAFETY: the key and value names are NUL-terminated UTF-16; `buf` and
    // `len` describe a writable buffer of `len` bytes.
    let rc = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ | RRF_RT_REG_EXPAND_SZ | RRF_NOEXPAND,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast(),
            &mut len,
        )
    };
    if rc != 0 {
        return None;
    }
    let chars = (len as usize / 2).min(buf.len());
    let s = String::from_utf16_lossy(&buf[..chars]);
    Some(s.trim_end_matches('\0').to_string())
}

#[cfg(not(windows))]
pub fn personal_shell_folder_raw() -> Option<String> {
    None
}

/// The real `SGWGame` folder to seed from, or why the redirect cannot work
/// on this machine. Windows default when the registry value is missing.
pub fn real_sgwgame() -> Result<PathBuf, String> {
    let profile = std::env::var_os(USER_PROFILE_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| "USERPROFILE is unset".to_string())?;
    let raw = personal_shell_folder_raw().unwrap_or_else(|| r"%USERPROFILE%\Documents".into());
    redirectable_documents(&raw, &profile).map(|docs| sgwgame_dir(&docs))
}

/// `1` keeps every lab client on the real, shared Firesky folder (the
/// pre-#1312 behaviour; two clients then lock each other's cache).
pub const SHARED_USER_DIR_ENV: &str = "CIMMERIA_LAB_SHARED_USER_DIR";

/// Whether [`SHARED_USER_DIR_ENV`]'s raw value asks for the shared folder.
pub fn shared_user_dir_from(raw: Option<&str>) -> bool {
    matches!(raw.map(str::trim), Some("1" | "true" | "yes"))
}

/// Get instance `label`'s profile ready and return the environment entry
/// that points the game at it, or `None` to launch on the shared folder
/// (opted out, a non-redirectable Documents folder, or a seed failure; each
/// logged). Called on every launch; the seed itself happens once.
pub fn prepare(install_dir: &Path, label: &str) -> Option<(String, String)> {
    if shared_user_dir_from(std::env::var(SHARED_USER_DIR_ENV).ok().as_deref()) {
        tracing::info!(target: "lab.instance", event = "user_dir_shared", instance = label,
            reason = "opted_out", "lab client uses the shared Firesky folder ({SHARED_USER_DIR_ENV})");
        return None;
    }
    let source = match real_sgwgame() {
        Ok(s) => s,
        Err(why) => {
            tracing::warn!(target: "lab.instance", event = "user_dir_shared", instance = label,
                reason = "documents_not_redirectable", detail = %why,
                "lab client uses the shared Firesky folder");
            return None;
        }
    };
    let profile = profile_dir(install_dir, label);
    match seed(&source, &profile) {
        Ok(outcome) => {
            let (seeded, files, bytes) = match outcome {
                Seeded::AlreadyThere => ("already_there", 0, 0),
                Seeded::Copied { files, bytes } => ("copied", files, bytes),
                Seeded::NoSource => ("no_source", 0, 0),
            };
            tracing::info!(target: "lab.instance", event = "user_dir_ready", instance = label,
                seeded, files, bytes, profile = %profile.display(),
                "lab client gets its own Firesky folder");
            Some((
                USER_PROFILE_ENV.to_string(),
                profile.to_string_lossy().into_owned(),
            ))
        }
        Err(e) => {
            tracing::warn!(target: "lab.instance", event = "user_dir_shared", instance = label,
                reason = "seed_failed", error = %e, profile = %profile.display(),
                "could not seed the instance profile; lab client uses the shared Firesky folder");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_explicit_yes_shares_the_folder() {
        for yes in ["1", "true", "yes", " 1 "] {
            assert!(shared_user_dir_from(Some(yes)), "{yes:?}");
        }
        for no in [None, Some(""), Some("0"), Some("false"), Some("no")] {
            assert!(!shared_user_dir_from(no), "{no:?}");
        }
    }

    #[test]
    fn profile_lives_in_the_instance_dir() {
        let p = profile_dir(Path::new("C:/Games/SGW"), "p3");
        assert!(p.ends_with("Binaries/sessions/instances/p3/profile"));
    }

    #[test]
    fn userprofile_relative_documents_are_redirectable() {
        let home = Path::new("C:/Users/tester");
        for raw in [
            r"%USERPROFILE%\Documents",
            r"%userprofile%\Documents",
            "%USERPROFILE%/Documents",
            r" %USERPROFILE%\Documents\ ",
        ] {
            assert_eq!(
                redirectable_documents(raw, home).unwrap(),
                home.join("Documents"),
                "{raw:?}"
            );
        }
    }

    #[test]
    fn absolute_documents_are_not_redirectable() {
        let home = Path::new("C:/Users/tester");
        for raw in [
            r"C:\Users\tester\OneDrive\Documents",
            r"D:\Docs",
            r"%OneDrive%\Documents",
            "",
        ] {
            let err = redirectable_documents(raw, home).unwrap_err();
            assert!(err.contains("#1312"), "{raw:?}: {err}");
        }
    }

    fn write(path: &Path, body: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn seed_copies_config_content_cache_and_top_files_only() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("real/SGWGame");
        write(&src.join("Cache.en-US/TextStrings.pak"), "pak");
        write(&src.join("Config/SGWEngine.ini"), "ini");
        write(&src.join("Content/LocalShaderCache-PC-D3D-SM3.upk"), "upk");
        write(&src.join("SavedSystemOptions.xml"), "xml");
        write(&src.join("lab/Labone/ActionButtons - Saved Vars.lua"), "account");
        write(&src.join("Logs/Launch.log"), "log");
        write(&src.join("CrashDumps/x.dmp"), "dump");
        let profile = tmp.path().join("profile");

        let out = seed(&src, &profile).unwrap();
        assert_eq!(out, Seeded::Copied { files: 4, bytes: 12 });
        let dst = sgwgame_dir(&profile.join("Documents"));
        for kept in [
            "Cache.en-US/TextStrings.pak",
            "Config/SGWEngine.ini",
            "Content/LocalShaderCache-PC-D3D-SM3.upk",
            "SavedSystemOptions.xml",
        ] {
            assert!(dst.join(kept).is_file(), "{kept} must be seeded");
        }
        for left in ["lab", "Logs", "CrashDumps"] {
            assert!(!dst.join(left).exists(), "{left} must not be seeded");
        }
        assert!(!dst.with_file_name("SGWGame.seeding").exists());
    }

    #[test]
    fn seed_runs_once_so_the_instance_keeps_its_own_cache() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("real/SGWGame");
        write(&src.join("Cache.en-US/CookedWorldInfo.pak"), "v1");
        let profile = tmp.path().join("profile");
        seed(&src, &profile).unwrap();
        let mine = sgwgame_dir(&profile.join("Documents")).join("Cache.en-US/CookedWorldInfo.pak");
        std::fs::write(&mine, "updated by this instance's login").unwrap();
        write(&src.join("Cache.en-US/CookedWorldInfo.pak"), "v2");

        assert_eq!(seed(&src, &profile).unwrap(), Seeded::AlreadyThere);
        assert_eq!(
            std::fs::read_to_string(&mine).unwrap(),
            "updated by this instance's login"
        );
    }

    #[test]
    fn seed_without_a_real_folder_starts_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let profile = tmp.path().join("profile");
        assert_eq!(
            seed(&tmp.path().join("missing"), &profile).unwrap(),
            Seeded::NoSource
        );
        assert!(sgwgame_dir(&profile.join("Documents")).is_dir());
    }

    #[test]
    fn an_interrupted_seed_is_redone() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("real/SGWGame");
        write(&src.join("Config/SGWEngine.ini"), "ini");
        let profile = tmp.path().join("profile");
        let half = sgwgame_dir(&profile.join("Documents")).with_file_name("SGWGame.seeding");
        write(&half.join("partial.tmp"), "x");

        assert_eq!(seed(&src, &profile).unwrap(), Seeded::Copied { files: 1, bytes: 3 });
        assert!(!half.exists());
    }
}
