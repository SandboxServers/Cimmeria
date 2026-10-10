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
//! `USERPROFILE` pointed at `<root>/<name>/profile`, seeded once from the
//! real folder (config, shader cache and a warm cooked-data cache, never the
//! per-account folders). The root is `%LOCALAPPDATA%\cimmeria-lab\instances`
//! (or `CIMMERIA_LAB_PROFILE_ROOT`), outside the game install: `SGW.exe`
//! refuses a `USERPROFILE` inside its own install folder (#1312). Nothing else in the client reads
//! `USERPROFILE`; its one other folder lookup is `CSIDL_LOCAL_APPDATA`
//! (`0x004935ad`), which moves into the profile too and is created on demand.
//!
//! The redirect only works where `Personal` is `%USERPROFILE%`-relative
//! (the Windows default). A machine whose Documents folder is redirected to
//! an absolute path (OneDrive Known Folder Move, a GPO) gets a warning at
//! launch and the shared folder; the durable fix there is a
//! `SHGetFolderPathW` hook in the lab DLL (#1312).

use std::path::{Component, Path, PathBuf, Prefix};

/// The variable the game resolves My Documents through.
pub const USER_PROFILE_ENV: &str = "USERPROFILE";

/// The folder holding every instance's profile, `<root>\<label>\profile`.
/// Default `%LOCALAPPDATA%\cimmeria-lab\instances`. It must be outside the
/// game install: `SGW.exe` refuses a `USERPROFILE` inside its own install
/// folder (#1312).
pub const PROFILE_ROOT_ENV: &str = "CIMMERIA_LAB_PROFILE_ROOT";

/// `My Games\Firesky\SGWGame` under a Documents folder.
const SGWGAME: [&str; 3] = ["My Games", "Firesky", "SGWGame"];

/// Folders copied from the real `SGWGame` into a new profile. Everything
/// else at the top level that is a folder is a per-account folder
/// (`<account>\<character>\* - Saved Vars.lua`), a log or a dump: left out.
const SEED_DIRS: [&str; 3] = ["Config", "Content", "Cache.en-US"];

/// `<root>\<label>\profile`: the profile the game sees as `USERPROFILE`. Its
/// `Documents` folder is the instance's My Documents.
pub fn profile_dir(root: &Path, label: &str) -> PathBuf {
    root.join(label).join("profile")
}

/// The profile root from the raw [`PROFILE_ROOT_ENV`] value (a non-empty
/// trimmed value wins), else `<local_app_data>\cimmeria-lab\instances`, else
/// `None`.
pub fn profile_root_from(raw: Option<&str>, local_app_data: Option<&Path>) -> Option<PathBuf> {
    match raw.map(str::trim).filter(|s| !s.is_empty()) {
        Some(root) => Some(PathBuf::from(root)),
        None => local_app_data.map(|l| l.join("cimmeria-lab").join("instances")),
    }
}

/// Whether `path` is `dir` or lies under it. Windows paths: the comparison
/// ignores case and works on whole components, so `C:\Games\SGW2` is not
/// inside `C:\Games\SGW`. Both paths are normalised lexically first: `.` is
/// dropped, `..` pops, and a `\\?\` verbatim prefix matches its plain form.
/// Junctions, symlinks and 8.3 short names are not resolved. An empty `dir`
/// counts as containing everything (fail-safe: the caller refuses the root).
pub fn inside(path: &Path, dir: &Path) -> bool {
    let (dir_anchor, dir_parts) = lexical(dir);
    if dir_anchor.is_empty() && dir_parts.is_empty() {
        return true;
    }
    let (path_anchor, path_parts) = lexical(path);
    path_anchor == dir_anchor && path_parts.starts_with(&dir_parts)
}

/// The drive or UNC anchor (lowercased, `C:` and `\\?\C:` alike) and the
/// normal components below it (lowercased), with `.` dropped and `..` popped.
fn lexical(path: &Path) -> (String, Vec<String>) {
    let mut anchor = String::new();
    let mut parts: Vec<String> = Vec::new();
    for c in path.components() {
        match c {
            Component::Prefix(prefix) => {
                anchor = match prefix.kind() {
                    Prefix::Disk(d) | Prefix::VerbatimDisk(d) => {
                        format!("{}:", char::from(d).to_lowercase())
                    }
                    _ => prefix.as_os_str().to_string_lossy().to_lowercase(),
                }
            }
            Component::RootDir => anchor.push('\\'),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop();
            }
            Component::Normal(s) => parts.push(s.to_string_lossy().to_lowercase()),
        }
    }
    (anchor, parts)
}

