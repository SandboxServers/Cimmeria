//! The display step of Play (C-PD7, D-PD16): before each launch the launcher
//! writes its display choice into the client profile, so a change made in the
//! game's own options lasts one session. Best effort: it never fails a launch.
use super::events;
#[cfg(any(target_os = "linux", test))]
use crate::linux_proton::prefix::Prefix;
use crate::storage::display_options::{DisplayMode, DisplayOptions};
use crate::storage::DesktopState;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[cfg(test)]
#[path = "display_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "display_proton_tests.rs"]
mod proton_tests;

/// The client's saved options, in the profile folder.
const SAVED_OPTIONS: &str = "SavedSystemOptions.xml";
/// The user engine ini, in the profile folder.
const ENGINE_INI: &str = "Config/SGWEngine.ini";
/// The only engine ini section the launcher touches.
const WINDOWS_CLIENT: &str = "[WinDrv.WindowsClient]";
/// The first Linux Play's saved options (PP-S2 B2): the game writes none itself.
#[cfg(any(target_os = "linux", test))]
const SEED_SAVED_OPTIONS: &str = concat!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n",
    "<savedOptions xmlns:SOAP-ENV=\"http://schemas.xmlsoap.org/soap/envelope/\" ",
    "xmlns:SOAP-ENC=\"http://schemas.xmlsoap.org/soap/encoding/\" ",
    "xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" ",
    "xmlns:xsd=\"http://www.w3.org/2001/XMLSchema\" ",
    "xmlns:SGWSystemOptions=\"http://www.cheyenneme.com/sgw/options\">",
    "<options valueBool=\"true\" version=\"1\" optionName=\"windowedMode\" groupName=\"video\"></options>",
    "</savedOptions>",
);

/// What happened to one profile file, as its closed event code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOutcome {
    Written,
    Unchanged,
    /// A saved options file the launcher created (PP-17, D-PP27).
    #[cfg_attr(not(any(target_os = "linux", test)), allow(dead_code))]
    Seeded,
    Missing,
    KeyMissing,
    Unreadable,
    Failed,
}

impl FileOutcome {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Written => "written",
            Self::Unchanged => "unchanged",
            Self::Seeded => "seeded",
            Self::Missing => "missing",
            Self::KeyMissing => "key_missing",
            Self::Unreadable => "unreadable",
            Self::Failed => "failed",
        }
    }
}

/// The saved display options, read under the state lock. A lock that cannot
/// be had gives the default: the display step never stops a launch.
pub(crate) fn options(state: &Arc<Mutex<DesktopState>>) -> DisplayOptions {
    state
        .lock()
        .map(|owner| owner.preferences.display)
        .unwrap_or_default()
}

