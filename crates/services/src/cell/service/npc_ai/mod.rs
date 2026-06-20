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

mod ability_select;
mod dispatch;
mod fight;
mod follow;
mod investigate;
mod leash;
mod lifecycle;
mod patrol;
mod wander;

// Re-export discipline: external callers reach these via
// `super::npc_ai::X` (message_loop) and
// `crate::cell::service::npc_ai::X` (the sibling `tests/` modules).
// Keeping the names re-exported here means those paths are identical
// after the split — this is an internal refactor, not a public-surface
// change.
pub(super) use ability_select::choose_npc_ability;
pub(super) use dispatch::{npc_ai_retry_sweep, npc_ai_tick};

// Test-only re-export: the sibling `tests/npc_ai.rs` exercises the
// private `compute_backup_waypoint` degenerate branch through this
// shim. Gated on `cfg(test)` so the non-test build doesn't flag it as
// an unused import.
#[cfg(test)]
pub(super) use ability_select::compute_backup_waypoint_for_test;

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
    tracing::Span::current().record("decision_outcome", outcome);
    cimmeria_observability::counter!(
        "npc_ai_decisions_total",
        "decision_outcome" => outcome,
    );
}
