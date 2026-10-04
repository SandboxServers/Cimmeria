//! Native process handoff and same-volume exclusive rename primitives.
use super::Error;
use std::{
    fs,
    path::{Path, PathBuf},
};
pub(super) fn spawn(path: &Path, args: &[String]) -> Result<(), Error> {
    #[cfg(not(windows))]
    {
        use std::process::{Command, Stdio};
        Command::new(path)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| Error::Spawn)?;
        Ok(())
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::{
            Foundation::CloseHandle,
            UI::{
                Shell::{
                    ShellExecuteExW, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
                },
                WindowsAndMessaging::SW_SHOW,
            },
        };
        let file: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        // NSIS documents /D as the final, unquoted argument. All other native
        // arguments use Windows quoting; neither arguments nor paths cross IPC.
        let parameters = args
            .iter()
            .map(|arg| {
                if arg.starts_with("/D=") {
                    arg.clone()
                } else {
                    format!("\"{}\"", arg.replace('"', "\\\""))
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let parameters: Vec<u16> = parameters.encode_utf16().chain(Some(0)).collect();
        let verb: Vec<u16> = "open".encode_utf16().chain(Some(0)).collect();
        let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = parameters.as_ptr();
        info.nShow = SW_SHOW;
        if unsafe { ShellExecuteExW(&mut info) } == 0 {
            return Err(Error::Spawn);
        }
        if !info.hProcess.is_null() {
            unsafe { CloseHandle(info.hProcess) };
        }
        Ok(())
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
pub(super) fn msiexec() -> Result<PathBuf, Error> {
    #[cfg(windows)]
    {
        let mut buffer = [0u16; 32768];
        let len = unsafe {
            windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW(
                buffer.as_mut_ptr(),
                buffer.len() as u32,
            )
        } as usize;
        if len == 0 || len >= buffer.len() {
            return Err(Error::Target);
        }
        use std::os::windows::ffi::OsStringExt;
        Ok(PathBuf::from(std::ffi::OsString::from_wide(&buffer[..len])).join("msiexec.exe"))
    }
    #[cfg(not(windows))]
    {
        Err(Error::Platform)
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
