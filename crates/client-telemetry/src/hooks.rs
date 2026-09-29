//! Hook installation — the actual observation surface.
//!
//! Called from `boot::bootstrap_phase2` after the queue +
//! uploader are running. Each hook below registers itself with
//! SGW.exe and emits events into the shared queue when fired.
//!
//! # What gets hooked
//!
//! Anchors per the `client-instrumentation-hookpoints.md` and
//! `client-instrumentation-entry-points.md` docs. Three techniques:
//!
//! - **Inline JMP** (MinHook) — `inline_hooks/`. 22 hooks (the 11 below
//!   plus the entity-lifecycle, inbound-message, outgoing-RPC and Mercury
//!   receive-path hooks documented in `inline_hooks/mod.rs`):
//!   FEngineLoop::Tick, FArchiveAsync::Serialize,
//!   UWorld::UpdateLevelStreamingInner, UObject::StaticLoadObject,
//!   GameBeing::onStateFieldUpdate, USGWAnimNotify::Notify A+B,
//!   APlayerController::execConsoleCommand,
//!   FFullScreenMovieBink::Tick, the entity-method silent-drop
//!   oracle (`client.dispatch.method_dropped`), and the CME
//!   event-registry lookup (`client.cme.event`), which names every
//!   CME event the client creates, inbound server methods included.
//!
//! There is no CME subscriber install any more. The old one called
//! `0x00a5c0f0` with a C string where it takes a `std::string`, and
//! `0x00a5c150`, which is `count(name)` on the same map, as
//! "subscribe": it could only ever fail (2026-09-28). The event-registry
//! hook replaces the two events it was meant to produce.
//! - **IAT swap** — `iat_hooks.rs`. 7 hooks: lua_pcall/call/newstate
//!   + CreateThread + LoadLibraryW/A + GetForegroundWindow.
//! - **Vtable swap** — `vtable_hooks.rs`. 3 hooks: CEGUI::DefaultLogger::
//!   logEvent + AActor::Tick + USequence::UpdateOp.
//!
//! # Address stability
//!
//! SGW.exe has ASLR disabled (AtreaFixASLR.bat clears
//! `IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE` on-disk), so all
//! addresses below are stable across launches and identical on
//! every machine running the same SGW.exe build.
//!
//! # The fingerprint gate
//!
//! `boot` calls [`install_all`] only after [`crate::fingerprint`] has
//! matched every hooked function and vtable slot against the QA build,
//! under the install lock shared with the client-patches DLL. On any
//! mismatch nothing here runs.
//!
//! # Failure shape
//!
//! Every install attempt is best-effort: a failed hook (address
//! mismatch from a stale binary, MinHook init failure, missing
//! signal name) logs an error event into the queue and the hook
//! becomes a no-op. The DLL never returns failure to SGW.exe — a
//! broken hook is invisible to the game.

use crate::events::ClientNativeEvent;
use crate::queue::Producer;

// Pure helpers run only from the i686 CEGUI logger detour.
#[cfg_attr(not(target_arch = "x86"), allow(dead_code))]
mod cegui_log;
// The one-time dump of every registered CME event type (i686 walker).
#[cfg_attr(not(target_arch = "x86"), allow(dead_code))]
mod cme_catalog;
// One place the entity, net and Lua detours emit an event (i686 only).
#[cfg_attr(not(target_arch = "x86"), allow(dead_code))]
mod emit;
// Per-entity state, map reader and field builders behind the
// `client.entity.*` events; driven only by the i686 detours.
#[cfg_attr(not(target_arch = "x86"), allow(dead_code))]
pub(crate) mod entity_trace;
mod iat_hooks;
mod inline_hooks;
// Mercury receive-path readers and classifiers behind the
// `client.mercury.packet_in|fragment|bundle` events; driven only by the
// i686 detours.
#[cfg_attr(not(target_arch = "x86"), allow(dead_code))]
pub(crate) mod mercury_recv;
// Used only by the i686 CME event-factory detour.
#[cfg_attr(not(target_arch = "x86"), allow(dead_code))]
mod name_throttle;
// `pub(crate)` so the lab bridge's dynamic-hook installer
// (`bridge::dynamic_hooks::native`, #686) can reuse the inline-hook
// trampoline primitive without duplicating the protect/patch/flush dance.
pub(crate) mod primitives;
mod sampling;
// The engine layer's log sinks and OS-level seams (BigWorld messages, UE3
// `GLog`, log4cxx, debug strings, exceptions). Own module tree, own
// fingerprint sites.
pub mod seams;
pub mod sinks;
mod vtable_hooks;

pub use sampling::SamplingCounter;

// Reusable low-level hooking mechanics, shared by the IAT/vtable
// telemetry hooks here and consumed by the future client-patch crates.
#[cfg(target_arch = "x86")]
pub use primitives::{install_inline_hook, InlineHook, JMP_REL32_LEN};
pub use primitives::{replace_iat_slot, swap_vtable_slot, HookError};

/// Install every Phase-2 tier-1 hook. Called once from
/// `boot::bootstrap_phase2` after the queue + uploader are up.
///
/// `producer` is cloned into each hook's per-installation context
/// so the hook's emit path doesn't have to walk through `OnceLock`.
/// The clones are cheap (Arc-backed inside crossbeam-channel).
pub fn install_all(producer: Producer) {
    // Each install function is responsible for its own success/
    // failure event. Order doesn't matter — they're independent.
    inline_hooks::install(producer.clone());
    iat_hooks::install(producer.clone());
    vtable_hooks::install(producer.clone());
    sinks::install(producer.clone());
    seams::install(producer.clone());
    sinks::emit_capabilities(&producer);
}

/// Convenience: emit a one-shot info event with this target +
/// fields. Used by hook installers to report success/failure
/// without each one repeating the builder ceremony.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
pub(crate) fn emit_info(
    producer: &Producer,
    target: &str,
    fields: impl IntoIterator<Item = (&'static str, serde_json::Value)>,
) {
    emit(producer, "info", target, fields);
}

/// Convenience: emit a one-shot warn event for install failures.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
pub(crate) fn emit_warn(
    producer: &Producer,
    target: &str,
    fields: impl IntoIterator<Item = (&'static str, serde_json::Value)>,
) {
    emit(producer, "warn", target, fields);
}

/// Queue the event, and write the same thing to the local log: install
/// events are rare, and the log is what is left when the upload fails.
fn emit(
    producer: &Producer,
    level: &str,
    target: &str,
    fields: impl IntoIterator<Item = (&'static str, serde_json::Value)>,
) {
    let mut b = ClientNativeEvent::builder(target, level);
    let mut text = format!("{level} {target}");
    for (k, v) in fields {
        text.push_str(&format!(" {k}={v}"));
        b = b.field(k, v);
    }
    crate::log::line(text);
    producer.try_emit(b);
}
