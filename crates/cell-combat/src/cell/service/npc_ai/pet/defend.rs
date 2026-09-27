//! The owner's side of a pet's fight (D-PT06).
//!
//! The owner's combat state is the player `threatened_mobs` set and the
//! `BSF_InCombat` bit (`combat::threat::player_combat`). A mob adds a player
//! to that set when the player's own threat lands on it. A pet's threat
//! lands the pet, not the owner, so on its own it never put the owner in
//! combat (audit A-30). [`sync_owner_combat`] mirrors it on every pet turn:
//!
//! - every mob that has the pet on its threat list enters the owner's set;
//! - an entry that neither the owner nor any of the owner's pets still
//!   explains leaves it.
//!
//! Exits also happen without it: the leash drain and the dead-NPC sweep walk
//! every player whose set names the mob.

use tokio::sync::mpsc;

use crate::cell::combat;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Mirror the pet's fights into its owner's combat state, and drop the
/// entries nothing explains any more. Sends the owner `onStateFieldUpdate` on
/// each `BSF_InCombat` edge.
pub(super) async fn sync_owner_combat(
    pet_id: u32,
    owner_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let mobs = space_mgr.npc_ids_in_space_of(pet_id);
    let Some(owner) = space_mgr.get_entity(owner_id) else {
        return;
    };
    // Only a fight the owner could have picked itself puts it in combat: a
    // mob that is not attackable (#444, `player_may_attack`) and still lists
    // the pet does not.
    let engaged: Vec<u32> = mobs
        .iter()
        .copied()
        .filter(|&m| {
            space_mgr.get_entity(m).is_some_and(|e| {
                e.threat_list.contains_key(&pet_id)
                    && !combat::is_dead_state(e.state_field)
                    && super::fight_refusal(owner, e).is_none()
            })
        })
        .collect();
    for mob_id in engaged {
        if let Some(state) = combat::enter_player_combat(space_mgr, owner_id, mob_id) {
            let id = super::owner_identity(space_mgr, pet_id, owner_id);
            tracing::debug!(
                target: "pets.ai",
                entity_id = pet_id,
                event = "owner_combat_entered",
                decision_outcome = "owner_combat_entered",
                pet_id,
                owner_id,
                account_id = id.account_id,
                player_id = id.player_id,
                target_id = mob_id,
                new_state = state,
                "pet: the pet's fight put its owner in combat"
            );
            // Appearance first, then the state bit: the same order and
            // fan-out as the owner's own first hit (`damage_apply`).
            crate::cell::abilities::request_appearance_refresh(owner_id, tx, space_mgr).await;
            crate::cell::abilities::send_entity_method_to_self_and_witnesses(
                owner_id,
                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                state.to_le_bytes().to_vec(),
                tx,
                space_mgr,
            )
            .await;
        }
    }

    // Only pets the owner itself summoned explain an entry: a pet left under
    // a reused owner id is not this player's.
    let owner_identity = space_mgr.player_identity(owner_id);
    let pets: Vec<u32> = space_mgr
        .pets
        .pets_of(owner_id)
        .into_iter()
        .filter(|&p| space_mgr.pets.summoner_matches(p, owner_identity))
        .collect();
    let stale: Vec<u32> = space_mgr
        .get_entity(owner_id)
        .map(|o| o.threatened_mobs.iter().copied().collect::<Vec<u32>>())
        .unwrap_or_default()
        .into_iter()
        // Only NPC sources are this sweep's to prune: a player source (a duel
        // opponent, SS-D2) has no threat list to consult and is cleared by
        // the system that added it, never here.
        .filter(|m| !space_mgr.get_entity(*m).is_some_and(|e| e.is_player))
        .filter(|m| {
            space_mgr.get_entity(*m).is_none_or(|e| {
                combat::is_dead_state(e.state_field)
                    || !(e.threat_list.contains_key(&owner_id)
                        || pets.iter().any(|p| e.threat_list.contains_key(p)))
            })
        })
        .collect();
    for mob_id in stale {
        if let Some(state) = combat::exit_player_combat(space_mgr, owner_id, mob_id) {
            let id = super::owner_identity(space_mgr, pet_id, owner_id);
            tracing::debug!(
                target: "pets.ai",
                entity_id = pet_id,
                event = "owner_combat_left",
                decision_outcome = "owner_combat_left",
                pet_id,
                owner_id,
                account_id = id.account_id,
                player_id = id.player_id,
                target_id = mob_id,
                new_state = state,
                "pet: the pet's fight ended, its owner left combat"
            );
            // To the owner's own client only, as `leash::send_combat_exits`
            // does for the same bit.
            crate::cell::abilities::send_entity_method(
                owner_id,
                crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                state.to_le_bytes().to_vec(),
                tx,
                space_mgr,
            )
            .await;
        }
    }
}
