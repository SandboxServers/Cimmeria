//! DLL injection for `cimmeria-client-telemetry`.
//!
//! Side-loads a Rust-built cdylib into a Windows process via the
//! standard `CreateProcess(SUSPENDED)` → `VirtualAllocEx` →
//! `WriteProcessMemory` → `CreateRemoteThread(LoadLibraryW)` →
//! `ResumeThread` sequence. Used to attach
//! `cimmeria-client-telemetry.dll` to `SGW.exe` at game start for
//! client-side observability (issue #417).
//!
//! # Trust model
//!
//! Same machine, same user, same launcher session. The launcher is
//! already trusted to run arbitrary code (it's a desktop app the dev
//! installed); the injection path doesn't escalate. The DLL path is
//! checked for existence + UTF-16 convertibility before any kernel
//! call, but no signature verification — that's a deployment-time
//! concern (sign the DLL alongside the launcher) rather than a
//! per-injection one.
//!
//! # Why authored from scratch
//!
//! AteraLoader.exe / AtreaRL.dll already inject *something* into
//! SGW.exe in the debug-bat path, but they're third-party binaries
//! we can't extend (per project preference — see
//! [`docs/architecture/client-telemetry.md`]). Their behaviour is a
//! useful reference, their code is not in our supply chain.

use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum InjectError {
    #[error("DLL not found on disk: {0}")]
    DllMissing(PathBuf),
    #[error(
        "DLL path is too long for the injector (max {max} UTF-16 code units including NUL; got {got})"
    )]
    DllPathTooLong { got: usize, max: usize },
    #[error("DLL path contains a NUL byte: {0}")]
    DllPathHasNul(PathBuf),
    #[cfg(windows)]
    #[error("Win32 API {api} failed: GetLastError = {code}")]
    Win32 { api: &'static str, code: u32 },
    #[cfg(windows)]
    #[error("LoadLibraryW returned NULL in the target process (DLL failed to load or DllMain returned FALSE)")]
    RemoteLoadFailed,
}

/// Maximum UTF-16 code units we'll write into the remote process,
/// including the trailing NUL. The Windows `LoadLibraryW` API itself
/// supports `MAX_PATH = 260` by default; opt-in `\\?\` prefixing
/// raises the cap, but our launcher already writes the DLL alongside
/// itself in a path well under that, so we don't take on the extra
/// complexity here.
const MAX_DLL_PATH_W: usize = 260;

/// Encode a filesystem path as a NUL-terminated UTF-16 byte buffer
/// suitable for `WriteProcessMemory` → `LoadLibraryW`. Returns the
/// buffer as a `Vec<u16>` so the caller controls the lifetime
/// (Win32 wants a pointer + byte count, not a `&CStr`-style view).
///
/// Factored out as a pure function so unit tests can pin the
/// encoding rules without spawning a target process.
pub fn encode_dll_path_w(dll_path: &Path) -> Result<Vec<u16>, InjectError> {
    if !dll_path.exists() {
        return Err(InjectError::DllMissing(dll_path.to_path_buf()));
    }

    // Refuse interior NULs early. `OsStr::encode_wide` on Windows
    // would happily encode them, and the kernel would silently
    // truncate at the first NUL — meaning `LoadLibraryW` would try
    // to load a different (likely nonexistent) DLL than what the
    // caller asked for. That's exactly the kind of "the call
    // appeared to succeed" bug that costs hours to triage.
    if dll_path.as_os_str().to_string_lossy().contains('\0') {
        return Err(InjectError::DllPathHasNul(dll_path.to_path_buf()));
    }

    let mut wide: Vec<u16> = encode_wide(dll_path);
    wide.push(0);
    check_wide_len(wide.len())?;
    Ok(wide)
}

