//! In-game combat and ability debug (ability-mechanics AB-N1).
//!
//! A GM turns debugging on with the native `gmDebug*` commands (cell methods
//! 169 to 172 and 176, `cimmeria-cell-console`'s `gm/combat_debug.rs`), or a
//! crafted caller with cells 2, 3 and 6. While anyone has it on, the combat
//! pipeline writes **notes** here as it decides: the fire, the QR roll and
//! the pools of each hit, each `effect_planned` row, each NVP damage entry,
//! each landing of a beneficial or routed effect, each ledger entry and each
//! pulse. The notes carry the same values the AB-T3 rows log, taken at the
//! same points.
//!
//! When a cast's scope closes, [`deliver::flush`] turns each finished
//! [`record::CastDebug`] into text with the one formatter ([`format`]),
//! picks who asked for it ([`deliver`]), and for every line writes an
//! `abilities.debug` DEBUG row and sends the same text to the debug target's
//! client as an `onPlayerCommunication` line on `CHAN_FEEDBACK`. The client
//! has no combat-debug window and no handler for `onSendCombatDebug`
//! (`docs/reverse-engineering/findings/native-combat-debug.md`), so the chat
//! line is the only way to show it.
//!
//! Nothing is noted while no one is debugging ([`CombatDebug::is_active`]),
//! so the pipeline pays one empty-map check per decision.
//!
//! - [`settings`]: one watcher's toggles, the `bCombatDebug` family of the
//!   def, kept here rather than on the entity.
//! - [`commands`]: the toggle bodies, shared by the GM handlers and cells
//!   2, 3 and 6, plus `setAbilityDebugTarget` and `clearAbilityDebug`.
//! - [`record`]: the notes and the open records.
//! - [`format`]: notes to text, and the chat-line split.
//! - [`deliver`]: routing, the per-recipient rate limit, the rows and the
//!   sends.

pub mod commands;
pub mod deliver;
pub mod format;
pub mod record;
pub mod settings;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

pub use deliver::{flush, send_feedback_line};
pub use record::{
    CastDebug, CastKind, HitNote, LandingNote, LedgerNote, Note, NvpNote, PlanNote, Pools,
    PulseNote,
};
pub use settings::DebugSettings;

use deliver::RateWindow;

/// Open records kept before the oldest is finalized early. A cast closes its
/// own record when its scope ends; this bound only matters for notes made
/// outside any scope that no flush picks up for a while.
pub(crate) const MAX_OPEN_RECORDS: usize = 64;

/// Notes kept per record. A cone that hits fifty targets is still one cast;
/// past this the record keeps a count of what it dropped and says so.
pub(crate) const MAX_NOTES_PER_RECORD: usize = 96;

/// `entity_id`'s HEALTH and FOCUS now (zeros when it is gone).
pub fn pools_of(mgr: &crate::cell::space_manager::SpaceManager, entity_id: u32) -> Pools {
    use cimmeria_entity::stats::{FOCUS, HEALTH};
    mgr.get_entity(entity_id).map_or(Pools::default(), |e| {
        let cur = |id| e.stats.get(id).map_or(0, |s| s.cur);
        Pools {
            health: cur(HEALTH),
            focus: cur(FOCUS),
        }
    })
}

/// The combat-debug state of one cell (`SpaceManager::combat_debug`).
#[derive(Debug, Default)]
pub struct CombatDebug {
    /// Who has debugging on, by entity id. Empty means nothing is noted.
    pub(crate) watchers: HashMap<u32, DebugSettings>,
    /// Casts being resolved, in the order they were opened.
    pub(crate) open: Vec<CastDebug>,
    /// Per-recipient line budget (pulse storms), by recipient entity id.
    pub(crate) windows: HashMap<u32, RateWindow>,
}

impl CombatDebug {
    /// Whether anyone has debugging on, so a note would be kept.
    pub fn is_active(&self) -> bool {
        !self.watchers.is_empty()
    }

    /// The settings of `entity_id`, if it has any toggle on.
    pub fn settings(&self, entity_id: u32) -> Option<&DebugSettings> {
        self.watchers.get(&entity_id)
    }

    /// Note one decision of `caster_id`'s cast `cast_id`. A no-op while no
    /// one is debugging.
    pub fn note(&mut self, caster_id: u32, cast_id: Option<i32>, ability_id: i32, note: Note) {
        if !self.is_active() {
            return;
        }
        let idx = match self
            .open
            .iter()
            .position(|r| r.caster_id == caster_id && r.cast_id == cast_id)
        {
            Some(i) => i,
            None => {
                if self.open.len() >= MAX_OPEN_RECORDS {
                    // Keep it for the next flush rather than lose it: the
                    // oldest record is the one no scope will close.
                    let oldest = self.open.remove(0);
                    tracing::debug!(
                        target: "abilities.debug",
                        event = "combat_debug_record_evicted",
                        caster_id = oldest.caster_id,
                        cast_id = oldest.cast_id,
                        ability_id = oldest.ability_id,
                        open = MAX_OPEN_RECORDS,
                        "combat debug: oldest open record evicted unflushed"
                    );
                }
                self.open
                    .push(CastDebug::new(caster_id, cast_id, ability_id));
                self.open.len() - 1
            }
        };
        self.open[idx].push(note);
    }
}
