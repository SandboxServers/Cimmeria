//! Per-frame tick handlers: AoI propagation, reload-completion promotion,
//! holster-deferred attack/reload promotion, out-of-combat regen, NPC
//! movement along nav paths, and NPC respawn promotion.
//!
//! Each tick family lives in its own submodule; this `mod.rs` is purely
//! the module wiring + re-exports so callers in `message_loop` keep their
//! `super::ticks::<tick>` paths unchanged.

mod aoi;
mod auto_cycle;
mod cover;
pub(crate) mod holster;
mod npc_movement;
mod npc_respawn;
mod pending_holster;
mod regen;
mod reload_completion;

pub(super) use aoi::run_aoi_tick;
pub(super) use auto_cycle::auto_cycle_tick;
pub(super) use cover::cover_detection_tick;
pub(crate) use holster::HOLSTER_ANIMATION_DURATION;
pub(super) use holster::{holster_timer_tick, pending_slot_swap_tick};
pub(super) use npc_movement::npc_movement_tick;
pub(super) use npc_respawn::npc_respawn_tick;
pub(super) use pending_holster::{pending_attack_tick, pending_reload_tick};
pub(super) use regen::regen_tick;
pub(super) use reload_completion::reload_completion_tick;
