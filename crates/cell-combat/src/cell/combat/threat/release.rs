//! Taking an NPC out of every player's fight when it leaves without dying:
//! a GM `.despawn` / `.delspawn` / `/gmdespawn`, a `.respawnall` reset, a
//! content `despawn_entity`, the AI `Despawning` state, a lab dummy's
//! expiry.
//!
//! **Why.** A hit puts the NPC in the attacker's `threatened_mobs`, and
//! `BSF_InCombat` stays set while that set is non-empty (no regen, no
//! out-of-combat holster). Death drains the corpse from every set
//! (`clear_dead_npc_from_all_player_threat`) and the leash drains a mob that
//! gives up, but `SpaceManager::despawn_npc` only removes the entity: a
//! player who had hit a despawned mob stayed in combat with something that
//! no longer existed, until relog.
//!
//! [`release_npc_from_player_combat`] is the one drain for those paths, and
//! [`despawn_npc_releasing_combat`] the despawn that runs it first.

use tokio::sync::mpsc;

use super::drain_npc_from_player_combat;
use crate::cell::abilities::send_entity_method_to_self_and_witnesses;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::{DespawnOutcome, SpaceManager};
use crate::mercury::method_idx::ON_STATE_FIELD_UPDATE;

/// Drop `npc_id` from every player's combat set and clear its threat list.
/// Each player whose `BSF_InCombat` just cleared is sent the new state field,
/// with their witnesses; a player still fighting another mob keeps it and is
/// sent nothing. `why` is the row's `reason`. Returns how many players left
/// combat. A no-op for a player id or a missing entity.
pub async fn release_npc_from_player_combat(
    npc_id: u32,
    why: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> usize {
    if !space_mgr.get_entity(npc_id).is_some_and(|e| !e.is_player) {
        return 0;
    }
    let exits = drain_npc_from_player_combat(space_mgr, npc_id);
    if let Some(npc) = space_mgr.get_entity_mut(npc_id) {
        npc.threat_list.clear();
    }
    for &(player_id, state) in &exits {
        send_entity_method_to_self_and_witnesses(
            player_id,
            ON_STATE_FIELD_UPDATE,
            state.to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;
    }
    if !exits.is_empty() {
        // Module-path target: exported by `OTEL_FILTER`'s `cimmeria_cell_combat=debug`.
        tracing::debug!(
            event = "npc_released_from_player_combat",
            npc_id,
            reason = why,
            combat_exits = exits.len(),
            players = ?exits.iter().map(|&(p, _)| p).collect::<Vec<_>>(),
            "NPC left without dying; players whose last threat it was left combat"
        );
    }
    exits.len()
}

/// [`release_npc_from_player_combat`], then `SpaceManager::despawn_npc`.
/// Every non-death despawn of an NPC a player may have hit goes through here.
pub async fn despawn_npc_releasing_combat(
    npc_id: u32,
    why: &'static str,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> DespawnOutcome {
    release_npc_from_player_combat(npc_id, why, tx, space_mgr).await;
    space_mgr.despawn_npc(npc_id, tx).await
}
