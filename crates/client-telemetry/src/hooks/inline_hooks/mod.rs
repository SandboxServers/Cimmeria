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
//! - [`mercury_dispatch`] — the entity-method silent-drop oracle
//!   (network thread).
//! - [`engine_frame`] — per-frame drivers: `FEngineLoop::Tick` and
//!   `FFullScreenMovieBink::Tick`.
//! - [`engine_loading`] — asset/package loading + level streaming:
//!   `FArchiveAsync::Serialize`, `UObject::StaticLoadObject`,
//!   `UWorld::UpdateLevelStreamingInner`.
//! - [`state_flags`] — `GameBeing::onStateFieldUpdate` (all 9
//!   BSF_* flags via one dispatcher hook).
//! - [`anim_notify`] — `USGWAnimNotify_Event::Notify` A + B.
//! - [`console_command`] — `APlayerController::execConsoleCommand`.
//! - [`cme_event_factory`] — the CME event-registry lookup: every event
//!   the client creates by name, including each inbound entity method it
//!   routed (2026-09-28). An `Event_NetIn_*` created while the network
//!   thread dispatches for an entity carries that entity's id.
//! - [`entity_lifecycle`] — `enterAoI`, `createEntity`, `enterWorld`,
//!   `leaveAoI`, destroy and the appearance request, each with the
//!   manager's before/after view of the entity (`client.entity.*`).
//! - [`entity_messages`] — inbound entity methods and properties, the
//!   deferred-message queue and its replay (`client.mercury.entity_*`),
//!   which also set the dispatch context the CME and drop events read.
//! - [`net_out`] — the outgoing entity-method router (`client.net.out`).
//!
//! # What's installed
//!
//! 27 hooks. Every address below was re-checked against the QA
//! `SGW.exe` on 2026-09-27 (function entry, `ret N` against the detour's
//! argument count) and is covered by the [fingerprint
//! gate](crate::fingerprint), which installs none of them on a build
//! whose bytes differ. Two earlier anchors were removed then:
//! `Mercury::Nub::handleMessage` (`0x01b18be0` is a log string) and the
//! cooked-data PAK load (`0x00420074` is mid-function). #989 tracks
//! re-adding them.
//!
//! Every detour, and the trampoline type it calls the original
//! through, uses the `-unwind` ABI (`thiscall-unwind`, `C-unwind`).
//! UE3 reports errors by throwing C++ exceptions; with a plain ABI the
//! unwind aborts the process at the detour instead of reaching the
//! engine's handler. Rust panics inside a detour stay inside
//! `catch_unwind`, so none can unwind into the game.
//!
//! | Function | Address | Target | Sampling |
//! |---|---|---|---|
//! | `FEngineLoop::Tick` | `0x00416ec0` | `client.engine.tick` | 1/100 (~0.3-1.2 Hz at 30-120 fps) |
//! | `FArchiveAsync::Serialize` (vtbl slot 1) | `0x004c7ae0` | `client.engine.async_archive_serialize` | 1/1000 (hot during loads) |
//! | `UWorld::UpdateLevelStreamingInner` | `0x0054e9c0` | `client.engine.update_level_streaming` | 1/10 (fires per streaming level per frame) |
//! | `UObject::StaticLoadObject` | `0x004a8e10` | `client.engine.static_load_object` (with `package_name` field) | 1/10 (bursts during cold loads) |
//! | `GameBeing::onStateFieldUpdate` (2 stack args) | `0x00e01c90` | `client.state.field_update` | 1/1 (one hook covers all 9 BSF_* flags via dispatcher) |
//! | `USGWAnimNotify_Event::Notify` (A) | `0x00e974b0` | `client.anim.notify` (`variant=a`) | 1/100 (shared with B) |
//! | `USGWAnimNotify_Event::Notify` (B) | `0x00e97070` | `client.anim.notify` (`variant=b`) | 1/100 (shared with A) |
//! | `APlayerController::execConsoleCommand` (`this, FFrame&, Result*`) | `0x00539850` | `client.input.console_command` | 1/1 |
//! | `FFullScreenMovieBink::Tick` (vtbl slot 1) | `0x0050bbc0` | `client.engine.bink_tick` (with `delta_seconds` field) | 1/30 (~1/sec during cinematics) |
//! | `EntityDescription_GetExposedClientMethodByIndex` (silent-drop oracle) | `0x01590f30` | `client.dispatch.method_dropped` (with `method_index` field) | 1/1 unsampled — drops are the finding |
//! | CME event-registry lookup (`thiscall(registry, const std::string&)`) | `0x00a5c0f0` | `client.cme.event` (`event`, `kind`, `entity_id` for `net_in`; `info` for `net_in`, else `debug`) | per-name token bucket: burst 8, 4/s, `suppressed` count on the next emit; `net_in` inside an entity dispatch: per (event, entity) |
//! | `EntityManager::onEntityEnter` (`enterAoI`) | `0x00dd24f0` | `client.entity.enter` | per (event, entity) bucket |
//! | `EntityManager::onEntityCreate` | `0x00dd2270` | `client.entity.create` | per (event, entity) bucket |
//! | `EntityManager::enterWorld` | `0x00dd1d00` | `client.entity.entered_world` | per (event, entity) bucket |
//! | `EntityManager::onEntityLeave` (`leaveAoI`) | `0x00dd2800` | `client.entity.leave` | per (event, entity) bucket |
//! | entity destroy | `0x00dd1120` | `client.entity.destroyed` | per (event, entity) bucket |
//! | `GameEntity` appearance request | `0x00e69150` | `client.entity.appearance_request` | per (event, entity) bucket |
//! | appearance job scheduler | `0x00e998e0` | (marks the request as scheduled; no event) | - |
//! | `EntityManager::onEntityMethod` | `0x00dd2b80` | `client.mercury.entity_method` (`debug` delivered, `info` queued) | per (event, entity) bucket |
//! | `EntityManager::onEntityProperty` | `0x00dd29d0` | `client.mercury.entity_property` | per (event, entity) bucket |
//! | queued-message replay | `0x00dd1e40` | `client.entity.queue_replay` | per (event, entity) bucket |
//! | `RouteOutgoingEntityRpc` (`stdcall`, 4 args) | `0x00c6fc40` | `client.net.out` | per (method, entity) bucket |
//! | `Nub::processFilteredPacket` (`this, addr, packet`, `ret 8`) | `0x01580840` | `client.mercury.packet_in` (+ `client.mercury.error`) | ordinary traffic: bucket; fragments, buffered, in-flight and every non-happy packet: unthrottled |
//! | `UnAckedHandler::queueAckForPacket` (`ret 0x10`) | `0x0158cba0` | (records the window disposition for the packet event; no event) | - |
//! | `Nub::processPacket` (`this, addr, packet, channel`, `ret 0xc`) | `0x0157fd20` | `client.mercury.fragment` (+ `client.mercury.error`) | fragments only, unthrottled |
//! | `Nub::processOrderedPacket` (`this, message`, `ret 4`, game thread) | `0x0157c820` | `client.mercury.bundle` `phase=start|end` (+ `client.mercury.error`) | assembled and non-happy bundles unthrottled; clean single-packet bundles: bucket |
//! | `Bundle::iterator::unpack` (`this, element`, `ret 4`, game thread) | `0x01579830` | (feeds the bundle summary; no event) | - |
//!
//! # Why MinHook
//!
//! `retour 0.3` requires nightly Rust (`feature(unboxed_closures)`).
//! `minhook-sys` is a thin FFI binding to the MinHook C library
//! that builds on stable. The API is `MH_CreateHook(target,
//! detour, &mut trampoline)` + `MH_EnableHook(target)`.

