//! The `abilities.snapshot` telemetry row (ability-mechanics AB-T5).
//!
//! [`CellEntity::ability_state`] builds the snapshot (warmup, cooldowns,
//! pulses, ledger entries and absorb pools, `state_field` refcounts, every
//! stat); this module puts it in SigNoz at the moments a human or a UAT
//! anchor will want to read it back:
//!
//! - a `.bug` bookmark ([`SnapshotTrigger::Bookmark`], carrying the
//!   bookmark's `bookmark_id`), so every UAT anchor records the state its
//!   row started from;
//! - a player's death ([`SnapshotTrigger::Death`]), taken before the
//!   `EF_ClearOnDeath` strip, so it shows what the player died with;
//! - a player's logout ([`SnapshotTrigger::Logout`]), before the teardown.
//!
//! One INFO row each: low frequency, high signal. The whole snapshot rides
//! in one compact JSON field, `snapshot`, beside a few flat counts a query
//! can filter on without parsing it. Deaths and logouts are logged for
//! players only: an NPC dies far more often than a player and has no
//! session to debug, and the lab's `server_ability_state` reads any entity
//! live.
//!
//! [`CellEntity::ability_state`]: cimmeria_entity::cell_entity::CellEntity::ability_state

use std::time::Instant;

use cimmeria_entity::cell_entity::AbilityStateSnapshot;

use crate::cell::space_manager::SpaceManager;

/// Why a snapshot row was written: its `trigger` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotTrigger {
    /// A `.bug` bookmark.
    Bookmark,
    /// The entity died.
    Death,
    /// The player logged out.
    Logout,
}

impl SnapshotTrigger {
    /// Stable `trigger` value for logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bookmark => "bookmark",
            Self::Death => "death",
            Self::Logout => "logout",
        }
    }
}

impl SpaceManager {
    /// `entity_id`'s ability state as of now, or `None` when it does not
    /// exist. Read-only.
    pub fn ability_state(&self, entity_id: u32) -> Option<AbilityStateSnapshot> {
        self.get_entity(entity_id)
            .map(|e| e.ability_state(Instant::now()))
    }
}

/// Write one `abilities.snapshot` row for `entity_id`. Returns whether a row
/// was written (`false`: no such entity).
///
/// `bookmark_id` joins a [`SnapshotTrigger::Bookmark`] row to its
/// `playtest.bookmark` header; `None` for the other triggers.
pub fn log_ability_snapshot(
    space_mgr: &SpaceManager,
    entity_id: u32,
    trigger: SnapshotTrigger,
    bookmark_id: Option<u64>,
) -> bool {
    let Some(snapshot) = space_mgr.ability_state(entity_id) else {
        tracing::debug!(
            target: "abilities.snapshot",
            event = "ability_snapshot_skipped",
            reason = "entity_missing",
            entity_id,
            entity_name = space_mgr.entity_label(entity_id),
            trigger = trigger.as_str(),
            "ability snapshot not written: the entity is gone"
        );
        return false;
    };
    emit(space_mgr, &snapshot, trigger, bookmark_id);
    true
}

fn emit(
    space_mgr: &SpaceManager,
    s: &AbilityStateSnapshot,
    trigger: SnapshotTrigger,
    bookmark_id: Option<u64>,
) {
    let who = space_mgr.player_identity(s.entity_id);
    // A snapshot of plain data cannot fail to serialise; the fallback keeps
    // the row (and its counts) if that ever changes.
    let json = serde_json::to_string(s).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"));
    tracing::info!(
        target: "abilities.snapshot",
        event = "ability_snapshot",
        trigger = trigger.as_str(),
        bookmark_id, // nt:id-only a UAT bookmark's sequence number, unnamed
        entity_id = s.entity_id,
        entity_name = space_mgr.entity_label(s.entity_id),
        account_id = who.account_id,
        account_name = who.account_name,
        player_id = who.player_id,
        player_name = who.player_name,
        is_player = s.is_player,
        state_field = s.state_field,
        state_field_names = %cimmeria_wire::state_field::STATE_FLAGS.render(s.state_field),
        pending_cast_id = s.pending_cast.as_ref().map(|p| p.cast_id), // nt:id-only per-cast sequence number, no name exists
        pending_ability_id = s.pending_cast.as_ref().map(|p| p.ability_id),
        pending_ability_name = super::content_names::ability_name(s.pending_cast.as_ref().map(|p| p.ability_id)),
        cooldowns = s.cooldowns.len(),
        pulsing = s.pulsing.len(),
        ledger = s.ledger.len(),
        pending_timer_clears = s.pending_timer_clears.len(),
        snapshot = %json,
        "ability state snapshot"
    );
}

#[cfg(test)]
#[path = "ability_snapshot_tests.rs"]
mod tests;
