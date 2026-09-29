//! Per-frame tick handlers: AoI propagation, reload-completion promotion,
//! holster-deferred attack/reload promotion, out-of-combat regen, NPC
//! movement along nav paths, NPC respawn promotion, and the in-combat
//! player vitals sample.
//!
//! Each tick family lives in its own submodule; this `mod.rs` is purely
//! the module wiring + re-exports so callers in `message_loop` keep their
//! `super::ticks::<tick>` paths unchanged.

mod aoi;
mod auto_cycle;
mod cover;
mod crafting_stations;
pub(crate) mod holster;
mod npc_ground;
mod npc_movement;
mod npc_respawn;
mod pending_holster;
mod regen;
mod reload_completion;
mod vitals;

pub(super) use aoi::run_aoi_tick;
pub(super) use auto_cycle::auto_cycle_tick;
pub(super) use cover::cover_detection_tick;
pub(super) use crafting_stations::crafting_station_tick;
// The holster animation constant is combat's (§2F of
// docs/architecture/services-crate-split.md); re-exported at its old path.
pub(crate) use crate::cell::combat::HOLSTER_ANIMATION_DURATION;
pub(super) use holster::{holster_timer_tick, pending_slot_swap_tick};
pub(super) use npc_movement::npc_movement_tick;
pub(super) use npc_respawn::npc_respawn_tick;
pub(super) use pending_holster::{pending_attack_tick, pending_reload_tick};
pub(super) use regen::regen_tick;
pub(super) use reload_completion::reload_completion_tick;
pub(super) use vitals::{vitals_sample_tick, VITALS_SAMPLE_EVERY_TICKS};
