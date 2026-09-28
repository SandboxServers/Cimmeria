//! `APlayerController::execConsoleCommand` hook — fires on every
//! script-invoked console command.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::OnceLock;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use crate::queue::Producer;

/// `APlayerController::execConsoleCommand` — UnrealScript exec
/// wrapper, FuncMap-bound at 0x01db2460. Fires on every script-
/// invoked console command.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) const ADDR_CONSOLE_COMMAND: usize = 0x00539850;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static CONSOLE_COMMAND_TRAMPOLINE: OnceLock<usize> = OnceLock::new();

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) unsafe fn install_console_command(producer: &Producer) {
    super::install_one(
        producer,
        "console_command",
        ADDR_CONSOLE_COMMAND,
        console_command_detour as *mut c_void,
        &CONSOLE_COMMAND_TRAMPOLINE,
    );
}

/// Detour for `APlayerController::execConsoleCommand`.
///
/// Signature: `thiscall fn(*mut APlayerController, FFrame& stack, void* result)`,
/// the standard UE3 `exec` thunk shape. The function returns with
/// `ret 8`, so it takes two stack arguments; a one-argument detour
/// would leave four bytes on the caller's stack on every console
/// command. The command string is in the FFrame; decoding it would
/// require parsing FFrame bytecode, so the event is bare for now.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall-unwind" fn console_command_detour(
    this: *mut c_void,
    frame: *mut c_void,
    result: *mut c_void,
) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(crate::events::ClientNativeEvent::builder(
                "client.input.console_command",
                "info",
            ));
        }
    });

    if let Some(t) = CONSOLE_COMMAND_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall-unwind" fn(*mut c_void, *mut c_void, *mut c_void) =
            unsafe { std::mem::transmute(*t) };
        original(this, frame, result);
    }
}

#[cfg(test)]
mod tests {
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    use super::*;

    /// The detour hands both stack arguments of the exec thunk to the
    /// original (it pops 8 bytes; a one-argument detour popped 4), and an
    /// exception from the original unwinds through it (#915).
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn forwards_frame_and_result_and_lets_exceptions_through() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static SEEN: [AtomicUsize; 3] = [const { AtomicUsize::new(0) }; 3];
        unsafe extern "thiscall-unwind" fn original(
            this: *mut c_void,
            frame: *mut c_void,
            result: *mut c_void,
        ) {
            for (slot, v) in SEEN.iter().zip([this, frame, result]) {
                slot.store(v as usize, Ordering::SeqCst);
            }
            panic!("script error");
        }
        CONSOLE_COMMAND_TRAMPOLINE
            .set(original as *const () as usize)
            .expect("only this test sets the trampoline");
        let caught = std::panic::catch_unwind(|| unsafe {
            console_command_detour(
                0x1111 as *mut c_void,
                0x2222 as *mut c_void,
                0x3333 as *mut c_void,
            )
        });
        assert!(caught.is_err());
        let seen: Vec<usize> = SEEN.iter().map(|s| s.load(Ordering::SeqCst)).collect();
        assert_eq!(seen, [0x1111, 0x2222, 0x3333]);
    }
}
