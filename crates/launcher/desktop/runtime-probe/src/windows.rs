//! One-shot x86 process only. Search game and system directories; suppress loader
//! error dialogs. The SGW manifest supplies its actual side-by-side CRT policy.
use crate::{collect, LoadResult, Report, Request};
use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::Path};
use windows_sys::Win32::{
    Foundation::*,
    System::{
        ApplicationInstallationAndServicing::*, Diagnostics::Debug::*, LibraryLoader::*,
        WindowsProgramming::ACTCTX_FLAG_RESOURCE_NAME_VALID,
    },
};

fn wide(value: &OsStr) -> Result<Vec<u16>, &'static str> {
    let mut bytes: Vec<u16> = value.encode_wide().collect();
    if bytes.contains(&0) {
        return Err("invalid_directory");
    }
    bytes.push(0);
    Ok(bytes)
}
fn load(dll: &str) -> LoadResult {
    load_path(OsStr::new(dll))
}
fn load_path(dll: &OsStr) -> LoadResult {
    let name = match wide(dll) {
        Ok(name) => name,
        Err(_) => return LoadResult::Unavailable { win32_error: 123 },
    };
    let module = unsafe { LoadLibraryExW(name.as_ptr(), std::ptr::null_mut(), 0) };
    if module.is_null() {
        LoadResult::Unavailable {
            win32_error: unsafe { GetLastError() },
        }
    } else {
        unsafe { FreeLibrary(module) };
        LoadResult::Loaded {}
    }
}
struct Context {
    handle: HANDLE,
    cookie: usize,
}
impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            DeactivateActCtx(0, self.cookie);
            ReleaseActCtx(self.handle);
        }
    }
}
fn activation(exe: &Path) -> Result<Context, u32> {
    let source = wide(exe.as_os_str()).map_err(|_| 123_u32)?;
    // CREATEPROCESS_MANIFEST_RESOURCE_ID, an integer resource identifier,
    // not a dereferenceable pointer. Missing manifests report a context failure.
    let resource_id = 1_u16;
    let context = ACTCTXW {
        cbSize: std::mem::size_of::<ACTCTXW>() as u32,
        dwFlags: ACTCTX_FLAG_RESOURCE_NAME_VALID,
        lpSource: source.as_ptr(),
        lpResourceName: usize::from(resource_id) as *const u16,
        ..Default::default()
    };
    let handle = unsafe { CreateActCtxW(&context) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(unsafe { GetLastError() });
    }
    let mut cookie = 0;
    if unsafe { ActivateActCtx(handle, &mut cookie) } == 0 {
        let error = unsafe { GetLastError() };
        unsafe { ReleaseActCtx(handle) };
        return Err(error);
    }
    Ok(Context { handle, cookie })
}
struct SearchDirectory(*mut std::ffi::c_void);
impl Drop for SearchDirectory {
    fn drop(&mut self) {
        unsafe { RemoveDllDirectory(self.0) };
    }
}
pub fn probe(request: Request) -> Result<Report, &'static str> {
    let directory = request
        .game_binaries
        .canonicalize()
        .map_err(|_| "invalid_directory")?;
    if !directory.is_dir() || !directory.join("SGW.exe").is_file() {
        return Err("invalid_directory");
    }
    let path = wide(directory.as_os_str())?;
    unsafe { SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX) };
    if unsafe {
        SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_SYSTEM32 | LOAD_LIBRARY_SEARCH_USER_DIRS)
    } == 0
    {
        return Err("search_configuration_failed");
    }
    let cookie = unsafe { AddDllDirectory(path.as_ptr()) };
    if cookie.is_null() {
        return Err("search_configuration_failed");
    }
    let _search = SearchDirectory(cookie);
    let context = activation(&directory.join("SGW.exe"));
    let result = match &context {
        Ok(_) => LoadResult::Loaded {},
        Err(error) => LoadResult::Unavailable {
            win32_error: *error,
        },
    };
    Ok(collect(result, |dll| {
        if dll == "PhysXLoader.dll" {
            load_path(directory.join(dll).as_os_str())
        } else {
            load(dll)
        }
    }))
}

/// CI exercises actual x86 loader success/failure without game code or a window.
pub fn self_test() -> bool {
    unsafe { SetErrorMode(SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX) };
    load("kernel32.dll") == LoadResult::Loaded {}
        && matches!(
            load("cimmeria-missing-probe-fixture.dll"),
            LoadResult::Unavailable { .. }
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Scratch(std::path::PathBuf);
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    #[test]
    fn probe_configures_search_and_reports_missing_manifest_without_claiming_crt() {
        let path = std::env::temp_dir().join(format!(
            "cimmeria-probe-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        let scratch = Scratch(path);
        // Inert invalid image: CreateActCtx must reject it, never execute it.
        std::fs::write(scratch.0.join("SGW.exe"), b"not a PE image").unwrap();
        let report = probe(Request {
            schema_version: 1,
            game_binaries: scratch.0.clone(),
        })
        .unwrap();
        assert!(matches!(
            report.activation_context,
            LoadResult::Unavailable { .. }
        ));
        assert_eq!(report.modules[0].result, LoadResult::ContextUnavailable {});
        assert_eq!(report.modules[1].result, LoadResult::ContextUnavailable {});
        assert!(matches!(
            report.modules[4].result,
            LoadResult::Unavailable { .. }
        ));
        assert!(!report.physx_engine_checked && !report.game_started);
        assert!(self_test());
    }
}
