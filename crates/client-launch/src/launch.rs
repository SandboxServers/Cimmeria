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
    #[error("DLL injection failed: {0}")]
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
    let (canon_install, canon_exe) = checked_sgw_exe(install_dir)?;

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

/// Launch `SGW.exe` suspended, inject each DLL in `dlls` in order, then
/// resume it. Each injection waits for the DLL's `DllMain` to return
/// before the next one starts, so the order is the order the DLLs'
/// bootstrap threads start in.
///
/// Unlike [`launch_sgw_with_telemetry`], a failed injection kills the
/// suspended process (it never ran user code) and returns the error, so
/// the caller can fall back to a plain [`launch_sgw`] without two
/// copies of the game. The returned [`inject::RunningProcess`] keeps
/// the process handle for waiting on the exit.
#[cfg(windows)]
pub fn launch_sgw_injected(
    install_dir: &Path,
    dlls: &[PathBuf],
) -> Result<inject::RunningProcess, LaunchError> {
    let (canon_install, canon_exe) = checked_sgw_exe(install_dir)?;
    let suspended = inject::create_process_suspended(&canon_exe, Some(&canon_install))?;
    for dll in dlls {
        if let Err(e) = inject::inject_dll(suspended.process_handle(), dll) {
            suspended.terminate();
            return Err(e.into());
        }
    }
    Ok(suspended.resume_running()?)
}

/// Non-Windows stub, like [`launch_sgw_with_telemetry`]'s.
#[cfg(not(windows))]
pub fn launch_sgw_injected(
    install_dir: &Path,
    dlls: &[PathBuf],
) -> Result<inject::RunningProcess, LaunchError> {
    let exe = install_dir.join("SGW.exe");
    if !exe.exists() {
        return Err(LaunchError::NotFound(exe));
    }
    Err(LaunchError::Inject(inject::InjectError::DllMissing(
        dlls.first().cloned().unwrap_or_default(),
    )))
}

/// [`launch_sgw`], returning the [`Child`] so a telemetry session can
/// wait on the game's exit.
pub fn launch_sgw_with_child(install_dir: &Path) -> Result<Child, LaunchError> {
    spawn(install_dir, "SGW.exe", false)
}

/// `Path::canonicalize` without the Windows verbatim prefix.
///
/// On Windows `canonicalize` returns `\\?\C:\...`. Windows does not
/// normalize `..` inside a verbatim path, and SGW.exe finds its config as
/// `..\SGWGame\Config\` relative to its working directory, so a verbatim
/// cwd makes it fail at startup with "Failed to find default engine .ini
/// file". A plain `C:\...` path names the same file.
pub fn canonical(path: &Path) -> std::io::Result<PathBuf> {
    Ok(strip_verbatim(path.canonicalize()?))
}

/// `\\?\C:\x` -> `C:\x` and `\\?\UNC\host\share` -> `\\host\share`;
/// anything else is returned unchanged.
pub fn strip_verbatim(path: PathBuf) -> PathBuf {
    let Some(s) = path.to_str() else {
        return path;
    };
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    match s.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => path,
    }
}

