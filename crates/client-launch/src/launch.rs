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
use std::process::{Child, Command};

use thiserror::Error;

use crate::inject;

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
    /// Telemetry-mode launch failed somewhere in the
    /// CreateProcessW(SUSPENDED) -> inject -> ResumeThread pipeline.
    /// Wrapped from [`inject::InjectError`] so callers see one error
    /// surface for the whole launch path.
    #[error("Telemetry launch failed: {0}")]
    Inject(#[from] inject::InjectError),
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
    spawn(install_dir, "SGW.exe", false).map(|c| c.id())
}

/// Launch `SGW.exe` with the `cimmeria-client-telemetry` DLL
/// injected before any user-mode threads run. Used when the player
/// has opted into client telemetry; the standard [`launch_sgw`]
/// path is left untouched for the non-telemetry default.
///
/// Sequence:
/// 1. `CreateProcessW(SGW.exe, CREATE_SUSPENDED)` — process exists
///    but no thread has executed yet.
/// 2. `inject::inject_dll(process, dll_path)` — allocates remote
///    memory, copies the DLL path, calls `LoadLibraryW` on a remote
///    thread, waits for `DllMain` to return.
/// 3. `ResumeThread(main)` — SGW.exe begins running with the
///    telemetry DLL already attached and its bootstrap thread
///    already spawned.
///
/// Returns the SGW.exe PID on success. The kernel handles are
/// closed by [`inject::SuspendedProcess`]'s `Drop` either way; this
/// function does not retain ownership of the spawned process —
/// launcher death does not kill the game.
#[cfg(windows)]
pub fn launch_sgw_with_telemetry(install_dir: &Path, dll_path: &Path) -> Result<u32, LaunchError> {
    let exe = install_dir.join("SGW.exe");
    if !exe.exists() {
        return Err(LaunchError::NotFound(exe));
    }
    let canon_install = install_dir.canonicalize()?;
    let canon_exe = exe.canonicalize()?;
    if !canon_exe.starts_with(&canon_install) {
        return Err(LaunchError::PathEscape {
            install_dir: canon_install,
            target: canon_exe,
        });
    }

    let suspended = inject::create_process_suspended(&canon_exe, Some(&canon_install))?;
    let pid = suspended.pid();

    // If injection fails, drop on `suspended` runs and closes the
    // handles — but the target stays suspended forever. We could
    // TerminateProcess here too, but a suspended zombie is more
    // diagnostic-friendly than a freshly-killed one: the operator
    // can attach a debugger to see what state the loader was in.
    // The launcher's session bookkeeping will eventually surface
    // the orphan via PID-already-gone checks.
    inject::inject_dll(suspended.process_handle(), dll_path)?;

    let _previous_suspend_count = suspended.resume()?;
    Ok(pid)
}

/// Non-Windows stub — no injection path exists on Linux/macOS;
/// errors so any accidental cross-platform call site is obvious.
#[cfg(not(windows))]
pub fn launch_sgw_with_telemetry(
    _install_dir: &Path,
    _dll_path: &Path,
) -> Result<u32, LaunchError> {
    Err(LaunchError::Inject(inject::InjectError::DllMissing(
        _dll_path.to_path_buf(),
    )))
}

pub fn launch_atera_debug(install_dir: &Path) -> Result<u32, LaunchError> {
    spawn(install_dir, "AtreaGameDebug.bat", true).map(|c| c.id())
}

pub fn launch_atera_fix_aslr(install_dir: &Path) -> Result<u32, LaunchError> {
    spawn(install_dir, "AtreaFixASLR.bat", true).map(|c| c.id())
}

/// Same as [`launch_atera_debug`] but returns the [`Child`] handle so
/// the telemetry pipeline can wait on game exit. Dropping the Child
/// does NOT kill the child process on std::process — launcher death
/// keeps the game alive.
pub fn launch_atera_debug_with_child(install_dir: &Path) -> Result<Child, LaunchError> {
    spawn(install_dir, "AtreaGameDebug.bat", true)
}

fn spawn(install_dir: &Path, file: &str, via_cmd: bool) -> Result<Child, LaunchError> {
    let path = install_dir.join(file);
    if !path.exists() {
        return Err(LaunchError::NotFound(path));
    }
    // Defence-in-depth: canonicalize both the install dir and the target
    // path, then check that the target is still under the install dir.
    // `install_dir` comes from the user-editable config —
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
    Ok(child)
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
