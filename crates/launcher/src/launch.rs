//! Launch + Atera detection.
//!
//! The launcher detects three on-disk artifacts in the install directory
//! and shows a launch button for each:
//!
//! - `SGW.exe` → main game (always available)
//! - `AteraLoader.exe` + `AtreaGameDebug.bat` → debug build via the Atera
//!   DLL injector (lets developers see Mercury / appearance / localization
//!   logging). Requires ASLR disabled on SGW.exe.
//! - `AtreaFixASLR.bat` → patches `IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE`
//!   off in SGW.exe so the Atera injector's hardcoded addresses resolve.

use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("File not found: {0}")]
    NotFound(PathBuf),
    #[error("Failed to spawn process: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("Launch target {target} escapes the install directory {install_dir}")]
    PathEscape {
        install_dir: PathBuf,
        target: PathBuf,
    },
}

#[derive(Debug, Clone, Default)]
pub struct LaunchOptions {
    pub sgw_present: bool,
    pub atera_loader_present: bool,
    pub atera_debug_bat_present: bool,
    pub atera_fix_aslr_bat_present: bool,
}

impl LaunchOptions {
    pub fn detect(install_dir: &Path) -> Self {
        Self {
            sgw_present: install_dir.join("SGW.exe").exists(),
            atera_loader_present: install_dir.join("AteraLoader.exe").exists(),
            atera_debug_bat_present: install_dir.join("AtreaGameDebug.bat").exists(),
            atera_fix_aslr_bat_present: install_dir.join("AtreaFixASLR.bat").exists(),
        }
    }

    /// True when both the loader and the debug bat are present. ASLR must
    /// have been disabled first (separate Fix ASLR button) but that's a
    /// one-time setup, not a per-launch requirement.
    pub fn atera_available(&self) -> bool {
        self.atera_loader_present && self.atera_debug_bat_present
    }
}

pub fn launch_sgw(install_dir: &Path) -> Result<u32, LaunchError> {
    spawn(install_dir, "SGW.exe", false)
}

pub fn launch_atera_debug(install_dir: &Path) -> Result<u32, LaunchError> {
    spawn(install_dir, "AtreaGameDebug.bat", true)
}

pub fn launch_atera_fix_aslr(install_dir: &Path) -> Result<u32, LaunchError> {
    spawn(install_dir, "AtreaFixASLR.bat", true)
}

fn spawn(install_dir: &Path, file: &str, via_cmd: bool) -> Result<u32, LaunchError> {
    let path = install_dir.join(file);
    if !path.exists() {
        return Err(LaunchError::NotFound(path));
    }
    // Defence-in-depth (Cady #4h): canonicalize both the install dir and
    // the target path, then check that the target is still under the
    // install dir. `install_dir` comes from the user-editable config —
    // this isn't a privilege boundary (the user chose the path) but
    // catches accidents like a config entry pointing into a junction
    // that resolves outside its declared root, which would otherwise
    // let an Atrea bat in an unexpected location run with the install
    // dir's cwd.
    let canon_install = install_dir.canonicalize()?;
    let canon_target = path.canonicalize()?;
    if !canon_target.starts_with(&canon_install) {
        return Err(LaunchError::PathEscape {
            install_dir: canon_install,
            target: canon_target,
        });
    }
    let child = if via_cmd {
        // `.bat` files need `cmd.exe /C` to spawn properly on Windows.
        let mut c = Command::new("cmd");
        c.arg("/C").arg(&canon_target).current_dir(&canon_install);
        c.spawn()?
    } else {
        Command::new(&canon_target)
            .current_dir(&canon_install)
            .spawn()?
    };
    Ok(child.id())
}

/// Best-effort writability probe: tries to create + remove a tiny file
/// in `dir`. Used by the install panel to disable the Install / Update
/// button when the chosen install directory isn't writable (e.g. user
/// picked `C:\Program Files\…` without UAC elevation), instead of
/// letting the download succeed and the extract fail opaquely.
pub fn install_dir_writable(dir: &Path) -> bool {
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".launcher-write-probe");
    let ok = std::fs::write(&probe, b"x").is_ok();
    let _ = std::fs::remove_file(&probe);
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_finds_only_present_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), "").unwrap();
        std::fs::write(dir.path().join("AteraLoader.exe"), "").unwrap();
        let opts = LaunchOptions::detect(dir.path());
        assert!(opts.sgw_present);
        assert!(opts.atera_loader_present);
        assert!(!opts.atera_debug_bat_present);
        assert!(!opts.atera_fix_aslr_bat_present);
        assert!(!opts.atera_available());
    }

    #[test]
    fn detect_marks_atera_available_when_pair_present() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("AteraLoader.exe"), "").unwrap();
        std::fs::write(dir.path().join("AtreaGameDebug.bat"), "").unwrap();
        let opts = LaunchOptions::detect(dir.path());
        assert!(opts.atera_available());
    }

    #[test]
    fn launch_errors_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let err = launch_sgw(dir.path()).unwrap_err();
        assert!(matches!(err, LaunchError::NotFound(_)));
    }

    #[test]
    fn detect_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let opts = LaunchOptions::detect(dir.path());
        assert!(!opts.sgw_present);
        assert!(!opts.atera_available());
    }

    #[test]
    fn install_dir_writable_succeeds_on_temp() {
        let dir = tempfile::tempdir().unwrap();
        assert!(install_dir_writable(dir.path()));
        // Probe file should not be left behind.
        assert!(!dir.path().join(".launcher-write-probe").exists());
    }

    #[test]
    fn install_dir_writable_creates_missing_parents() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested").join("install");
        assert!(install_dir_writable(&nested));
        assert!(nested.exists());
    }
}