/// `install_dir/SGW.exe`, canonicalized, refusing a target that
/// resolves outside the install directory. Returns
/// `(install dir, SGW.exe)`, both canonical and without the verbatim
/// prefix (see [`canonical`]).
pub fn checked_sgw_exe(install_dir: &Path) -> Result<(PathBuf, PathBuf), LaunchError> {
    let exe = install_dir.join("SGW.exe");
    if !exe.exists() {
        return Err(LaunchError::NotFound(exe));
    }
    let canon_install = canonical(install_dir)?;
    let canon_exe = canonical(&exe)?;
    if !canon_exe.starts_with(&canon_install) {
        return Err(LaunchError::PathEscape {
            install_dir: canon_install,
            target: canon_exe,
        });
    }
    Ok((canon_install, canon_exe))
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

/// Same as [`launch_atera_fix_aslr`] but returns the [`Child`], so the
/// caller can hold off other work on `SGW.exe` until the bat finishes.
pub fn launch_atera_fix_aslr_with_child(install_dir: &Path) -> Result<Child, LaunchError> {
    spawn(install_dir, "AtreaFixASLR.bat", true)
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
    let canon_install = canonical(install_dir)?;
    let canon_target = canonical(&path)?;
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
    fn strip_verbatim_drops_the_prefix_from_disk_and_unc_paths_only() {
        assert_eq!(
            strip_verbatim(PathBuf::from(r"\\?\C:\SGW\Binaries")),
            PathBuf::from(r"C:\SGW\Binaries")
        );
        assert_eq!(
            strip_verbatim(PathBuf::from(r"\\?\UNC\host\share\SGW")),
            PathBuf::from(r"\\host\share\SGW")
        );
        assert_eq!(
            strip_verbatim(PathBuf::from(r"C:\SGW")),
            PathBuf::from(r"C:\SGW")
        );
        // A verbatim non-disk path (a volume GUID) has no plain spelling.
        let guid = PathBuf::from(r"\\?\Volume{0}\SGW");
        assert_eq!(strip_verbatim(guid.clone()), guid);
    }

    /// Regression guard: SGW.exe started with a `\\?\` working directory
    /// quits with "Failed to find default engine .ini file", because
    /// `..\SGWGame\Config` does not resolve under a verbatim path. Reverting
    /// `canonical` to a bare `canonicalize` fails this on Windows.
    #[test]
    fn checked_sgw_exe_returns_plain_paths() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("SGW.exe"), "").unwrap();
        let (install, exe) = checked_sgw_exe(dir.path()).unwrap();
        for p in [&install, &exe] {
            assert!(!p.to_string_lossy().starts_with(r"\\?\"), "{}", p.display());
        }
        assert!(exe.starts_with(&install));
    }

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

    /// No SGW.exe: refused before any process is created, so no DLL
    /// is touched and nothing is left suspended.
    #[cfg(windows)]
    #[test]
    fn launch_sgw_injected_errors_when_sgw_missing() {
        let dir = tempfile::tempdir().unwrap();
        let dll = dir.path().join("x.dll");
        std::fs::write(&dll, b"").unwrap();
        let err = launch_sgw_injected(dir.path(), &[dll]).unwrap_err();
        assert!(matches!(err, LaunchError::NotFound(_)), "got {err:?}");
    }

    /// A 32-bit stand-in for SGW.exe and a 32-bit DLL from `SysWOW64`,
    /// or `None` on a machine without WOW64.
    #[cfg(windows)]
    fn wow64_fixture(exe: &str) -> Option<(tempfile::TempDir, PathBuf)> {
        let wow = PathBuf::from(std::env::var("SystemRoot").ok()?).join("SysWOW64");
        let dll = wow.join("version.dll");
        if !wow.join(exe).is_file() || !dll.is_file() {
            return None;
        }
        let dir = tempfile::tempdir().unwrap();
        std::fs::copy(wow.join(exe), dir.path().join("SGW.exe")).unwrap();
        Some((dir, dll))
    }

    /// The real pipeline against a real 32-bit process: a 32-bit
    /// launcher loads the DLL and the game runs to completion; a 64-bit
    /// one refuses instead of crashing the target with its own
    /// LoadLibraryW address.
    #[cfg(windows)]
    #[test]
    fn launch_sgw_injected_into_a_32_bit_process() {
        let Some((dir, dll)) = wow64_fixture("hostname.exe") else {
            eprintln!("no SysWOW64 hostname.exe/version.dll; skipping");
            return;
        };
        let result = launch_sgw_injected(dir.path(), &[dll]);
        if cfg!(target_pointer_width = "32") {
            let process = result.expect("a 32-bit injector must load a 32-bit DLL");
            assert_eq!(
                process.wait().unwrap(),
                0,
                "hostname.exe should exit cleanly"
            );
        } else {
            match result {
                Err(LaunchError::Inject(inject::InjectError::BitnessMismatch {
                    injector_bits: 64,
                    target_bits: 32,
                })) => {}
                other => panic!("expected BitnessMismatch, got {other:?}"),
            }
        }
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
