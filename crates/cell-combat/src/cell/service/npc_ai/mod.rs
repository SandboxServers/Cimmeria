//! NPC AI tick — fight (threat-target attacks + leashing), and leash recovery.
//!
//! # Cadence
//!
//! Two passes share the AI surface, both driven from
//! `cimmeria_services::cell::service::message_loop`:
//!
//! - **[`npc_ai_tick`]** — natural cadence, every 20th AoI tick (~2s
//!   at the 100ms AoI rate). Drives Idle-auto-aggro, Leashing, and
//!   the baseline Fighting pass for every NPC.
//! - **[`npc_ai_retry_sweep`]** — fast retry, every AoI tick (~100ms).
//!   Picks up Fighting NPCs whose `ai_retry_at` deadline has passed
//!   after a `handle_use_ability` launch failure, so the AI can
//!   re-attempt within 500–600ms instead of waiting the full 2s
//!   natural cadence. Iterates `space_mgr.pending_ai_retries`
//!   (`O(pending)`), not the full NPC list.
//!
//! Healthy NPCs never appear in `pending_ai_retries`; the retry
//! sweep is effectively free in that case.
//!
//! # Module layout
//!
//! This module is split along behavior-state seams:
//!
//! - [`dispatch`] — the [`npc_ai_tick`] state-machine entry and the
//!   [`npc_ai_retry_sweep`] fast-retry pass.
//! - [`fight`] — the Fighting handler.
//! - [`fight_target`] — the fight's target selection: prune dead, vanished
//!   and lost targets (draining their combat state) and start the walk home
//!   when nobody is left.
//! - [`idle_aggro`] — the Idle auto-aggro scan that seeds Fighting.
//! - [`aggro_gates`] — that scan's candidate gates (hostility, GM toggle,
//!   vertical band, radius, line of sight) and reject reasons (NA13).
//! - [`assist`] — same-room assist: a fresh engagement pulls hostile
//!   same-faction neighbours onto the same target, without chaining (NA14,
//!   a marked deviation from legacy).
//! - [`ability_select`] — ability bucket choice, range resolution,
//!   and the step-back waypoint geometry.
//! - [`step_back`] — the ranged step-back: comfort range, hysteresis and
//!   cooldown (NA32, D-NA15).
//! - [`patrol`] / [`wander`] / [`investigate`] / [`follow`] — the
//!   movement-state handlers.
//! - [`leash`] — the leash policy (NPC-to-spawn radius with hysteresis),
//!   the walk home with evade, and the reset on arrival (NA12).
//! - [`lifecycle`] — the terminal / quiescent states (despawn,
//!   submit, error).
//! - [`path_failure`] — the one throttled emitter every state above
//!   uses when `find_path` gives it nothing usable.
//! - [`aggro_acquired`] — the `npc_ai.aggro event=acquired` row and
//!   its cause-to-transition-reason mapping.
//! - [`transition`] — `set_ai_state`, the single writer of `ai_state`,
//!   which emits `npc_ai.transition` and `npc_ai_transitions_total`.
//! - [`chase`] — the Fighting handler's chase: the stop distance, the
//!   repath threshold, the hold and give-up at a route that cannot reach the
//!   target, and the off-mesh start and target recovery (NA15).
//! - [`fight_cover`] — the Fighting handler's cover step, including the
//!   `no_cover` reasons and the `cover.selection` sample.
//! - [`path_request`] — every AI `find_path`, logged as `npc_ai.path
//!   event=request` with its typed outcome.
//! - [`detectors`] — NA02's stuck / stale / floating / leash-loop / LoS /
//!   off-mesh rows. Reporting only; they change no decision.

mod ability_select;
mod aggro_acquired;
mod aggro_gates;
mod assist;
mod chase;
#[cfg(test)]
mod detector_tests;
// The NPC AI's state primitives (`detectors`, `transition`, `movement_stop`,
// `leash::policy`) are in `cimmeria-cell-world` (wave C1): every lower cell
// system calls them. Imported at their old paths.
pub use cimmeria_cell_world::cell::service::npc_ai::detectors;
mod dispatch;
mod fight;
// Public (hidden) only for the NA23 blind-slot guard in
// `cimmeria-services`' `service::tests::npc_ai_cover_peek`, which drives
// `blind_in_slot` directly on the seeded Cellblock guards.
#[doc(hidden)]
pub mod fight_cover;
mod fight_target;
mod follow;
#[cfg(test)]
mod ground_endpoint_tests;
mod idle_aggro;
mod investigate;
pub(in crate::cell) mod leash;
mod lifecycle;
use cimmeria_cell_world::cell::service::npc_ai::movement_stop;
mod path_failure;
mod path_request;
mod patrol;
mod step_back;
use cimmeria_cell_world::cell::service::npc_ai::transition;
mod wander;

// Re-export discipline: `cimmeria-services` reaches these as
// `super::npc_ai::X` (the message loop) and `crate::cell::service::npc_ai::X`
// (its tests) through its `npc_ai` module, which re-exports this one, so
// those paths did not change when the AI moved here (wave C2).
pub use dispatch::{npc_ai_retry_sweep, npc_ai_tick};

// Combat reaches these here. NA24: `abilities::death::resolve_death` purges a
// dead player from every NPC's threat list at the moment of death. NA14:
// `combat::generate_threat` fans a fresh engagement out to neighbours and
// logs the acquisition.
pub(in crate::cell) use aggro_acquired::log_aggro_acquired;
pub(in crate::cell) use assist::recruit_assisters;
pub(in crate::cell) use fight_target::purge_dead_player_from_threat;
// Stopping and rerouting an NPC: the only writers of `nav_path` outside the
// movement tick (NA10). The AI-state transition helper is the only way to
// change `ai_state` (the field is private in the entity crate). Combat, the
// content executor, the GM console and the respawn tick reach both here;
// they live in `cimmeria-cell-world`.
pub use movement_stop::{
    replace_nav_path_on, snap_npc_to, stop_movement_on, stop_npc_movement, StopReason,
};
pub use transition::{set_ai_state, set_ai_state_on, world_label, AiTransitionReason};

