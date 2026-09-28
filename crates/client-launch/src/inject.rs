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

// The process lifecycle moved to `crate::process`; re-exported so
// `inject::create_process_suspended` and friends keep resolving.
pub use crate::process::*;

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
    /// The injector and the target differ in bitness. `inject_dll`
    /// passes the injector's own `LoadLibraryW` address to the target,
    /// which is only valid when both run the same kernel32: a 64-bit
    /// injector's address is not the 32-bit (WOW64) `SGW.exe`'s
    /// `LoadLibraryW`, and the remote call fails as a bare
    /// `RemoteLoadFailed` that does not say why. Refused before any
    /// remote call.
    #[error(
        "the injector is {injector_bits}-bit but the target process is {target_bits}-bit; \
         DLL injection into SGW.exe needs a 32-bit (i686) launcher build"
    )]
    BitnessMismatch {
        injector_bits: u32,
        target_bits: u32,
    },
}

/// Whether an injector can NOT hand its own `LoadLibraryW` address to the
/// target: true when they differ in bitness. `injector_32` is this
/// build's pointer width, `os_64` whether Windows itself is 64-bit, and
/// `target_wow64` what `IsWow64Process` says of the target. On a 64-bit
/// OS a 32-bit process is a WOW64 one; on a 32-bit OS everything is
/// 32-bit and nothing is WOW64.
pub fn bitness_mismatch(injector_32: bool, os_64: bool, target_wow64: bool) -> bool {
    let target_32 = !os_64 || target_wow64;
    injector_32 != target_32
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
// `process` is an opaque kernel HANDLE (a raw pointer type in
// windows-sys), never dereferenced by us — it is only handed to Win32
// APIs. The function has always been safe to call with a handle from
// `create_process_suspended`; keeping it non-`unsafe` preserves the
// launcher's existing call sites through the #685 extraction. This lint
// only started applying now that these modules moved into a
// clippy-covered crate (sgw-launcher is clippy-excluded).
#[cfg(windows)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
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

    check_bitness(process)?;

    // SAFETY: `process` is a kernel HANDLE owned by the caller; all
    // pointers below either come from kernel allocations we just
    // made or point into stack-owned buffers whose lifetimes
    // outlive their use here.
    unsafe {
        // Resolve LoadLibraryW in OUR address space. Because
        // kernel32.dll is mapped at the same base address in every
        // process of the same bitness on the same boot (it's loaded
        // before ASLR randomisation applies to the process image), the
        // address we find here is the same address as in the target —
        // saving a remote symbol-resolution dance. `check_bitness`
        // above is what makes "same bitness" hold: a WOW64 target has
        // its own 32-bit kernel32 and no 64-bit one at all.
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

/// Refuse to inject across a bitness boundary; see
/// [`InjectError::BitnessMismatch`].
#[cfg(windows)]
fn check_bitness(process: windows_sys::Win32::Foundation::HANDLE) -> Result<(), InjectError> {
    use windows_sys::Win32::Foundation::{FALSE, HANDLE};
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, IsWow64Process};

    let is_wow64 = |handle: HANDLE| -> Result<bool, InjectError> {
        let mut flag = FALSE;
        // SAFETY: `handle` is a live process handle (the caller's, or the
        // pseudo-handle for this process) and `flag` outlives the call.
        if unsafe { IsWow64Process(handle, &mut flag) } == FALSE {
            return Err(InjectError::Win32 {
                api: "IsWow64Process",
                code: get_last_error(),
            });
        }
        Ok(flag != FALSE)
    };
    let injector_32 = cfg!(target_pointer_width = "32");
    // A 64-bit process only runs on a 64-bit OS; a 32-bit one is on a
    // 64-bit OS exactly when it is itself WOW64.
    // SAFETY: `GetCurrentProcess` returns a pseudo-handle; no preconditions.
    let os_64 = !injector_32 || is_wow64(unsafe { GetCurrentProcess() })?;
    if bitness_mismatch(injector_32, os_64, is_wow64(process)?) {
        let bits = |is_32: bool| if is_32 { 32 } else { 64 };
        return Err(InjectError::BitnessMismatch {
            injector_bits: bits(injector_32),
            target_bits: bits(!injector_32),
        });
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) fn get_last_error() -> u32 {
    // SAFETY: `GetLastError` is a per-thread TLS read, no
    // preconditions.
    unsafe { windows_sys::Win32::Foundation::GetLastError() }
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

    /// A 64-bit launcher handing its own LoadLibraryW to the 32-bit
    /// SGW.exe cannot load anything; it must be refused up front.
    #[test]
    fn bitness_mismatch_refuses_64_bit_injector_into_wow64_target() {
        assert!(bitness_mismatch(false, true, true));
    }

    #[test]
    fn bitness_mismatch_accepts_same_bitness() {
        // 32-bit injector and 32-bit target, both WOW64 on a 64-bit OS.
        assert!(!bitness_mismatch(true, true, true));
        // 64-bit injector, native 64-bit target.
        assert!(!bitness_mismatch(false, true, false));
        // 32-bit OS: everything is 32-bit and nothing is WOW64.
        assert!(!bitness_mismatch(true, false, false));
    }

    #[test]
    fn bitness_mismatch_refuses_32_bit_injector_into_native_64_bit_target() {
        assert!(bitness_mismatch(true, true, false));
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
