//! Inline (JMP-trampoline) hooks via MinHook.
//!
//! Patches the first ~5 bytes of a target function with a JMP to
//! our detour; MinHook builds a trampoline that runs the original
//! prologue + jumps back. Our detour emits an event and calls the
//! trampoline to invoke the original function transparently.
//!
//! # Module layout
//!
//! One submodule per hook family; each owns its target addresses,
//! trampoline statics, samplers, install fns, and detours:
//!
//! - [`mercury_dispatch`] — inbound Mercury packet dispatch + the
//!   entity-method silent-drop oracle (network thread).
//! - [`engine_frame`] — per-frame drivers: `FEngineLoop::Tick` and
//!   `FFullScreenMovieBink::Tick`.
//! - [`engine_loading`] — asset/package loading + level streaming:
//!   `FArchiveAsync::Serialize`, `UObject::StaticLoadObject`,
//!   `UWorld::UpdateLevelStreamingInner`, cooked-data PAK load.
//! - [`state_flags`] — `GameBeing::onStateFieldUpdate` (all 9
//!   BSF_* flags via one dispatcher hook).
//! - [`anim_notify`] — `USGWAnimNotify_Event::Notify` A + B.
//! - [`console_command`] — `APlayerController::execConsoleCommand`.
//!
//! # What's installed
//!
//! Phase 2 (5 hooks) + Phase 3/5 (6 additional inline hooks) + the
//! client-dispatch drop oracle (PR #620) = 12 hooks. All Phase 3-5
//! anchors resolved + signature-confirmed in Ghidra on 2026-06-04
//! (see `client-instrumentation-entry-points.md`).
//!
//! | Function | Address | Target | Sampling |
//! |---|---|---|---|
//! | `Mercury::Nub::handleMessage` | `0x01b18be0` | `client.mercury.dispatch` | none (network-bounded) |
//! | `FEngineLoop::Tick` | `0x00416ec0` | `client.engine.tick` | 1/100 (~0.3-1.2 Hz at 30-120 fps) |
//! | `FArchiveAsync::Serialize` (vtbl slot 1) | `0x004c7ae0` | `client.engine.async_archive_serialize` | 1/1000 (hot during loads) |
//! | `UWorld::UpdateLevelStreamingInner` | `0x0054e9c0` | `client.engine.update_level_streaming` | 1/10 (fires per streaming level per frame) |
//! | `UObject::StaticLoadObject` | `0x004a8e10` | `client.engine.static_load_object` (with `package_name` field) | 1/10 (bursts during cold loads) |
//! | `GameBeing::onStateFieldUpdate` | `0x00e01c90` | `client.state.field_update` | 1/1 (one hook covers all 9 BSF_* flags via dispatcher) |
//! | `USGWAnimNotify_Event::Notify` (A) | `0x00e974b0` | `client.anim.notify` (`variant=a`) | 1/100 (shared with B) |
//! | `USGWAnimNotify_Event::Notify` (B) | `0x00e97070` | `client.anim.notify` (`variant=b`) | 1/100 (shared with A) |
//! | Cooked-data PAK load | `0x00420074` | `client.engine.pak_load` (with `category` field) | 1/1 (21 PAKs total) |
//! | `APlayerController::execConsoleCommand` | `0x00539850` | `client.input.console_command` | 1/1 |
//! | `FFullScreenMovieBink::Tick` (vtbl slot 1) | `0x0050bbc0` | `client.engine.bink_tick` (with `delta_seconds` field) | 1/30 (~1/sec during cinematics) |
//! | `EntityDescription_GetExposedClientMethodByIndex` (silent-drop oracle) | `0x01590f30` | `client.dispatch.method_dropped` (with `method_index` field) | 1/1 unsampled — drops are the finding |
//!
//! # Why MinHook
//!
//! `retour 0.3` requires nightly Rust (`feature(unboxed_closures)`).
//! `minhook-sys` is a thin FFI binding to the MinHook C library
//! that builds on stable. The API is `MH_CreateHook(target,
//! detour, &mut trampoline)` + `MH_EnableHook(target)`.

#![allow(clippy::missing_safety_doc)] // FFI bindings — safety doc in fn-level

mod anim_notify;
mod console_command;
mod engine_frame;
mod engine_loading;
mod mercury_dispatch;
mod state_flags;

use crate::queue::Producer;

// Re-exported at the pre-split path — `iat_hooks.rs` borrows this
// bounded reader for its own foreign-string captures.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(super) use engine_loading::read_utf16_bounded;

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::ffi::c_void;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::OnceLock;

