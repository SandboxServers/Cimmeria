//! `AiState::Submit` disengagement guards (Harset H08).
//!
//! The bug shape these reproduce: `npc_ai_submit` used to wipe the NPC's
//! own `threat_list` in place, which leaves every attacker holding the
//! NPC in `threatened_mobs` forever (stuck `BSF_InCombat`, weapon stays
//! drawn, `regen_tick` permanently gated off) and leaves their auto-fire
//! loop running, so the surrendered NPC gets shot dead seconds later.
//!
//! Three stop mechanisms are pinned because they run on different clocks:
//! the AI-side handler (~2 s cadence, reached via `npc_ai_tick`), the
//! `auto_cycle_tick` target-validity gate (100 ms), which is what actually
//! closes the auto-fire kill window, and the `fire_pulse` surrender floor
//! (100 ms), which closes the damage-over-time one. The pulse floor's own
//! guards live with the pulsing tests (`cell::effects::pulsing::tests`), next
//! to the code they revert-verify.
//!
//! The guards are split by what they assert about:
//!
//! - [`player_scrub`] — the attacker's `threatened_mobs` /
//!   `BSF_InCombat` / re-engage behaviour.
//! - [`npc_quiescence`] — what the NPC itself ends up looking like:
//!   non-hostile, channel-free, out of cover, stopped, and facing the
//!   player it surrendered to.
//!
//! `auto_cycle` (the auto-fire loop) and `health_crossing` (the acceptance
//! case end to end through the real damage seam) drive the service loop's
//! auto-cycle tick, so they are `cell::service::npc_ai::lifecycle_tests` in
//! `cimmeria-services`. The fixtures both halves share are
//! `crate::test_fixtures::npc_surrender`.

mod npc_quiescence;
mod player_scrub;

use std::collections::HashMap;
use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::cell_entity::{AiState, MobMovementType};

use crate::cell::combat::{generate_threat, BSF_IN_COMBAT};
use crate::cell::messages::CellToBaseMsg;

pub(super) use crate::test_fixtures::npc_surrender::*;
