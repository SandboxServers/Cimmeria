//! The NPC AI, at its old path.
//!
//! The AI's behaviour (the tick, fighting, chasing, cover, assist, leashing,
//! patrol, wander, follow) is in `cimmeria-cell-combat`, and its state
//! primitives (the detectors, `set_ai_state`, movement stop, the leash policy)
//! are in `cimmeria-cell-world` (waves C2 and C1 of
//! `docs/architecture/services-crate-split.md`). This module re-exports all of
//! it, so every `crate::cell::service::npc_ai::X` path compiles unchanged.
//!
//! Two test suites stay here because their shared fixtures drive a tick of
//! the service loop: the NA02 detector tests (`ticks::npc_movement_tick`) and
//! the H08 surrender guards (`ticks::auto_cycle_tick`, and a chain engine for
//! the health-crossing case).

pub(crate) use cimmeria_cell_combat::cell::service::npc_ai::*;

#[cfg(test)]
mod detector_tests;
#[cfg(test)]
mod lifecycle_tests;
