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
//! - [`fight`] — the Fighting handler and Idle-auto-aggro seed.
//! - [`ability_select`] — ability bucket choice, range resolution,
//!   and the min-range backup-waypoint geometry.
//! - [`patrol`] / [`wander`] / [`investigate`] / [`follow`] — the
//!   movement-state handlers.
//! - [`leash`] — leash recovery (snap home + heal).
//! - [`lifecycle`] — the terminal / quiescent states (despawn,
//!   submit, error).
//! - [`path_failure`] — the one throttled emitter every state above
//!   uses when `find_path` gives it nothing usable.

mod ability_select;
mod dispatch;
mod fight;
mod follow;
mod investigate;
mod leash;
mod lifecycle;
mod path_failure;
mod patrol;
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

// Test-only re-export: the sibling `tests/npc_ai.rs` exercises the
// private `compute_backup_waypoint` degenerate branch through this
// shim. Gated on `cfg(test)` so the non-test build doesn't flag it as
// an unused import.
#[cfg(test)]
pub(super) use ability_select::compute_backup_waypoint_for_test;

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

/// The terminal outcome of the handler that just ran, for the per-tick
/// `npc_ai.tick` row. The AI tick runs NPCs strictly one after another on the
/// cell task, so one slot is enough; `dispatch` clears it before each handler.
static LAST_OUTCOME: std::sync::Mutex<&'static str> = std::sync::Mutex::new("");

fn set_last_outcome(outcome: &'static str) {
    *LAST_OUTCOME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = outcome;
}

pub(super) fn take_last_outcome() -> &'static str {
    std::mem::take(
        &mut *LAST_OUTCOME
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    )
}

/// `fight.rs` writes `decision_outcome` as an inline log field, so its
/// decisions never reached the `npc_ai_decisions_total` counter or the span.
/// Called immediately before each of those log lines.
pub(super) fn note_outcome(outcome: &'static str) {
    record_decision_outcome(outcome);
}