// ── Test hooks ──────────────────────────────────────────────────────────
//
// For this crate's tests and, behind the `test-support` feature, the tests
// of `cimmeria-services` (docs/architecture/services-crate-split.md §3).
// The production functions behind them keep their narrow visibility: a
// `pub` item in a private module that is re-exported only under the feature
// trips `unreachable_pub` in a normal build, so those get thin wrappers.

/// Test fixtures arrange a starting state without a transition row.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub use transition::force_ai_state;

/// The real AI tick, for the NA13 chain-replay guards
/// (`content::chain_replay_tests`), which run it against spawns loaded from
/// the seed.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub async fn npc_ai_tick_for_test(
    tx: &tokio::sync::mpsc::Sender<crate::cell::messages::CellToBaseMsg>,
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
    events: &dyn crate::cell::content_events::ContentEvents,
) {
    dispatch::npc_ai_tick(tx, space_mgr, events).await;
}

/// The degenerate (co-located NPC + target) branch of
/// `compute_backup_waypoint`, for `tests/npc_ai/ability_range.rs`.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub use ability_select::compute_backup_waypoint_for_test;

/// The ability selector, for the NPC AI selector tests and the Harset H09
/// live-DB guard (`cell/spawner_tests/harset/`), which loads a multi-row
/// ability set out of Postgres, spawns it, and then drives this selector.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub fn choose_npc_ability(
    npc_id: u32,
    space_mgr: &crate::cell::space_manager::SpaceManager,
) -> Option<i32> {
    ability_select::choose_npc_ability(npc_id, space_mgr)
}

/// The *production* selector (the reach-filtered one `fight` calls), for the
/// same Harset guard, so the seed and the melee gate are pinned together.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub fn choose_npc_ability_within_reach(
    npc_id: u32,
    space_mgr: &crate::cell::space_manager::SpaceManager,
    target_dist: f32,
    npc_attack_range: f32,
) -> Option<i32> {
    ability_select::choose_npc_ability_within_reach(
        npc_id,
        space_mgr,
        target_dist,
        npc_attack_range,
    )
}

/// The dispatcher's admit filter, for `tests/npc_ai/zero_health_guard.rs`,
/// which drives it with a synthetic clock to pin its per-NPC warn throttle.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub fn npc_is_incapacitated(
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
    npc_id: u32,
    now: std::time::Instant,
) -> bool {
    dispatch::npc_is_incapacitated(space_mgr, npc_id, now)
}

/// The admit filter's warn throttle. See [`npc_is_incapacitated`].
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub const ZERO_HEALTH_WARN_MIN_INTERVAL: std::time::Duration =
    dispatch::ZERO_HEALTH_WARN_MIN_INTERVAL;

/// One follow tick, for GC1b-2's chain-replay suite
/// (`content::chain_replay_tests::gc1_escort`), which asserts `nav_path`
/// after each step rather than running the whole-space `npc_ai_tick`.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub use follow::npc_ai_follow_for_test;

/// Co-located span-field record + counter emission for the
/// `decision_outcome` vocab. The dispatcher span at
/// [`npc_ai_tick`] declares
/// `fields(decision_outcome = tracing::field::Empty)`; each handler
/// fills the slot via this helper, which ALSO increments the
/// `npc_ai_decisions_total{decision_outcome}` counter once per tick.
///
/// Calling this twice in one handler emits two counter increments —
/// callers should pick one terminal outcome per tick.
pub(super) fn record_decision_outcome(outcome: &'static str) {
    set_last_outcome(outcome);
    tracing::Span::current().record("decision_outcome", outcome);
    cimmeria_observability::counter!(
        "npc_ai_decisions_total",
        "decision_outcome" => outcome,
    );
}

tokio::task_local! {
    /// The terminal outcome of the handler that just ran, for the per-tick
    /// `npc_ai.tick` row and the `stuck` detector. Scoped per NPC turn by
    /// [`with_outcome_slot`].
    ///
    /// Task-local rather than a process-wide static: the static was shared
    /// by every concurrently running test, so one test's dispatcher could
    /// clear or read another's outcome (the `tick_row` guard failed
    /// intermittently once NA02 added more AI-tick tests). A task-local
    /// follows the future across worker threads, so production — one cell
    /// task, NPCs strictly in sequence — sees exactly what it did before.
    static LAST_OUTCOME: std::cell::Cell<&'static str>;
}

fn set_last_outcome(outcome: &'static str) {
    // Outside a slot (the retry sweep, tests calling a handler directly)
    // there is nobody to read it: dropping it is what the old static's
    // "cleared before the next handler" amounted to.
    let _ = LAST_OUTCOME.try_with(|c| c.set(outcome));
}

pub(super) fn take_last_outcome() -> &'static str {
    LAST_OUTCOME.try_with(|c| c.replace("")).unwrap_or("")
}

/// Run one NPC's AI turn with its own outcome slot.
pub(super) async fn with_outcome_slot<F: std::future::Future>(f: F) -> F::Output {
    LAST_OUTCOME.scope(std::cell::Cell::new(""), f).await
}

/// `fight.rs` writes `decision_outcome` as an inline log field, so its
/// decisions never reached the `npc_ai_decisions_total` counter or the span.
/// Called immediately before each of those log lines.
pub(super) fn note_outcome(outcome: &'static str) {
    record_decision_outcome(outcome);
}
