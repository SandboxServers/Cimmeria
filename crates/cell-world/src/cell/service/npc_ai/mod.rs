//! NPC AI state primitives, the part of the AI every lower cell system calls
//! (docs/architecture/services-crate-split.md §2C):
//!
//! - [`transition`] — `set_ai_state`, the single writer of `ai_state`, which
//!   emits `npc_ai.transition` and `npc_ai_transitions_total`.
//! - [`movement_stop`] — stopping and rerouting an NPC: the only writers of
//!   `nav_path` outside the movement tick (NA10).
//! - [`leash::policy`] — the leash radius and the rules that decide when a
//!   fighting NPC gives up (NA12).
//! - [`detectors`] — NA02's stuck / stale / floating / leash-loop / LoS /
//!   off-mesh rows. Reporting only; they change no decision.
//!
//! The AI's behaviour (the tick, fighting, chasing, patrolling, the leash walk
//! home) is in `cimmeria-cell-combat`'s `cell::service::npc_ai`, which
//! re-exports these modules at their old paths.

pub mod detectors;
pub mod leash;
pub mod movement_stop;
pub mod transition;

// Stopping and rerouting an NPC. Combat and the content executor reach them
// here.
pub use movement_stop::{
    replace_nav_path_on, snap_npc_to, stop_movement_on, stop_npc_movement, StopReason,
};
// The AI-state transition helper is the only way to change `ai_state` (the
// field is private in the entity crate). Callers across `cell` -- combat, the
// content executor, the GM console, the respawn tick -- reach it through this
// re-export.
pub use transition::{set_ai_state, set_ai_state_on, world_label, AiTransitionReason};

// Test fixtures arrange a starting state without a transition row.
#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
pub use transition::force_ai_state;
