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
    let child = if via_cmd {
        // `.bat` files need `cmd.exe /C` to spawn properly on Windows.
        let mut c = Command::new("cmd");
        c.arg("/C").arg(&path).current_dir(install_dir);
        c.spawn()?
    } else {
        Command::new(&path).current_dir(install_dir).spawn()?
    };
    Ok(child.id())
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
}
