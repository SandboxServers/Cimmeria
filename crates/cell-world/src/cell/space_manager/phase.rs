//! Player phase membership (Tollana campaign, PH-01b).
//!
//! The visibility rule is `cimmeria_entity::cell_entity::phase_visible`,
//! applied in `compute_player_aoi`. This module only changes a player's
//! `PhaseState` and recomputes that player's AoI at once, so a phase change
//! shows on the client as an ordinary leave or a fresh create instead of
//! waiting for the next tick. Nothing here is persisted.

use std::fmt;

use cimmeria_entity::cell_entity::{PhaseKey, PhaseState};

use super::super::messages::CellToBaseMsg;
use super::SpaceManager;

/// Why `set_player_phase` refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseError {
    /// No cell entity with that id.
    PlayerMissing,
    /// The entity exists but is not a player.
    NotAPlayer,
}

impl fmt::Display for PhaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PhaseError::PlayerMissing => f.write_str("no such cell entity"),
            PhaseError::NotAPlayer => f.write_str("entity is not a player"),
        }
    }
}

impl std::error::Error for PhaseError {}

impl SpaceManager {
    /// Add (`member`) or remove `phase` from the player's held phases and
    /// return the AoI events that follow from it. A no-op change returns an
    /// empty list without recomputing. The caller must forward every returned
    /// event: the witness set is already updated, so a dropped introduction
    /// would not be sent again.
    pub fn set_player_phase(
        &mut self,
        player_id: u32,
        phase: &PhaseKey,
        member: bool,
    ) -> Result<Vec<CellToBaseMsg>, PhaseError> {
        let entity = self
            .get_entity_mut(player_id)
            .ok_or(PhaseError::PlayerMissing)?;
        if !entity.is_player {
            return Err(PhaseError::NotAPlayer);
        }
        let changed = match entity.extensions.get_mut::<PhaseState>() {
            Some(state) if member => state.player_phases.insert(phase.clone()),
            Some(state) => state.player_phases.remove(phase),
            // Removing a phase the player never held leaves no state behind.
            None if member => {
                let mut state = PhaseState::default();
                state.player_phases.insert(phase.clone());
                entity.extensions.insert(state);
                true
            }
            None => false,
        };
        if !changed {
            return Ok(Vec::new());
        }
        let events = self.compute_aoi_changes_for_player(player_id);
        let entered = events
            .iter()
            .filter(|e| matches!(e, CellToBaseMsg::EnteredAoI { .. }))
            .count();
        let left = events
            .iter()
            .filter(|e| matches!(e, CellToBaseMsg::LeftAoI { .. }))
            .count();
        tracing::info!(
            event = "phase.changed",
            player_id,
            player_name = self.entity_label(player_id),
            phase = %phase,
            member,
            entered,
            left,
            "phase changed"
        );
        Ok(events)
    }
}