/// `<documents>\My Games\Firesky\SGWGame`.
pub fn sgwgame_dir(documents: &Path) -> PathBuf {
    SGWGAME
        .iter()
        .fold(documents.to_path_buf(), |p, s| p.join(s))
}

/// What [`seed`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seeded {
    /// The profile already had an `SGWGame` folder: left alone, so the
    /// instance keeps the cache its own logins brought up to date.
    AlreadyThere,
    /// Copied this many files from the real folder.
    Copied {
        /// Files copied.
        files: usize,
        /// Their total size.
        bytes: u64,
    },
    /// The real folder does not exist yet (the game never ran on this
    /// account): the instance starts empty and the server fills its cache.
    NoSource,
}

/// Serialises seeds: two launches of one instance at once must not race on
/// its `SGWGame.seeding` folder (the loser would fall back to the shared
/// folder, the #1312 lock). Seeds are rare, so one lock for every instance.
static SEED_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Seed `profile`'s `SGWGame` from `source` (the real one), once.
pub fn seed(source: &Path, profile: &Path) -> std::io::Result<Seeded> {
    let _guard = SEED_LOCK.lock().unwrap_or_else(|p| p.into_inner());
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
/// `HKCU\...\Explorer\User Shell Folders`: `Ok(None)` when the value is
/// absent (Windows then uses its default), `Err` on any other failure.
#[cfg(windows)]
pub fn personal_shell_folder_raw() -> Result<Option<String>, String> {
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_MORE_DATA};
    use windows_sys::Win32::System::Registry::{
        RegGetValueW, HKEY_CURRENT_USER, RRF_NOEXPAND, RRF_RT_REG_EXPAND_SZ, RRF_RT_REG_SZ,
    };
    let wide = |s: &str| s.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let key = wide(r"Software\Microsoft\Windows\CurrentVersion\Explorer\User Shell Folders");
    let value = wide("Personal");
    let mut buf = vec![0u16; 1024];
    // A value longer than the buffer reports its size: grow once and retry.
    for _ in 0..2 {
        let mut len = (buf.len() * 2) as u32;
        // SAFETY: the key and value names are NUL-terminated UTF-16; `buf`
        // and `len` describe a writable buffer of `len` bytes.
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
        match rc {
            0 => {
                let chars = (len as usize / 2).min(buf.len());
                let s = String::from_utf16_lossy(&buf[..chars]);
                return Ok(Some(s.trim_end_matches('\0').to_string()));
            }
            ERROR_FILE_NOT_FOUND => return Ok(None),
            ERROR_MORE_DATA => buf = vec![0u16; len as usize / 2 + 1],
            rc => {
                return Err(format!(
                    "reading the Personal shell folder failed (error {rc})"
                ))
            }
        }
    }
    Err("the Personal shell folder value kept growing".into())
}

/// Off Windows there is no shell-folder registry: always the default.
#[cfg(not(windows))]
pub fn personal_shell_folder_raw() -> Result<Option<String>, String> {
    Ok(None)
}

/// The real `SGWGame` folder to seed from, or why the redirect cannot work
/// on this machine. Windows default when the registry value is missing.
pub fn real_sgwgame() -> Result<PathBuf, String> {
    let profile = std::env::var_os(USER_PROFILE_ENV)
        .map(PathBuf::from)
        .ok_or_else(|| "USERPROFILE is unset".to_string())?;
    let raw = personal_shell_folder_raw()?.unwrap_or_else(|| r"%USERPROFILE%\Documents".into());
    redirectable_documents(&raw, &profile).map(|docs| sgwgame_dir(&docs))
}

/// `1` keeps every lab client on the real, shared Firesky folder (the
/// pre-#1312 behaviour; two clients then lock each other's cache).
pub const SHARED_USER_DIR_ENV: &str = "CIMMERIA_LAB_SHARED_USER_DIR";

/// Whether [`SHARED_USER_DIR_ENV`]'s raw value asks for the shared folder.
pub fn shared_user_dir_from(raw: Option<&str>) -> bool {
    let raw = raw.map(|s| s.trim().to_ascii_lowercase());
    matches!(raw.as_deref(), Some("1" | "true" | "yes"))
}

/// Get instance `label`'s profile ready and return the environment entry
/// that points the game at it, or `None` to launch on the shared folder
/// (opted out, a non-redirectable Documents folder, no profile root, a root
/// inside the install, or a seed failure; each logged). Called on every
/// launch; the seed itself happens once.
pub fn prepare(install_dir: &Path, label: &str) -> Option<(String, String)> {
    let shared = shared_user_dir_from(std::env::var(SHARED_USER_DIR_ENV).ok().as_deref());
    let root = profile_root_from(
        std::env::var(PROFILE_ROOT_ENV).ok().as_deref(),
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .as_deref(),
    );
    prepare_from(shared, real_sgwgame, root, install_dir, label)
}