/// Pure length validator — factored out so unit tests can exercise
/// the `DllPathTooLong` path without depending on the host
/// filesystem's name-length cap (Windows refuses to create a file
/// whose component is longer than NAME_MAX, which is well below
/// MAX_DLL_PATH_W).
fn check_wide_len(len: usize) -> Result<(), InjectError> {
    if len > MAX_DLL_PATH_W {
        Err(InjectError::DllPathTooLong {
            got: len,
            max: MAX_DLL_PATH_W,
        })
    } else {
        Ok(())
    }
}

#[cfg(windows)]
fn encode_wide(path: &Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    path.as_os_str().encode_wide().collect()
}

#[cfg(not(windows))]
fn encode_wide(path: &Path) -> Vec<u16> {
    // Linux CI builds this crate. The unit tests for the validation
    // helpers run on Linux too, so we need a path → UTF-16 fallback
    // for them. Lossy is fine here — the encoding only matters on
    // Windows, where `encode_wide` is exact.
    path.to_string_lossy().encode_utf16().collect()
}

/// Inject `dll_path` into the process referenced by `process` and
/// wait for the remote `LoadLibraryW` to return.
///
/// **Caller responsibilities:**
/// 1. The process MUST be created suspended (`CREATE_SUSPENDED`) so
///    the injected DLL's `DllMain` runs before any of the target's
///    own threads. Inject, then `ResumeThread` the main thread.
/// 2. The caller owns `process` and is responsible for closing it.
/// 3. The DLL at `dll_path` must be a Windows DLL matching the
///    target's bitness (i686 for SGW.exe, which is 32-bit).
///
/// On success, `cimmeria-client-telemetry.dll` is loaded in the
/// target process, its `DllMain` has run, and its bootstrap thread
/// is spawned. The caller may now `ResumeThread` the target's
/// suspended main thread.
#[cfg(windows)]
pub fn inject_dll(
    process: windows_sys::Win32::Foundation::HANDLE,
    dll_path: &Path,
) -> Result<(), InjectError> {
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::Debug::WriteProcessMemory;
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    use windows_sys::Win32::System::Memory::{
        VirtualAllocEx, VirtualFreeEx, MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE,
    };
    use windows_sys::Win32::System::Threading::{
        CreateRemoteThread, GetExitCodeThread, WaitForSingleObject, INFINITE,
    };

    if process.is_null() || process == INVALID_HANDLE_VALUE {
        return Err(InjectError::Win32 {
            api: "inject_dll",
            code: 0,
        });
    }

    let wide = encode_dll_path_w(dll_path)?;
    let wide_bytes = wide.len() * std::mem::size_of::<u16>();

    // SAFETY: `process` is a kernel HANDLE owned by the caller; all
    // pointers below either come from kernel allocations we just
    // made or point into stack-owned buffers whose lifetimes
    // outlive their use here.
    unsafe {
        // Resolve LoadLibraryW in OUR address space. Because
        // kernel32.dll is mapped at the same base address in every
        // process on the same OS (it's loaded before ASLR
        // randomisation applies to the process image), the address
        // we find here is the same address as in the target —
        // saving a remote symbol-resolution dance.
        //
        // `c"..."` literals produce a `&CStr` with a NUL terminator
        // and avoid the easy-to-typo manual `b"...\0"` pattern.
        let kernel32 = GetModuleHandleA(c"kernel32.dll".as_ptr() as *const u8);
        if kernel32.is_null() {
            return Err(InjectError::Win32 {
                api: "GetModuleHandleA(kernel32)",
                code: get_last_error(),
            });
        }
        let load_library_w = GetProcAddress(kernel32, c"LoadLibraryW".as_ptr() as *const u8);
        let load_library_w_fn = match load_library_w {
            Some(f) => f,
            None => {
                return Err(InjectError::Win32 {
                    api: "GetProcAddress(LoadLibraryW)",
                    code: get_last_error(),
                });
            }
        };

        // Allocate writable memory in the remote process for the
        // DLL path. The remote thread reads it from there.
        let remote_buf = VirtualAllocEx(
            process,
            std::ptr::null(),
            wide_bytes,
            MEM_COMMIT | MEM_RESERVE,
            PAGE_READWRITE,
        );
        if remote_buf.is_null() {
            return Err(InjectError::Win32 {
                api: "VirtualAllocEx",
                code: get_last_error(),
            });
        }

        // Helper that releases the remote buffer on any error path
        // below. RAII would be cleaner, but our error-return shape
        // doesn't easily compose with Drop here; explicit cleanup
        // keeps the failure modes visible.
        let cleanup_buf = || {
            VirtualFreeEx(process, remote_buf, 0, MEM_RELEASE);
        };

        let mut written: usize = 0;
        let ok = WriteProcessMemory(
            process,
            remote_buf,
            wide.as_ptr() as *const _,
            wide_bytes,
            &mut written,
        );
        if ok == FALSE || written != wide_bytes {
            let code = get_last_error();
            cleanup_buf();
            return Err(InjectError::Win32 {
                api: "WriteProcessMemory",
                code,
            });
        }

        // Spawn a remote thread whose entry point is
        // LoadLibraryW(remote_buf). When LoadLibraryW returns, the
        // DLL's DllMain has run.
        //
        // We transmute the typed `unsafe extern "system" fn() -> isize`
        // that GetProcAddress hands back into the
        // `LPTHREAD_START_ROUTINE` shape CreateRemoteThread expects
        // (same calling convention, just different argument types
        // — the Win32 ABI doesn't care). The fully-qualified
        // type arguments are spelled out to satisfy clippy's
        // `missing_transmute_annotations` and to document the
        // exact conversion happening here.
        type Lpthread = unsafe extern "system" fn(lpparameter: *mut std::ffi::c_void) -> u32;
        type GetProcAddrFn = unsafe extern "system" fn() -> isize;
        let entry: Lpthread = std::mem::transmute::<GetProcAddrFn, Lpthread>(load_library_w_fn);

        let thread = CreateRemoteThread(
            process,
            std::ptr::null(),
            0,
            Some(entry),
            remote_buf,
            0,
            std::ptr::null_mut(),
        );
        if thread.is_null() {
            let code = get_last_error();
            cleanup_buf();
            return Err(InjectError::Win32 {
                api: "CreateRemoteThread",
                code,
            });
        }

        // Wait for LoadLibraryW to return in the remote process.
        // INFINITE is safe here: we hold the only handle to the
        // remote thread, and it's running our own (very short) DLL
        // load — if it hangs, the host process is already broken.
        WaitForSingleObject(thread, INFINITE);

        // Check what LoadLibraryW returned in the remote process.
        // On Win32, GetExitCodeThread() yields the thread's return
        // value, which for our entry point is the HMODULE
        // LoadLibraryW returned (NULL = failure). This is the
        // standard 32-bit injection technique; the technique is
        // unsound on 64-bit (HMODULE is 64-bit, exit code is
        // 32-bit, so we'd lose the upper bits) — but SGW.exe is
        // 32-bit, so the technique is exact for our use case.
        let mut exit_code: u32 = 0;
        let got_exit = GetExitCodeThread(thread, &mut exit_code);
        CloseHandle(thread);
        cleanup_buf();
        if got_exit == FALSE {
            return Err(InjectError::Win32 {
                api: "GetExitCodeThread",
                code: get_last_error(),
            });
        }
        if exit_code == 0 {
            return Err(InjectError::RemoteLoadFailed);
        }
    }

    Ok(())
}