#![allow(clippy::missing_safety_doc)] // FFI bindings — safety doc in fn-level

mod anim_notify;
// Its pure helpers (kind/level/throttle) run only from the i686 detour.
#[cfg_attr(not(target_arch = "x86"), allow(dead_code))]
mod cme_event_factory;
mod console_command;
mod engine_frame;
mod engine_loading;
// The event family of a class name, shared with the catalog dump.
pub(crate) use cme_event_factory::kind_of as event_kind;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod entity_lifecycle;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod entity_messages;
mod mercury_dispatch;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod mercury_recv;
#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod net_out;
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

    engine_frame::install_engine_tick(&producer);
    engine_loading::install_archive_async_serialize(&producer);
    engine_loading::install_update_level_streaming_inner(&producer);
    engine_loading::install_static_load_object(&producer);
    state_flags::install_state_field_update(&producer);
    anim_notify::install_anim_notify_a(&producer);
    anim_notify::install_anim_notify_b(&producer);
    console_command::install_console_command(&producer);
    engine_frame::install_bink_tick(&producer);
    mercury_dispatch::install_entity_method_not_found(&producer);
    cme_event_factory::install_cme_event_factory(&producer);
    entity_lifecycle::install_all(&producer);
    entity_messages::install_all(&producer);
    net_out::install_all(&producer);
    mercury_recv::install_all(&producer);

    super::emit_info(
        &producer,
        "client.hooks.inline.install_complete",
        [("hook_count", serde_json::json!(27))],
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
            // CME event-registry lookup: `thiscall(registry, const
            // std::string&) -> event*`, `ret 4` (2026-09-28).
            assert_eq!(super::cme_event_factory::ADDR_CME_EVENT_FACTORY, 0x00a5c0f0);
            // Entity lifecycle, message and outgoing-RPC anchors
            // (findings/client-entity-lifecycle.md, 2026-09-28).
            assert_eq!(super::entity_lifecycle::ADDR_ENTER_AOI, 0x00dd24f0);
            assert_eq!(super::entity_lifecycle::ADDR_CREATE_ENTITY, 0x00dd2270);
            assert_eq!(super::entity_lifecycle::ADDR_ENTER_WORLD, 0x00dd1d00);
            assert_eq!(super::entity_lifecycle::ADDR_LEAVE_AOI, 0x00dd2800);
            assert_eq!(super::entity_lifecycle::ADDR_DESTROY_ENTITY, 0x00dd1120);
            assert_eq!(super::entity_lifecycle::ADDR_APPEARANCE_REQUEST, 0x00e69150);
            assert_eq!(
                super::entity_lifecycle::ADDR_APPEARANCE_SCHEDULE,
                0x00e998e0
            );
            assert_eq!(super::entity_messages::ADDR_ENTITY_METHOD, 0x00dd2b80);
            assert_eq!(super::entity_messages::ADDR_ENTITY_PROPERTY, 0x00dd29d0);
            assert_eq!(super::entity_messages::ADDR_QUEUE_REPLAY, 0x00dd1e40);
            assert_eq!(super::net_out::ADDR_ROUTE_OUTGOING_RPC, 0x00c6fc40);
            // Mercury receive path (findings/client-mercury-receive-path.md).
            assert_eq!(
                super::mercury_recv::ADDR_PROCESS_FILTERED_PACKET,
                0x01580840
            );
            assert_eq!(super::mercury_recv::ADDR_QUEUE_ACK, 0x0158cba0);
            assert_eq!(super::mercury_recv::ADDR_PROCESS_PACKET, 0x0157fd20);
            assert_eq!(super::mercury_recv::ADDR_PROCESS_ORDERED_PACKET, 0x0157c820);
            assert_eq!(super::mercury_recv::ADDR_BUNDLE_UNPACK, 0x01579830);
        }
    }
    /// Every inline-hooked address is a fingerprinted site, so a build
    /// whose bytes differ there installs nothing.
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    #[test]
    fn every_hooked_address_is_fingerprinted() {
        let hooked = [
            super::mercury_dispatch::ADDR_ENTITY_METHOD_NOT_FOUND,
            super::engine_frame::ADDR_FENGINE_LOOP_TICK,
            super::engine_frame::ADDR_BINK_TICK,
            super::engine_loading::ADDR_ARCHIVE_ASYNC_SERIALIZE,
            super::engine_loading::ADDR_UPDATE_LEVEL_STREAMING_INNER,
            super::engine_loading::ADDR_STATIC_LOAD_OBJECT,
            super::state_flags::ADDR_STATE_FIELD_UPDATE,
            super::anim_notify::ADDR_ANIM_NOTIFY_A,
            super::anim_notify::ADDR_ANIM_NOTIFY_B,
            super::console_command::ADDR_CONSOLE_COMMAND,
            super::cme_event_factory::ADDR_CME_EVENT_FACTORY,
            super::entity_lifecycle::ADDR_ENTER_AOI,
            super::entity_lifecycle::ADDR_CREATE_ENTITY,
            super::entity_lifecycle::ADDR_ENTER_WORLD,
            super::entity_lifecycle::ADDR_LEAVE_AOI,
            super::entity_lifecycle::ADDR_DESTROY_ENTITY,
            super::entity_lifecycle::ADDR_APPEARANCE_REQUEST,
            super::entity_lifecycle::ADDR_APPEARANCE_SCHEDULE,
            super::entity_messages::ADDR_ENTITY_METHOD,
            super::entity_messages::ADDR_ENTITY_PROPERTY,
            super::entity_messages::ADDR_QUEUE_REPLAY,
            super::net_out::ADDR_ROUTE_OUTGOING_RPC,
            super::mercury_recv::ADDR_PROCESS_FILTERED_PACKET,
            super::mercury_recv::ADDR_QUEUE_ACK,
            super::mercury_recv::ADDR_PROCESS_PACKET,
            super::mercury_recv::ADDR_PROCESS_ORDERED_PACKET,
            super::mercury_recv::ADDR_BUNDLE_UNPACK,
        ];
        for addr in hooked {
            assert!(
                crate::fingerprint::CODE_SITES
                    .iter()
                    .any(|s| s.address == addr),
                "0x{addr:08x} is hooked but not fingerprinted"
            );
        }
    }
}