/// [`prepare`] over its inputs: the opt-out, how to find the real `SGWGame`
/// folder (only asked when not opted out), and the profile root.
fn prepare_from(
    shared: bool,
    real: impl FnOnce() -> Result<PathBuf, String>,
    root: Option<PathBuf>,
    install_dir: &Path,
    label: &str,
) -> Option<(String, String)> {
    if shared {
        tracing::info!(target: "lab.instance", event = "user_dir_shared", instance = label,
            reason = "opted_out", "lab client uses the shared Firesky folder ({SHARED_USER_DIR_ENV})");
        return None;
    }
    let source = match real() {
        Ok(s) => s,
        Err(why) => {
            tracing::warn!(target: "lab.instance", event = "user_dir_shared", instance = label,
                reason = "documents_not_redirectable", detail = %why,
                "lab client uses the shared Firesky folder");
            return None;
        }
    };
    let Some(root) = root else {
        tracing::warn!(target: "lab.instance", event = "user_dir_shared", instance = label,
            reason = "no_profile_root",
            "lab client uses the shared Firesky folder; set {PROFILE_ROOT_ENV} or LOCALAPPDATA");
        return None;
    };
    if !root.is_absolute() {
        tracing::warn!(target: "lab.instance", event = "user_dir_shared", instance = label,
            reason = "profile_root_not_absolute", root = %root.display(),
            "a relative profile root would resolve inside the install; set {PROFILE_ROOT_ENV} to an absolute path");
        return None;
    }
    if inside(&root, install_dir) {
        tracing::warn!(target: "lab.instance", event = "user_dir_shared", instance = label,
            reason = "profile_root_inside_install", root = %root.display(),
            install = %install_dir.display(),
            "SGW.exe refuses a user folder inside its install; set {PROFILE_ROOT_ENV} outside it");
        return None;
    }
    let profile = profile_dir(&root, label);
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
        for yes in ["1", "true", "yes", " 1 ", "TRUE", "Yes"] {
            assert!(shared_user_dir_from(Some(yes)), "{yes:?}");
        }
        for no in [None, Some(""), Some("0"), Some("false"), Some("no")] {
            assert!(!shared_user_dir_from(no), "{no:?}");
        }
    }

    #[test]
    fn profile_lives_under_the_root() {
        let p = profile_dir(Path::new("C:/cimmeria-lab/instances"), "p3");
        assert!(p.ends_with("cimmeria-lab/instances/p3/profile"));
    }

    #[test]
    fn profile_root_prefers_the_override_then_local_app_data() {
        let local = Path::new(r"C:\Users\tester\AppData\Local");
        let default = local.join("cimmeria-lab").join("instances");
        assert_eq!(
            profile_root_from(Some(r" D:\lab\profiles "), Some(local)),
            Some(PathBuf::from(r"D:\lab\profiles"))
        );
        for raw in [None, Some(""), Some("   ")] {
            assert_eq!(
                profile_root_from(raw, Some(local)),
                Some(default.clone()),
                "{raw:?}"
            );
        }
        assert_eq!(profile_root_from(None, None), None);
        assert_eq!(profile_root_from(Some(""), None), None);
    }

    #[test]
    fn inside_ignores_case_and_needs_a_component_boundary() {
        let install = Path::new(r"c:\games\sgw");
        assert!(inside(Path::new(r"C:\Games\SGW\binaries\x"), install));
        assert!(inside(Path::new(r"C:\Games\SGW"), install));
        assert!(inside(Path::new("C:/Games/SGW/Binaries"), install));
        assert!(!inside(Path::new(r"C:\Games\SGW2"), install));
        assert!(!inside(Path::new(r"C:\Games"), install));
        assert!(!inside(Path::new(r"D:\Games\SGW\x"), install));
    }

    #[test]
    fn inside_normalises_dots_verbatim_prefixes_and_non_ascii_case() {
        let install = Path::new(r"C:\Games\SGW");
        // `..` pops: this path is the install's own sibling folder walked back in.
        assert!(inside(Path::new(r"C:\Games\Other\..\SGW\x"), install));
        assert!(!inside(Path::new(r"C:\Games\SGW\..\Other"), install));
        // `.` is dropped.
        assert!(inside(Path::new(r"C:\Games\.\SGW\x"), install));
        // A verbatim prefix matches its plain drive form.
        assert!(inside(Path::new(r"\\?\C:\Games\SGW\x"), install));
        assert!(inside(Path::new(r"\\?\c:\games\sgw"), install));
        assert!(!inside(Path::new(r"\\?\C:\Games\SGW2"), install));
        // Lowercasing is not ASCII-only.
        assert!(inside(
            Path::new(r"C:\Jeux\ÉTÉ\x"),
            Path::new(r"c:\jeux\été")
        ));
    }

    #[test]
    fn inside_treats_an_empty_dir_as_containing_everything() {
        assert!(inside(Path::new(r"C:\Games\SGW\x"), Path::new("")));
        assert!(!inside(Path::new(""), Path::new(r"C:\Games\SGW")));
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
        write(
            &src.join("lab/Labone/ActionButtons - Saved Vars.lua"),
            "account",
        );
        write(&src.join("Logs/Launch.log"), "log");
        write(&src.join("CrashDumps/x.dmp"), "dump");
        let profile = tmp.path().join("profile");

        let out = seed(&src, &profile).unwrap();
        assert_eq!(
            out,
            Seeded::Copied {
                files: 4,
                bytes: 12
            }
        );
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

        assert_eq!(
            seed(&src, &profile).unwrap(),
            Seeded::Copied { files: 1, bytes: 3 }
        );
        assert!(!half.exists());
        assert!(
            !sgwgame_dir(&profile.join("Documents"))
                .join("partial.tmp")
                .exists(),
            "a half-finished seed's files must not reach the instance"
        );
    }

    #[test]
    fn prepare_points_the_default_instance_at_its_own_profile() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("real/SGWGame");
        write(&src.join("Config/SGWEngine.ini"), "ini");
        let install = tmp.path().join("install");
        let root = tmp.path().join("profiles");
        let env = prepare_from(
            false,
            || Ok(src.clone()),
            Some(root.clone()),
            &install,
            "default",
        )
        .unwrap();
        let profile = profile_dir(&root, "default");
        assert_eq!(
            env,
            (
                USER_PROFILE_ENV.to_string(),
                profile.to_string_lossy().into_owned()
            )
        );
        assert!(sgwgame_dir(&profile.join("Documents"))
            .join("Config/SGWEngine.ini")
            .is_file());
    }

    #[test]
    fn prepare_falls_back_to_the_shared_folder() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("install");
        let src = tmp.path().join("real/SGWGame");
        let root = tmp.path().join("profiles");
        write(&src.join("Config/SGWEngine.ini"), "ini");

        // Opted out: the real folder is not even looked up.
        assert_eq!(
            prepare_from(
                true,
                || panic!("not asked"),
                Some(root.clone()),
                &install,
                "p2"
            ),
            None
        );
        // Documents not redirectable (OneDrive, a registry error).
        assert_eq!(
            prepare_from(
                false,
                || Err("absolute".into()),
                Some(root.clone()),
                &install,
                "p2"
            ),
            None
        );
        // The seed fails: the profile's Documents is a file.
        write(&profile_dir(&root, "p3").join("Documents"), "not a dir");
        assert_eq!(
            prepare_from(
                false,
                || Ok(src.clone()),
                Some(root.clone()),
                &install,
                "p3"
            ),
            None
        );
    }

    #[test]
    fn prepare_refuses_an_unsafe_root() {
        let tmp = tempfile::tempdir().unwrap();
        let install = tmp.path().join("install");
        let src = tmp.path().join("real/SGWGame");
        write(&src.join("Config/SGWEngine.ini"), "ini");

        let inside_root = install.join("Binaries/sessions/instances");
        assert_eq!(
            prepare_from(false, || Ok(src.clone()), Some(inside_root), &install, "p2"),
            None
        );
        // No root at all (no override, no LOCALAPPDATA): also shared.
        assert_eq!(
            prepare_from(false, || Ok(src.clone()), None, &install, "p2"),
            None
        );
        // A relative root would resolve against the game's cwd (Binaries).
        let relative = PathBuf::from(r"profiles\instances");
        assert_eq!(
            prepare_from(false, || Ok(src.clone()), Some(relative), &install, "p2"),
            None
        );
        // An empty LOCALAPPDATA gives the same relative default root.
        let from_empty_local = profile_root_from(None, Some(Path::new(""))).unwrap();
        assert!(!from_empty_local.is_absolute());
        assert_eq!(
            prepare_from(
                false,
                || Ok(src.clone()),
                Some(from_empty_local),
                &install,
                "p2"
            ),
            None
        );
    }
}
