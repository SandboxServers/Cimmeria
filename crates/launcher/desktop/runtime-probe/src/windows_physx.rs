//! Experimental ABI probe, isolated by the native parent's deadline/process scope.
use crate::physx::{self, Descriptor, SdkResult};
use sha2::{Digest, Sha256};
use std::{ffi::c_void, io::Read, os::windows::fs::OpenOptionsExt, path::Path};
use windows_sys::Win32::{
    Foundation::*, Storage::FileSystem::FILE_SHARE_READ, System::LibraryLoader::*,
};

const LOADER_SHA256: &str = "863e3ec87198bf1a5d5638a20695529dacc9460b0939f2579fe7a7faad2af924";
type Create =
    unsafe extern "C" fn(u32, *mut c_void, *mut c_void, *const Descriptor, *mut u32) -> *mut c_void;
type Release = unsafe extern "C" fn(*mut c_void);
struct Module(HMODULE);
impl Drop for Module {
    fn drop(&mut self) {
        unsafe {
            FreeLibrary(self.0);
        }
    }
}

pub fn probe(directory: &Path) -> SdkResult {
    let path = directory.join("PhysXLoader.dll");
    // Deny writes/deletion across hash verification, load and SDK teardown.
    let Ok(mut file) = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&path)
    else {
        return SdkResult::UnverifiedLoader {};
    };
    if !file
        .metadata()
        .is_ok_and(|m| m.is_file() && m.len() == 52256)
    {
        return SdkResult::UnverifiedLoader {};
    }
    let mut bytes = Vec::new();
    if (&mut file).take(52257).read_to_end(&mut bytes).is_err()
        || format!("{:x}", Sha256::digest(&bytes)) != LOADER_SHA256
    {
        return SdkResult::UnverifiedLoader {};
    }
    let Ok(name) = super::windows::wide(path.as_os_str()) else {
        return SdkResult::UnverifiedLoader {};
    };
    let module = unsafe { LoadLibraryExW(name.as_ptr(), std::ptr::null_mut(), 0) };
    if module.is_null() {
        return SdkResult::LoadFailed {
            win32_error: unsafe { GetLastError() },
        };
    }
    let module = Module(module);
    let create = unsafe { GetProcAddress(module.0, c"NxCreatePhysicsSDK".as_ptr().cast()) };
    let release = unsafe { GetProcAddress(module.0, c"NxReleasePhysicsSDK".as_ptr().cast()) };
    let (Some(create), Some(release)) = (create, release) else {
        return SdkResult::MissingExport {};
    };
    // Exact original loader ABI is pinned above; both functions use cdecl on x86.
    let create: Create = unsafe { std::mem::transmute(create) };
    let release: Release = unsafe { std::mem::transmute(release) };
    physx::exercise(
        |version, descriptor, error| unsafe {
            // Null default allocator/output is documented for SDK2.8, but remains
            // experimental until exercised against this exact2.6.3 core.
            create(
                version,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                descriptor,
                error,
            )
        },
        |sdk| unsafe { release(sdk) },
    )
}