#[cfg(windows)]
fn get_last_error() -> u32 {
    // SAFETY: `GetLastError` is a per-thread TLS read, no
    // preconditions.
    unsafe { windows_sys::Win32::Foundation::GetLastError() }
}

/// A child process that was created with `CREATE_SUSPENDED` and is
/// waiting on `ResumeThread` to start executing user code. Held as
/// an RAII guard so the kernel handles are released even on early-
/// return error paths.
///
/// Typical lifecycle:
/// 1. [`create_process_suspended`] returns this.
/// 2. Caller invokes [`inject_dll`] with `self.process_handle()`.
/// 3. Caller calls [`SuspendedProcess::resume`] to start the
///    target's main thread.
///
/// Dropping without calling [`resume`] leaves the child process
/// suspended forever — the OS reclaims it when the launcher exits,
/// but if the launcher keeps running you've leaked a zombie. The
/// `Drop` impl closes the handles either way; it's the caller's
/// responsibility to call `resume` (or explicitly terminate) on the
/// happy path.
#[cfg(windows)]
pub struct SuspendedProcess {
    process_handle: windows_sys::Win32::Foundation::HANDLE,
    thread_handle: windows_sys::Win32::Foundation::HANDLE,
    pid: u32,
}

#[cfg(windows)]
impl SuspendedProcess {
    /// Process ID of the spawned (suspended) target.
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Raw process HANDLE — feed this into [`inject_dll`].
    pub fn process_handle(&self) -> windows_sys::Win32::Foundation::HANDLE {
        self.process_handle
    }

