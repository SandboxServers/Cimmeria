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
/// Signature: `extern "thiscall" fn(*mut APlayerController, int frame_ptr)`.
/// The UnrealScript exec wrapper — actual command string is in the
/// FFrame at param_1. Decoding it here would require parsing FFrame
/// bytecode; skip for now and emit the bare event.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
#[allow(improper_ctypes_definitions)]
unsafe extern "thiscall" fn console_command_detour(this: *mut c_void, frame: i32) {
    let _ = std::panic::catch_unwind(|| {
        if let Some(p) = crate::boot::producer() {
            p.try_emit(crate::events::ClientNativeEvent::builder(
                "client.input.console_command",
                "info",
            ));
        }
    });

    if let Some(t) = CONSOLE_COMMAND_TRAMPOLINE.get() {
        let original: unsafe extern "thiscall" fn(*mut c_void, i32) =
            unsafe { std::mem::transmute(*t) };
        original(this, frame);
    }
}