/// Sets `valueBool` on the one `windowedMode` line of `SavedSystemOptions.xml`.
/// Every other byte is kept. No such line, or two of them, gives `None`.
pub(crate) fn saved_options(text: &str, windowed: bool) -> Option<String> {
    let is_option = |line: &str| {
        line.contains(r#"optionName="windowedMode""#) && line.contains(r#"groupName="video""#)
    };
    if text.split_inclusive('\n').filter(|l| is_option(l)).count() != 1 {
        return None;
    }
    let value = if windowed { "true" } else { "false" };
    let mut out = String::with_capacity(text.len() + 1);
    for line in text.split_inclusive('\n') {
        if !is_option(line) {
            out.push_str(line);
            continue;
        }
        let start = line.find(r#"valueBool=""#)? + r#"valueBool=""#.len();
        let end = start + line[start..].find('"')?;
        out.push_str(&line[..start]);
        out.push_str(value);
        out.push_str(&line[end..]);
    }
    Some(out)
}

/// Sets `StartupFullscreen` and, with a `size`, `StartupResolutionX` and
/// `StartupResolutionY`, inside `[WinDrv.WindowsClient]` only. Keys match
/// case-insensitively and keep their spelling; line endings are kept. A
/// missing section or key gives `None`.
pub(crate) fn engine_ini(text: &str, fullscreen: bool, size: Option<(u32, u32)>) -> Option<String> {
    let mut wanted = vec![(
        "StartupFullscreen",
        if fullscreen { "True" } else { "False" }.to_owned(),
    )];
    if let Some((width, height)) = size {
        wanted.push(("StartupResolutionX", width.to_string()));
        wanted.push(("StartupResolutionY", height.to_string()));
    }
    let mut found = vec![false; wanted.len()];
    let mut inside = false;
    let mut out = String::with_capacity(text.len() + 8);
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches(['\r', '\n']);
        let ending = &line[body.len()..];
        if body.trim_start().starts_with('[') {
            inside = body.trim().eq_ignore_ascii_case(WINDOWS_CLIENT);
            out.push_str(line);
            continue;
        }
        let replaced = inside
            .then(|| body.split_once('='))
            .flatten()
            .and_then(|(key, _)| {
                let index = wanted
                    .iter()
                    .position(|(name, _)| key.trim().eq_ignore_ascii_case(name))?;
                found[index] = true;
                Some(format!("{key}={}{ending}", wanted[index].1))
            });
        out.push_str(replaced.as_deref().unwrap_or(line));
    }
    found.iter().all(|f| *f).then_some(out)
}

/// The launch helper's arguments for SGW.exe (C-PD7; PD-25 passes them).
#[allow(dead_code)] // Until PD-25 builds the helper request with them.
pub(crate) fn arguments(display: DisplayOptions) -> Vec<String> {
    let display = display.effective();
    match display.mode {
        DisplayMode::Windowed => vec![
            "-windowed".to_owned(),
            format!("ResX={}", display.width),
            format!("ResY={}", display.height),
        ],
        DisplayMode::Fullscreen => Vec::new(),
    }
}

/// The Windows client profile, `<Documents>/My Games/Firesky/SGWGame`. Tests
/// get none, so a test run never edits the developer's own profile.
#[cfg(all(not(target_os = "macos"), not(target_os = "linux"), not(test)))]
pub(crate) fn native_profile() -> Option<PathBuf> {
    cimmeria_launcher_core::client_paths::firesky_root().map(|root| root.join("SGWGame"))
}

/// Native Play is Windows-only; macOS, Linux and tests have no native profile.
#[cfg(any(target_os = "macos", target_os = "linux", test))]
pub(crate) fn native_profile() -> Option<PathBuf> {
    None
}

/// The Wine prefix's client profile: the one user folder other than `Public`,
/// then `Documents/My Games/Firesky/SGWGame`. Zero or several user folders, a
/// missing profile, or a path that leaves the prefix give `None`.
#[cfg(target_os = "macos")]
pub(crate) fn wine_profile(prefix: &Path) -> Option<PathBuf> {
    let prefix = prefix.canonicalize().ok()?;
    let mut users = std::fs::read_dir(prefix.join("drive_c/users"))
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name() != "Public")
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()));
    let user = users.next()?;
    if users.next().is_some() {
        return None;
    }
    let profile = user
        .path()
        .join("Documents/My Games/Firesky/SGWGame")
        .canonicalize()
        .ok()?;
    super::preparation::contained(&prefix, &profile).ok()?;
    Some(profile)
}

/// The Proton prefix's client profile: `Documents/My Games/Firesky/SGWGame` in
/// the prefix's `steamuser` folder (PP-17).
#[cfg(any(target_os = "linux", test))]
pub(super) fn proton_profile(prefix: &Prefix) -> PathBuf {
    prefix
        .profile_root()
        .join("Documents/My Games/Firesky/SGWGame")
}

/// Writes the effective display choice into both profile files and logs one
/// `display_settings` event. Never fails: Play continues whatever happens.
pub(crate) fn apply(operation: Uuid, profile: Option<&Path>, display: DisplayOptions) {
    let display = display.effective();
    let fullscreen = display.mode == DisplayMode::Fullscreen;
    let size = (!fullscreen).then_some((display.width, display.height));
    let (saved, ini) = match profile {
        Some(profile) => (
            update(&profile.join(SAVED_OPTIONS), |text| {
                saved_options(text, !fullscreen)
            }),
            update(&profile.join(ENGINE_INI), |text| {
                engine_ini(text, fullscreen, size)
            }),
        ),
        None => (FileOutcome::Missing, FileOutcome::Missing),
    };
    let mode = match display.mode {
        DisplayMode::Windowed => "windowed",
        DisplayMode::Fullscreen => "fullscreen",
    };
    events::display_settings(operation, mode, saved.code(), ini.code());
}

