//! Win32 IAT detours for thread, module and focus observations.
use super::*;

/// `HANDLE CreateThread(LPSECURITY_ATTRIBUTES, SIZE_T stack_size,
///   LPTHREAD_START_ROUTINE start, LPVOID parameter, DWORD flags,
///   LPDWORD out_thread_id)` — `__stdcall`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "stdcall-unwind" fn create_thread_detour(
    sec_attrs: *mut c_void,
    stack_size: usize,
    start_routine: *mut c_void,
    parameter: *mut c_void,
    flags: u32,
    out_thread_id: *mut u32,
) -> *mut c_void {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(
                crate::events::ClientNativeEvent::builder("client.os.create_thread", "info")
                    .field("stack_size", serde_json::json!(stack_size))
                    .field("flags", serde_json::json!(flags)),
            );
        }
    });

    let orig_addr = ORIG_CREATE_THREAD.load(Ordering::Acquire);
    if orig_addr == 0 {
        return std::ptr::null_mut();
    }
    let original: unsafe extern "stdcall-unwind" fn(
        *mut c_void,
        usize,
        *mut c_void,
        *mut c_void,
        u32,
        *mut u32,
    ) -> *mut c_void = unsafe { std::mem::transmute(orig_addr) };
    original(
        sec_attrs,
        stack_size,
        start_routine,
        parameter,
        flags,
        out_thread_id,
    )
}

/// `HMODULE LoadLibraryW(LPCWSTR lib_filename)` — `__stdcall`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "stdcall-unwind" fn load_library_w_detour(
    lib_filename: *const u16,
) -> *mut c_void {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            let name = crate::hooks::inline_hooks::read_utf16_bounded(lib_filename, 256);
            p.try_emit(
                crate::events::ClientNativeEvent::builder("client.os.load_library", "info")
                    .field("encoding", serde_json::json!("wide"))
                    .field("name", serde_json::Value::String(name)),
            );
        }
    });

    let orig_addr = ORIG_LOAD_LIBRARY_W.load(Ordering::Acquire);
    if orig_addr == 0 {
        return std::ptr::null_mut();
    }
    let original: unsafe extern "stdcall-unwind" fn(*const u16) -> *mut c_void =
        unsafe { std::mem::transmute(orig_addr) };
    original(lib_filename)
}

/// `HMODULE LoadLibraryA(LPCSTR lib_filename)` — `__stdcall`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "stdcall-unwind" fn load_library_a_detour(
    lib_filename: *const u8,
) -> *mut c_void {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            let name = read_ascii_bounded(lib_filename, 256);
            p.try_emit(
                crate::events::ClientNativeEvent::builder("client.os.load_library", "info")
                    .field("encoding", serde_json::json!("ansi"))
                    .field("name", serde_json::Value::String(name)),
            );
        }
    });

    let orig_addr = ORIG_LOAD_LIBRARY_A.load(Ordering::Acquire);
    if orig_addr == 0 {
        return std::ptr::null_mut();
    }
    let original: unsafe extern "stdcall-unwind" fn(*const u8) -> *mut c_void =
        unsafe { std::mem::transmute(orig_addr) };
    original(lib_filename)
}

/// `HWND GetForegroundWindow(void)` — `__stdcall`.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
pub(super) unsafe extern "stdcall-unwind" fn get_foreground_window_detour() -> *mut c_void {
    let _ = std::panic::catch_unwind(|| {
        if GET_FOREGROUND_WINDOW_SAMPLER.should_emit() {
            if let Some(p) = crate::boot::producer() {
                p.try_emit(crate::events::ClientNativeEvent::builder(
                    "client.os.get_foreground_window",
                    "debug",
                ));
            }
        }
    });

    // Lab virtual focus: report the game window as foreground so a lab
    // client in the background keeps processing the injected input.
    #[cfg(feature = "lab-bridge")]
    if let Some(hwnd) = crate::bridge::input::virtual_focus_hwnd() {
        return hwnd as *mut c_void;
    }
    let orig_addr = ORIG_GET_FOREGROUND_WINDOW.load(Ordering::Acquire);
    if orig_addr == 0 {
        return std::ptr::null_mut();
    }
    let original: unsafe extern "stdcall-unwind" fn() -> *mut c_void =
        unsafe { std::mem::transmute(orig_addr) };
    original()
}
/// ASCII-bounded string read for `LoadLibraryA` — same safety
/// contract as `inline_hooks::read_utf16_bounded` but for narrow
/// strings.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) fn read_ascii_bounded(ptr: *const u8, max_chars: usize) -> String {
    if ptr.is_null() {
        return "<null>".into();
    }
    let mut len = 0usize;
    while len < max_chars {
        // SAFETY: bounded by max_chars; caller's buffer is the
        // path passed to LoadLibraryA which is always NUL-terminated.
        let ch = unsafe { *ptr.add(len) };
        if ch == 0 {
            break;
        }
        len += 1;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
    String::from_utf8_lossy(slice).into_owned()
}
