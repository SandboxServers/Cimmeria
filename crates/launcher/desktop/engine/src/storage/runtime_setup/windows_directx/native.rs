//! Windows system-directory selection and interactive Microsoft installer launch.
use super::*;
use windows_sys::Win32::System::SystemInformation::{
    GetSystemDirectoryW, GetSystemWow64Directory2W,
};
pub struct System;
impl Probe for System {
    fn x86_directory(&self) -> Option<PathBuf> {
        use std::os::windows::ffi::OsStringExt;
        let mut buffer = [0u16; 32768];
        // IMAGE_FILE_MACHINE_I386: never mistake ARM64/x64 DLLs for SGW's x86 dependencies.
        let mut len =
            unsafe { GetSystemWow64Directory2W(buffer.as_mut_ptr(), buffer.len() as u32, 0x014c) };
        if len == 0 && cfg!(target_arch = "x86") {
            len = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) };
        }
        (len > 0 && (len as usize) < buffer.len())
            .then(|| PathBuf::from(std::ffi::OsString::from_wide(&buffer[..len as usize])))
    }
}

pub fn run(installer: &Path) -> Result<u32, super::super::windows_physx::ElevateError> {
    super::super::windows_physx::native::run_elevated(installer, None)
}
