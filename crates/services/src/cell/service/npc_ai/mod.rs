//! NPC AI tick — fight (threat-target attacks + leashing), and leash recovery.
//!
//! # Cadence
//!
//! Two passes share the AI surface, both driven from
//! [`crate::cell::service::message_loop`]:
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
//!   and the min-range backup-waypoint geometry.
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
pub(in crate::cell) mod detectors;
mod dispatch;
mod fight;
mod fight_cover;
mod fight_target;
mod follow;
#[cfg(test)]
mod ground_endpoint_tests;
mod idle_aggro;
mod investigate;
mod leash;
mod lifecycle;
mod movement_stop;
mod path_failure;
mod path_request;
mod patrol;
mod transition;
mod wander;

// Re-export discipline: external callers reach these via
// `super::npc_ai::X` (message_loop) and
// `crate::cell::service::npc_ai::X` (the sibling `tests/` modules).
// Keeping the names re-exported here means those paths are identical
// after the split — this is an internal refactor, not a public-surface
// change.
// Only test code reaches this via the `npc_ai::` re-export; production
// callers use `ability_select::` directly. Gate to the test build so
// clippy's non-test pass doesn't flag it unused.
//
// Scope is `crate::cell` rather than `super` because the Harset H09
// loader-to-chooser round-trip guard lives with the other live-DB seed
// guards in `cell/spawner/tests/harset/` — it loads a multi-row ability
// set out of Postgres, spawns it, and then drives this selector. Keeping
// that guard next to the spawner tests it shares a fixture with is worth
// one module level of visibility; nothing outside `cell` can see it.
//
// `choose_npc_ability_within_reach` is re-exported at the same scope for
// the same reason: the Harset live-DB guard drives the *production*
// selector (the reach-filtered one `fight` calls) over set 4 and set 5 as
// the seed actually loads them, so the seed and the melee gate are pinned
// together rather than in two tests that could drift apart.
#[cfg(test)]
pub(in crate::cell) use ability_select::{choose_npc_ability, choose_npc_ability_within_reach};
pub(super) use dispatch::{npc_ai_retry_sweep, npc_ai_tick};
// The NA13 chain-replay guards (`content::chain_replay_tests`) run the
// real AI tick against spawns loaded from the seed.
#[cfg(test)]
pub(in crate::cell) async fn npc_ai_tick_for_test(
    tx: &tokio::sync::mpsc::Sender<crate::cell::messages::CellToBaseMsg>,
    space_mgr: &mut crate::cell::space_manager::SpaceManager,
    engine: &cimmeria_content_engine::chain::ChainEngine,
) {
    dispatch::npc_ai_tick(tx, space_mgr, engine).await;
}

// The AI-state transition helper is the only way to change `ai_state`
// (the field is private in the entity crate). Callers across `cell` --
// combat, the content executor, the GM console, the respawn tick -- reach
// it through this re-export.
pub(in crate::cell) use aggro_acquired::log_aggro_acquired;
// NA14: `combat::generate_threat` fans a fresh engagement out to neighbours.
pub(in crate::cell) use assist::recruit_assisters;
// Stopping and rerouting an NPC: the only writers of `nav_path` outside the
// movement tick (NA10). Combat and the content executor reach them here.
pub(in crate::cell) use movement_stop::{
    replace_nav_path_on, snap_npc_to, stop_movement_on, stop_npc_movement, StopReason,
};
pub(in crate::cell) use transition::{
    set_ai_state, set_ai_state_on, world_label, AiTransitionReason,
};

// Test fixtures arrange a starting state without a transition row.
#[cfg(test)]
pub(in crate::cell) use transition::force_ai_state;

// Test-only re-export: the sibling `tests/npc_ai.rs` exercises the
// private `compute_backup_waypoint` degenerate branch through this
// shim. Gated on `cfg(test)` so the non-test build doesn't flag it as
// an unused import.
#[cfg(test)]
pub(super) use ability_select::compute_backup_waypoint_for_test;

// Test-only re-export: `tests/npc_ai/zero_health_guard.rs` drives the
// admit filter with a synthetic clock to pin its per-NPC warn throttle.
#[cfg(test)]
pub(super) use dispatch::{npc_is_incapacitated, ZERO_HEALTH_WARN_MIN_INTERVAL};

// Test-only re-export: GC1b-2's chain-replay suite
// (`crate::cell::content::chain_replay_tests::gc1_escort`) drives
// individual follow AI ticks directly rather than the full
// `npc_ai_tick` dispatcher (which is `pub(in crate::cell::service)` and
// snapshots the entire NPC list) so it can assert `nav_path` after each
// step. `pub(crate)`, not `pub(super)`, because the caller lives outside
// `cell::service` — same pattern as `compute_backup_waypoint_for_test`
// above, one visibility level wider because this caller is a sibling of
// `cell::service`, not a descendant of it. `npc_ai_follow` itself is
// declared `pub(crate)` under `cfg(test)` in `follow.rs` (see that
// wrapper's doc comment) — a `use` re-export cannot widen an item's
// visibility beyond what it was declared with.
#[cfg(test)]
pub(crate) use follow::npc_ai_follow_for_test;

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
