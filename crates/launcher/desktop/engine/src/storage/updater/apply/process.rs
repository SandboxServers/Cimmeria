//! Native process handoff and same-volume exclusive rename primitives.
use super::Error;
use std::{fs, path::Path};
/// Start the relaunched launcher. CreateProcess on Windows: the launcher
/// starts itself, needs no elevation, and a missing file fails synchronously
/// instead of raising a shell error dialog (the ShellExecute stall, #1194).
pub(super) fn spawn(path: &Path, args: &[String]) -> Result<(), Error> {
    use std::process::{Command, Stdio};
    let mut command = Command::new(path);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        command.current_dir(dir);
    }
    command.spawn().map_err(|error| {
        tracing::warn!(target: "launcher.update", event = "relaunch_failed", %error, path = %path.display(), "could not start the updated launcher");
        Error::Spawn
    })?;
    Ok(())
}
/// Whether a process with this id is still running. The restarted launcher
/// polls this for the old one before it takes the single-instance guard.
/// A process this user cannot open counts as gone; the state lock still
/// serializes the two.
pub fn process_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_TIMEOUT},
            System::Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
        };
        // SAFETY: a null handle is checked; the handle is closed once.
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        if handle.is_null() {
            return false;
        }
        let alive = unsafe { WaitForSingleObject(handle, 0) } == WAIT_TIMEOUT;
        unsafe { CloseHandle(handle) };
        alive
    }
    #[cfg(target_os = "macos")]
    {
        let Ok(pid) = libc::pid_t::try_from(pid) else {
            return false;
        };
        // SAFETY: signal 0 only checks that the process exists. EPERM means
        // it exists but belongs to another user.
        (unsafe { libc::kill(pid, 0) } == 0)
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = pid;
        false
    }
}
pub(super) fn sync_parent(_path: &Path) -> Result<(), Error> {
    #[cfg(unix)]
    fs::File::open(_path.parent().ok_or(Error::Target)?)
        .and_then(|f| f.sync_all())
        .map_err(|_| Error::Replace)?;
    Ok(())
}
#[cfg(test)]
thread_local! { pub(super) static FAIL_FINAL_RENAME: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
pub(in crate::storage::updater) fn rename_new(from: &Path, to: &Path) -> Result<(), Error> {
    #[cfg(test)]
    if from
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name.to_string_lossy().starts_with(".cimmeria-update-"))
        && FAIL_FINAL_RENAME.with(|flag| flag.replace(false))
    {
        return Err(Error::Replace);
    }
    #[cfg(target_os = "macos")]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let from = CString::new(from.as_os_str().as_bytes()).map_err(|_| Error::Target)?;
        let to = CString::new(to.as_os_str().as_bytes()).map_err(|_| Error::Target)?;
        if unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_EXCL) } != 0 {
            return Err(Error::Replace);
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        // Windows rename does not replace an existing destination. Unix bundle
        // replacement is supported only on macOS, never this fallback.
        if to.symlink_metadata().is_ok() {
            return Err(Error::Replace);
        }
        fs::rename(from, to).map_err(|_| Error::Replace)
    }
}
/// Mac publication and rollback keep the installed name present atomically.
pub(super) fn exchange(left: &Path, right: &Path) -> Result<(), Error> {
    #[cfg(target_os = "macos")]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let left = CString::new(left.as_os_str().as_bytes()).map_err(|_| Error::Target)?;
        let right = CString::new(right.as_os_str().as_bytes()).map_err(|_| Error::Target)?;
        if unsafe { libc::renamex_np(left.as_ptr(), right.as_ptr(), libc::RENAME_SWAP) } != 0 {
            return Err(Error::Replace);
        }
        Ok(())
    }
    #[cfg(all(not(target_os = "macos"), test))]
    {
        // Foreign-platform tests exercise the state machine only. This is never
        // available in production and is not evidence of Mac atomicity.
        let held = left.with_extension(format!("swap-fixture-{}", uuid::Uuid::new_v4()));
        fs::rename(left, &held).map_err(|_| Error::Replace)?;
        fs::rename(right, left).map_err(|_| Error::Replace)?;
        fs::rename(held, right).map_err(|_| Error::Replace)
    }
    #[cfg(all(not(target_os = "macos"), not(test)))]
    {
        let _ = (left, right);
        Err(Error::Platform)
    }
}
