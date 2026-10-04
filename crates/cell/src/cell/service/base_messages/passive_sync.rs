//! The passive pass at the three seams where the known set changes (world
//! entry, a purchase or GM grant, a respec), and the stat update it owes.
//!
//! Since ability mechanics AB-08 a stat passive (1450 Cover Penetration,
//! 1731 Warrior's Resilience) moves a stat through the timed effect ledger.
//! The client's numbers come from the base's world-entry burst before
//! `InitPlayerState`, and nothing else flushes this entity's stats at these
//! seams, so the pass sends the dirty stats itself when a script ran.

use tokio::sync::mpsc;

use crate::cell::effects::passives::{apply_passives, PassiveChange};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// [`apply_passives`] on `entity_id`, then its dirty stats to its own
/// client and its witnesses. Returns how many effect scripts ran.
pub(super) async fn apply_passives_and_sync(
    entity_id: u32,
    ability_ids: &[i32],
    change: PassiveChange,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    let ran = apply_passives(space_mgr, entity_id, ability_ids, change);
    if ran == 0 {
        return 0;
    }
    let Some(entity) = space_mgr.get_entity_mut(entity_id) else {
        return ran;
    };
    let payload = entity.stats.serialize_dirty();
    entity.stats.clear_dirty();
    // serialize_dirty always emits the 4-byte count prefix.
    if payload.len() > 4 {
        // Self and witnesses, as every other stat change (`fire_beneficial`'s
        // flush): a passive's stats are as public as a buff's.
        let _ = crate::cell::abilities::send_entity_method_to_self_and_witnesses(
            entity_id,
            crate::mercury::method_idx::ON_STAT_UPDATE,
            payload,
            tx,
            space_mgr,
        )
        .await;
    }
    ran
}
