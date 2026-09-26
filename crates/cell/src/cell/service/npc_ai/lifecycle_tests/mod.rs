//! `AiState::Submit` disengagement guards (Harset H08) that drive the
//! service loop's `auto_cycle_tick`, so they cannot move to
//! `cimmeria-cell-combat` with the rest of the surrender guards
//! (`cell::service::npc_ai::lifecycle::tests` there; wave C2 of
//! `docs/architecture/services-crate-split.md`).
//!
//! - [`auto_cycle`] — the auto-fire loop, both the one-shot sweep at
//!   surrender and the per-tick validity gate, which is what actually closes
//!   the auto-fire kill window (100 ms, faster than the ~2 s AI handler).
//! - [`health_crossing`] — the packet's acceptance case driven end to
//!   end through the real damage seam and a content chain, rather than
//!   through a hand-built `ai_state` write.
//!
//! The fixtures are `cimmeria_cell_combat::test_fixtures::npc_surrender`,
//! shared with the guards in the combat crate.

mod auto_cycle;
mod health_crossing;

use std::collections::HashMap;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;

use crate::cell::combat::{generate_threat, BSF_AUTO_CYCLING, BSF_IN_COMBAT};
use crate::cell::space_manager::SpaceManager;

pub(super) use cimmeria_cell_combat::test_fixtures::npc_surrender::*;