    /// Resume the suspended main thread, allowing the target to
    /// begin executing user code. Consumes self so the handles are
    /// closed exactly once on the happy path. Returns the
    /// suspended-thread previous-suspend-count from `ResumeThread`
    /// (almost always `1` for a freshly-suspended process; anything
    /// else indicates an unexpected re-suspension).
    pub fn resume(self) -> Result<u32, InjectError> {
        use windows_sys::Win32::System::Threading::ResumeThread;

        // SAFETY: thread_handle was returned by CreateProcessW and
        // hasn't been closed yet (Drop runs after this).
        let prev = unsafe { ResumeThread(self.thread_handle) };
        if prev == u32::MAX {
            return Err(InjectError::Win32 {
                api: "ResumeThread",
                code: get_last_error(),
            });
        }
        Ok(prev)
    }
}

#[cfg(windows)]
impl Drop for SuspendedProcess {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        // SAFETY: HANDLEs are owned by self; we close them once.
        unsafe {
            if !self.thread_handle.is_null() {
                CloseHandle(self.thread_handle);
            }
            if !self.process_handle.is_null() {
                CloseHandle(self.process_handle);
            }
        }
    }
}

/// Spawn `exe_path` suspended (`CREATE_SUSPENDED`) so the caller
/// can inject a DLL before any of the target's user-mode threads
/// run. The returned [`SuspendedProcess`] owns the kernel handles.
///
/// `cwd` sets the target's working directory; pass `None` to inherit
/// the launcher's. SGW.exe is path-sensitive (it resolves `..\..\Game`
/// from cwd), so the launcher always passes `Some(install_dir)`.
///
/// No arguments are passed to the target. SGW.exe's launch path
/// doesn't take any.
#[cfg(windows)]
pub fn create_process_suspended(
    exe_path: &Path,
    cwd: Option<&Path>,
) -> Result<SuspendedProcess, InjectError> {
    use windows_sys::Win32::Foundation::{FALSE, TRUE};
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, CREATE_SUSPENDED, CREATE_UNICODE_ENVIRONMENT, PROCESS_INFORMATION,
        STARTUPINFOW,
    };

    if !exe_path.exists() {
        return Err(InjectError::DllMissing(exe_path.to_path_buf()));
    }

    // CreateProcessW reads lpCommandLine as MUTABLE wide-char data
    // (it can rewrite the buffer in place), so we build it as a
    // Vec<u16> rather than a string literal pointer. Quote the
    // path so embedded spaces aren't interpreted as separators.
    let exe_wide: Vec<u16> = {
        use std::os::windows::ffi::OsStrExt;
        let mut v: Vec<u16> = std::iter::once(b'"' as u16)
            .chain(exe_path.as_os_str().encode_wide())
            .chain(std::iter::once(b'"' as u16))
            .chain(std::iter::once(0))
            .collect();
        v.shrink_to_fit();
        v
    };
    let mut command_line = exe_wide.clone();

    let app_name: Vec<u16> = {
        use std::os::windows::ffi::OsStrExt;
        exe_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };

    let cwd_wide: Option<Vec<u16>> = cwd.map(|p| {
        use std::os::windows::ffi::OsStrExt;
        p.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    });
    let cwd_ptr = cwd_wide
        .as_ref()
        .map(|v| v.as_ptr())
        .unwrap_or(std::ptr::null());

    // SAFETY: Zero-init both Win32 structs. STARTUPINFOW's cb field
    // MUST be set to size_of so the kernel knows which fields exist.
    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    let mut pi: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };

    // SAFETY: All pointers either point to valid wide-string
    // buffers above or are NULL where the API allows it.
    let ok = unsafe {
        CreateProcessW(
            app_name.as_ptr(),
            command_line.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            FALSE,
            CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT,
            std::ptr::null(),
            cwd_ptr,
            &startup,
            &mut pi,
        )
    };
    if ok == TRUE {
        Ok(SuspendedProcess {
            process_handle: pi.hProcess,
            thread_handle: pi.hThread,
            pid: pi.dwProcessId,
        })
    } else {
        Err(InjectError::Win32 {
            api: "CreateProcessW",
            code: get_last_error(),
        })
    }
}

