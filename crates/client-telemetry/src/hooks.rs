//! Tier-1 hook installation — the actual observation surface.
//!
//! Called from `boot::bootstrap_phase2` after the queue +
//! uploader are running. Each hook below registers itself with
//! SGW.exe and emits events into the shared queue when fired.
//!
//! # What gets hooked
//!
//! Anchors per `docs/reverse-engineering/findings/client-instrumentation-hookpoints.md`
//! (the Tier 1 table). Two kinds:
//!
//! - **CME EventSignal subscribe** (no code patching) —
//!   `Event_NetIn_onClientReady`, `Event_NetIn_onClientMapLoad`.
//! - **Inline JMP hook** (MinHook) — `FEngineLoop::Tick` (sampled),
//!   `UWorld::UpdateLevelStreaming`, `FArchiveAsync::Read*`,
//!   `LoadPackage`, `Mercury::Nub::handleMessage`.
//!
//! # Address stability
//!
//! SGW.exe has ASLR disabled (AtreaFixASLR.bat clears
//! `IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE` on-disk), so all
//! addresses below are stable across launches and identical on
//! every machine running the same SGW.exe build.
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

mod cme_hooks;
mod inline_hooks;
mod sampling;

pub use sampling::SamplingCounter;

/// Install every Phase-2 tier-1 hook. Called once from
/// `boot::bootstrap_phase2` after the queue + uploader are up.
///
/// `producer` is cloned into each hook's per-installation context
/// so the hook's emit path doesn't have to walk through `OnceLock`.
/// The clones are cheap (Arc-backed inside crossbeam-channel).
pub fn install_all(producer: Producer) {
    // Each install function is responsible for its own success/
    // failure event. Order doesn't matter — they're independent.
    cme_hooks::install(producer.clone());
    inline_hooks::install(producer);
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
    let mut b = ClientNativeEvent::builder(target, "info");
    for (k, v) in fields {
        b = b.field(k, v);
    }
    producer.try_emit(b);
}

/// Convenience: emit a one-shot warn event for install failures.
#[cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]
pub(crate) fn emit_warn(
    producer: &Producer,
    target: &str,
    fields: impl IntoIterator<Item = (&'static str, serde_json::Value)>,
) {
    let mut b = ClientNativeEvent::builder(target, "warn");
    for (k, v) in fields {
        b = b.field(k, v);
    }
    producer.try_emit(b);
}
