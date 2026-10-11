//! The exit hooks (TG-PIPE-03): `ExitProcess` and the CRT's `exit`.
//!
//! Each detour runs [`crate::shutdown::flush_at_exit`] (a bounded wait for
//! the uploader's last POST) and then chains to the real function, which
//! does not return. Why these two slots, and why not `DLL_PROCESS_DETACH`:
//! see the exit-path notes in [`crate::shutdown`].
//!
//! Both detours use `-unwind` ABIs like every other IAT detour here, and
//! catch panics from the flush: a bug in telemetry must never keep the game
//! from exiting.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::shutdown::{flush_at_exit, ExitReason};

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) static ORIG_EXIT_PROCESS: AtomicUsize = AtomicUsize::new(0);
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) static ORIG_CRT_EXIT: AtomicUsize = AtomicUsize::new(0);

/// `void ExitProcess(UINT code)` — `__stdcall`, does not return.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe extern "stdcall-unwind" fn exit_process_detour(code: u32) -> ! {
    let _ = std::panic::catch_unwind(|| flush_at_exit(ExitReason::ExitProcess));
    let orig_addr = ORIG_EXIT_PROCESS.load(Ordering::Acquire);
    if orig_addr == 0 {
        // Unreachable while the slot points here; still, never call 0.
        unsafe { windows_sys::Win32::System::Threading::ExitProcess(code) }
    }
    let original: unsafe extern "stdcall-unwind" fn(u32) -> ! =
        unsafe { std::mem::transmute(orig_addr) };
    unsafe { original(code) }
}

/// `void exit(int code)` from MSVCR80 — `__cdecl`, does not return. This is
/// the normal quit path: `___tmainCRTStartup` calls it after WinMain returns.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe extern "C-unwind" fn crt_exit_detour(code: i32) -> ! {
    let _ = std::panic::catch_unwind(|| flush_at_exit(ExitReason::CrtExit));
    let orig_addr = ORIG_CRT_EXIT.load(Ordering::Acquire);
    if orig_addr == 0 {
        std::process::exit(code)
    }
    let original: unsafe extern "C-unwind" fn(i32) -> ! = unsafe { std::mem::transmute(orig_addr) };
    unsafe { original(code) }
}

#[cfg(all(test, target_os = "windows", target_arch = "x86"))]
mod tests {
    use super::*;
    use crate::shutdown;
    use std::time::Duration;

    /// The `ExitProcess` detour raises the stop flag, waits for the uploader
    /// to report done, then chains to the original. A panicking stand-in for
    /// the original proves the chain is reached (the real one never returns).
    /// Removing the flush call fails the stop-flag and reason assertions.
    #[test]
    fn exit_process_detour_flushes_then_chains_to_the_original() {
        unsafe extern "stdcall-unwind" fn original(code: u32) -> ! {
            panic!("original ExitProcess({code})");
        }
        ORIG_EXIT_PROCESS.store(original as *const () as usize, Ordering::Release);

        let global = shutdown::global();
        assert!(!global.stop_requested());
        global.arm();
        let uploader = std::thread::spawn(|| {
            while !shutdown::global().stop_requested() {
                std::thread::sleep(Duration::from_millis(5));
            }
            shutdown::global().mark_flushed();
        });

        let caught = std::panic::catch_unwind(|| unsafe { exit_process_detour(7) });
        let message = *caught
            .expect_err("the original was reached")
            .downcast::<String>()
            .unwrap();
        uploader.join().unwrap();

        assert_eq!(message, "original ExitProcess(7)");
        assert!(global.stop_requested(), "the detour raised the stop flag");
        assert_eq!(global.reason(), Some(ExitReason::ExitProcess));
    }
}