/// Non-Windows stub.
#[cfg(not(windows))]
pub fn create_process_suspended<'a>(
    exe_path: &Path,
    _cwd: Option<&Path>,
) -> Result<(), InjectError> {
    Err(InjectError::DllMissing(exe_path.to_path_buf()))
}

/// Non-Windows stub — the launcher crate compiles on Linux for
/// `cargo check`/`coverage` purposes, but `inject_dll` has no
/// meaning there. The Linux path returns a clear error so any
/// accidental cross-platform call site is immediately visible.
#[cfg(not(windows))]
pub fn inject_dll<H>(_process: H, _dll_path: &Path) -> Result<(), InjectError> {
    Err(InjectError::DllMissing(_dll_path.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_dll_path_w_rejects_missing_file() {
        let bogus = Path::new("/nonexistent/cimmeria-client-telemetry.dll");
        match encode_dll_path_w(bogus) {
            Err(InjectError::DllMissing(p)) => assert_eq!(p, bogus),
            other => panic!("expected DllMissing, got {other:?}"),
        }
    }

    #[test]
    fn encode_dll_path_w_appends_nul_terminator() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let encoded = encode_dll_path_w(tmp.path()).unwrap();
        assert_eq!(
            encoded.last(),
            Some(&0u16),
            "encoded UTF-16 buffer must end with a NUL code unit so \
             LoadLibraryW knows where the string ends"
        );
        assert!(
            encoded.len() > 1,
            "encoded buffer should contain more than just the NUL"
        );
    }

    #[test]
    fn check_wide_len_accepts_at_cap() {
        assert!(check_wide_len(MAX_DLL_PATH_W).is_ok());
        assert!(check_wide_len(1).is_ok());
    }

    #[test]
    fn check_wide_len_rejects_above_cap() {
        match check_wide_len(MAX_DLL_PATH_W + 1) {
            Err(InjectError::DllPathTooLong { got, max }) => {
                assert_eq!(got, MAX_DLL_PATH_W + 1);
                assert_eq!(max, MAX_DLL_PATH_W);
            }
            other => panic!("expected DllPathTooLong, got {other:?}"),
        }
    }

    #[test]
    fn encode_dll_path_w_succeeds_on_short_existing_path() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let encoded = encode_dll_path_w(tmp.path()).expect("short, existing path must encode");
        assert!(encoded.len() <= MAX_DLL_PATH_W);
    }
}
