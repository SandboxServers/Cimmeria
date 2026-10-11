//! The real registry, system-directory and UAC calls behind `Probe` and `Elevate`.
use super::{Elevate, ElevateError, Probe};
use std::ffi::OsStr;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_CANCELLED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Registry::{
    RegGetValueW, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RRF_SUBKEY_WOW6432KEY,
};
use windows_sys::Win32::System::SystemInformation::{
    GetSystemDirectoryW, GetSystemWow64DirectoryW,
};
use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject, INFINITE};
use windows_sys::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_FLAG_NO_UI, SEE_MASK_NOASYNC, SEE_MASK_NOCLOSEPROCESS,
    SHELLEXECUTEINFOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

/// Written by PhysX System Software (AGEIA, later NVIDIA, 2.x cores included)
/// and read by the game's 32-bit `PhysXLoader.dll`.
const AGEIA_KEY: &str = r"SOFTWARE\AGEIA Technologies";
const CORE_PATH_VALUE: &str = "PhysXCore Path";

fn wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(Some(0)).collect()
}

fn directory(read: unsafe extern "system" fn(*mut u16, u32) -> u32) -> Option<PathBuf> {
    let mut buffer = [0u16; 260];
    // SAFETY: the buffer outlives the call and its length is passed in units.
    let units = unsafe { read(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    (units > 0 && units < buffer.len())
        .then(|| PathBuf::from(std::ffi::OsString::from_wide(&buffer[..units])))
}

/// The directory a 32-bit process gets as its system directory: `SysWOW64` on
/// 64-bit Windows (where `GetSystemWow64DirectoryW` succeeds), else `System32`.
fn system32_for_x86() -> Option<PathBuf> {
    directory(GetSystemWow64DirectoryW).or_else(|| directory(GetSystemDirectoryW))
}

pub struct System;
impl Probe for System {
    fn system_loader(&self) -> bool {
        // A metadata check, not LoadLibrary: a 32-bit DLL cannot load into the
        // 64-bit launcher, and loading would run its entry point.
        system32_for_x86().is_some_and(|dir| {
            std::fs::metadata(dir.join("PhysXLoader.dll")).is_ok_and(|m| m.is_file())
        })
    }

    fn core_path(&self) -> Option<PathBuf> {
        let key = wide(OsStr::new(AGEIA_KEY));
        let value = wide(OsStr::new(CORE_PATH_VALUE));
        let mut buffer = [0u16; 520];
        let mut bytes = std::mem::size_of_val(&buffer) as u32;
        // SAFETY: NUL-terminated names; the buffer outlives the call and `bytes`
        // is its size in bytes. RRF_SUBKEY_WOW6432KEY reads the 32-bit view,
        // which is the one the 32-bit loader sees.
        let status = unsafe {
            RegGetValueW(
                HKEY_LOCAL_MACHINE,
                key.as_ptr(),
                value.as_ptr(),
                RRF_RT_REG_SZ | RRF_SUBKEY_WOW6432KEY,
                std::ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
                &mut bytes,
            )
        };
        if status != 0 {
            return None;
        }
        let units = (bytes as usize / 2).min(buffer.len());
        let text = String::from_utf16_lossy(&buffer[..units]);
        let text = text.trim_end_matches('\0').trim();
        let path = PathBuf::from(text);
        (!text.is_empty() && path.is_dir()).then_some(path)
    }
}

/// `msiexec.exe /i "<msi>" /norestart`, elevated with the `runas` verb.
///
/// Full interactive UI, not `/passive` or `/qn`: the player reads and accepts
/// the vendor license in the installer itself; the launcher never accepts it for
/// them. `/norestart` leaves any restart to the player, which the result line
/// then asks for (exit code 3010).
pub struct ShellExecuteRunas;
impl Elevate for ShellExecuteRunas {
    fn run_msi(&self, msi: &Path) -> Result<u32, ElevateError> {
        // The full system path, so no search order can substitute another msiexec.
        let msiexec = directory(GetSystemDirectoryW)
            .ok_or(ElevateError::Win32(0))?
            .join("msiexec.exe");
        let file = wide(msiexec.as_os_str());
        let verb = wide(OsStr::new("runas"));
        let mut parameters = std::ffi::OsString::from("/i \"");
        parameters.push(msi.as_os_str());
        parameters.push("\" /norestart");
        let parameters = wide(&parameters);
        // SAFETY: an all-zero SHELLEXECUTEINFOW is a valid empty request.
        let mut info: SHELLEXECUTEINFOW = unsafe { std::mem::zeroed() };
        info.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        info.fMask = SEE_MASK_NOCLOSEPROCESS | SEE_MASK_NOASYNC | SEE_MASK_FLAG_NO_UI;
        info.lpVerb = verb.as_ptr();
        info.lpFile = file.as_ptr();
        info.lpParameters = parameters.as_ptr();
        info.nShow = SW_SHOWNORMAL;
        // SAFETY: every string is NUL-terminated and outlives the call.
        if unsafe { ShellExecuteExW(&mut info) } == 0 {
            let error = unsafe { GetLastError() };
            return Err(if error == ERROR_CANCELLED {
                ElevateError::Cancelled
            } else {
                ElevateError::Win32(error)
            });
        }
        if info.hProcess.is_null() {
            return Err(ElevateError::Win32(0));
        }
        // SAFETY: hProcess is the process handle SEE_MASK_NOCLOSEPROCESS returned;
        // it is closed exactly once below.
        unsafe {
            let mut code = 0u32;
            let waited = WaitForSingleObject(info.hProcess, INFINITE);
            let read = GetExitCodeProcess(info.hProcess, &mut code);
            let error = GetLastError();
            CloseHandle(info.hProcess);
            if waited != WAIT_OBJECT_0 || read == 0 {
                Err(ElevateError::Win32(error))
            } else {
                Ok(code)
            }
        }
    }
}