/// Install all inline hooks. Best-effort: a MinHook init failure
/// or a single CreateHook failure logs a warn event and the rest
/// of the hooks still attempt to install.
pub fn install(_producer: Producer) {
    // Active only on the real DLL target. The detour's `thiscall`
    // ABI is x86-only and MinHook's symbols only link on Windows.
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    unsafe {
        install_inner(_producer);
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_inner(producer: Producer) {
    // MinHook needs a one-time init per process.
    let init_status = minhook_sys::MH_Initialize();
    if init_status != minhook_sys::MH_OK && init_status != minhook_sys::MH_ERROR_ALREADY_INITIALIZED
    {
        super::emit_warn(
            &producer,
            "client.hooks.inline.init_failed",
            [("status", serde_json::json!(init_status as i32))],
        );
        return;
    }

    mercury_dispatch::install_handle_message(&producer);
    engine_frame::install_engine_tick(&producer);
    engine_loading::install_archive_async_serialize(&producer);
    engine_loading::install_update_level_streaming_inner(&producer);
    engine_loading::install_static_load_object(&producer);
    state_flags::install_state_field_update(&producer);
    anim_notify::install_anim_notify_a(&producer);
    anim_notify::install_anim_notify_b(&producer);
    engine_loading::install_cooked_data_load(&producer);
    console_command::install_console_command(&producer);
    engine_frame::install_bink_tick(&producer);
    mercury_dispatch::install_entity_method_not_found(&producer);

    super::emit_info(
        &producer,
        "client.hooks.inline.install_complete",
        [("hook_count", serde_json::json!(12))],
    );
}

/// Shared CreateHook + EnableHook plumbing — same for every
/// inline target. Reports `installed` on success and
/// `create_failed` / `enable_failed` on failure with the hook name
/// for SigNoz filtering.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
unsafe fn install_one(
    producer: &Producer,
    hook_name: &'static str,
    address: usize,
    detour: *mut c_void,
    trampoline_slot: &OnceLock<usize>,
) {
    let target = address as *mut c_void;
    let mut trampoline: *mut c_void = std::ptr::null_mut();

    let create_status = minhook_sys::MH_CreateHook(target, detour, &mut trampoline);
    if create_status != minhook_sys::MH_OK {
        super::emit_warn(
            producer,
            "client.hooks.inline.create_failed",
            [
                ("hook", serde_json::Value::String(hook_name.into())),
                ("status", serde_json::json!(create_status as i32)),
                (
                    "address",
                    serde_json::Value::String(format!("0x{address:08x}")),
                ),
            ],
        );
        return;
    }
    let _ = trampoline_slot.set(trampoline as usize);

    let enable_status = minhook_sys::MH_EnableHook(target);
    if enable_status != minhook_sys::MH_OK {
        super::emit_warn(
            producer,
            "client.hooks.inline.enable_failed",
            [
                ("hook", serde_json::Value::String(hook_name.into())),
                ("status", serde_json::json!(enable_status as i32)),
            ],
        );
        return;
    }

    super::emit_info(
        producer,
        "client.hooks.inline.installed",
        [
            ("hook", serde_json::Value::String(hook_name.into())),
            (
                "address",
                serde_json::Value::String(format!("0x{address:08x}")),
            ),
        ],
    );
}

#[cfg(test)]
mod tests {
    /// Pin the resolved addresses against the documented anchors.
    /// If a future RE pass moves any of them, this test trips and
    /// reminds us to update the docs + commit comment together.
    ///
    /// Aggregated here (rather than per-submodule) so this one test
    /// is the single manifest of every inline-hooked address,
    /// mirroring the module-doc table above.
    #[test]
    fn anchor_addresses_match_re_findings() {
        // Ghidra resolution 2026-06-04 (PR #504 follow-up):
        #[cfg(all(target_os = "windows", target_arch = "x86"))]
        {
            assert_eq!(super::mercury_dispatch::ADDR_HANDLE_MESSAGE, 0x01b18be0);
            assert_eq!(super::engine_frame::ADDR_FENGINE_LOOP_TICK, 0x00416ec0);
            assert_eq!(
                super::engine_loading::ADDR_ARCHIVE_ASYNC_SERIALIZE,
                0x004c7ae0
            );
            assert_eq!(
                super::engine_loading::ADDR_UPDATE_LEVEL_STREAMING_INNER,
                0x0054e9c0
            );
            assert_eq!(super::engine_loading::ADDR_STATIC_LOAD_OBJECT, 0x004a8e10);
            // Phase 3 + 5 inline targets (2026-06-04 manifest):
            assert_eq!(super::state_flags::ADDR_STATE_FIELD_UPDATE, 0x00e01c90);
            assert_eq!(super::anim_notify::ADDR_ANIM_NOTIFY_A, 0x00e974b0);
            assert_eq!(super::anim_notify::ADDR_ANIM_NOTIFY_B, 0x00e97070);
            assert_eq!(super::engine_loading::ADDR_COOKED_DATA_LOAD, 0x00420074);
            assert_eq!(super::console_command::ADDR_CONSOLE_COMMAND, 0x00539850);
            assert_eq!(super::engine_frame::ADDR_BINK_TICK, 0x0050bbc0);
            // Sole callee of the silent-drop path in
            // Client_NetIn_EntityMethodDispatch (0x00c6f8f0), called
            // from 0x00c6fa95. Exactly one xref — if that ever stops
            // being true, this hook reports false drops.
            assert_eq!(
                super::mercury_dispatch::ADDR_ENTITY_METHOD_NOT_FOUND,
                0x01590f30
            );
        }
    }
}