/// The Proton Play display step (PP-17, D-PP22, D-PP27): always a window. The
/// first Play seeds the saved options; later Plays edit them. Never fails Play.
#[cfg(any(target_os = "linux", test))]
pub(crate) fn apply_proton(operation: Uuid, pfx: &Path, profile: &Path, display: DisplayOptions) {
    let display = display.effective();
    let fullscreen = display.mode == DisplayMode::Fullscreen;
    let size = (!fullscreen).then_some((display.width, display.height));
    let saved = match profile.join(SAVED_OPTIONS).symlink_metadata() {
        Ok(_) => update(&profile.join(SAVED_OPTIONS), |text| {
            saved_options(text, !fullscreen)
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            seed(pfx, profile, !fullscreen)
        }
        Err(_) => FileOutcome::Unreadable,
    };
    let ini = update(&profile.join(ENGINE_INI), |text| {
        engine_ini(text, fullscreen, size)
    });
    let mode = match display.mode {
        DisplayMode::Windowed => "windowed",
        DisplayMode::Fullscreen => "fullscreen",
    };
    events::display_settings(operation, mode, saved.code(), ini.code());
}

/// Creates the saved options from the seed. Refused, and nothing written, when
/// a folder from the profile up to the prefix is a symlink (a link out of the
/// prefix). Never overwrites a file that appeared meanwhile.
#[cfg(any(target_os = "linux", test))]
fn seed(pfx: &Path, profile: &Path, windowed: bool) -> FileOutcome {
    for folder in profile.ancestors() {
        if folder
            .symlink_metadata()
            .is_ok_and(|meta| meta.file_type().is_symlink())
        {
            return FileOutcome::Failed;
        }
        if folder == pfx {
            break;
        }
    }
    let Some(text) = saved_options(SEED_SAVED_OPTIONS, windowed) else {
        return FileOutcome::Failed;
    };
    if std::fs::create_dir_all(profile).is_err() {
        return FileOutcome::Failed;
    }
    match create_new(&profile.join(SAVED_OPTIONS), &text) {
        Ok(()) => FileOutcome::Seeded,
        Err(_) => FileOutcome::Failed,
    }
}

/// Writes a file that must not exist yet: a temp file in the same folder, then
/// a rename that refuses to replace anything.
#[cfg(any(target_os = "linux", test))]
fn create_new(path: &Path, text: &str) -> std::io::Result<()> {
    let folder = path.parent().ok_or(std::io::ErrorKind::NotFound)?;
    let mut temp = tempfile::NamedTempFile::new_in(folder)?;
    temp.write_all(text.as_bytes())?;
    temp.as_file().sync_all()?;
    temp.persist_noclobber(path).map_err(|error| error.error)?;
    Ok(())
}

/// Reads, transforms and atomically replaces one existing UTF-8 file. Never
/// creates it, and writes only when the content changes.
fn update(path: &Path, transform: impl FnOnce(&str) -> Option<String>) -> FileOutcome {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return FileOutcome::Missing,
        Err(_) => return FileOutcome::Unreadable,
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return FileOutcome::Unreadable;
    };
    let Some(next) = transform(&text) else {
        return FileOutcome::KeyMissing;
    };
    if next == text {
        return FileOutcome::Unchanged;
    }
    match replace(path, &next) {
        Ok(()) => FileOutcome::Written,
        Err(_) => FileOutcome::Failed,
    }
}

/// A temp file in the same folder, given the file's permissions, then a
/// rename over it. A read-only file is refused rather than replaced.
fn replace(path: &Path, text: &str) -> std::io::Result<()> {
    let folder = path.parent().ok_or(std::io::ErrorKind::NotFound)?;
    let permissions = std::fs::metadata(path)?.permissions();
    if permissions.readonly() {
        return Err(std::io::ErrorKind::PermissionDenied.into());
    }
    let mut temp = tempfile::NamedTempFile::new_in(folder)?;
    temp.write_all(text.as_bytes())?;
    temp.as_file().sync_all()?;
    std::fs::set_permissions(temp.path(), permissions)?;
    temp.persist(path).map_err(|error| error.error)?;
    Ok(())
}
