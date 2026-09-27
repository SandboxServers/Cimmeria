//! Loot display + `lootItem` handler — `onLootDisplay` (flat 114) and the
//! `lootItem(index)` cell method — and, in [`restore`], putting an item back
//! on its corpse when the base refuses the grant.

use tokio::sync::mpsc;

use crate::cell::messages::{CellToBaseMsg, LootGrantSource};
use crate::cell::space_manager::SpaceManager;

mod restore;

pub use restore::handle_loot_grant_refused;

/// Send `onLootDisplay` (flat index 114) to the player with the NPC's loot.
///
/// Wire format per LootItemQuantity from alias.xml:
///   `itemID:i32, quantity:i16, index:i32, typeID:i32`
/// Outer: `entityId:i32, ARRAY<LootItemQuantity>, initial:i8`
///
/// `initial = 1` for the first display (opens the window), `0` for subsequent
/// refreshes after a lootItem (client refreshes contents; closes the window
/// if the list is now empty per Loot.lua's `LootWin:hide()` on count==0).
///
/// Reference: `python/cell/interactions/Lootable.py:sendLootList()`
pub(super) async fn send_loot_display(
    player_id: u32,
    npc_entity_id: i32,
    initial: u8,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    // Read loot items from the target entity
    let loot_items: Vec<(Option<i32>, i32, i32)> = space_mgr
        .get_entity(npc_entity_id as u32)
        .map(|e| {
            e.loot
                .iter()
                .map(|li| (li.design_id, li.quantity, li.index))
                .collect()
        })
        .unwrap_or_default();

    let count = loot_items.len() as u32;
    // Per item: 4 (itemID) + 2 (quantity i16) + 4 (index) + 4 (typeID) = 14 bytes
    let mut args = Vec::with_capacity(4 + 4 + loot_items.len() * 14 + 1);
    args.extend_from_slice(&npc_entity_id.to_le_bytes()); // EntityID
    args.extend_from_slice(&count.to_le_bytes()); // ARRAY count

    for (design_id, quantity, index) in &loot_items {
        let item_id = design_id.unwrap_or(0); // 0 = naquadah (cash)
        let type_id = if design_id.is_some() { 1i32 } else { 2i32 }; // LOOT_Item=1, LOOT_Cash=2
        args.extend_from_slice(&item_id.to_le_bytes()); // itemID: INT32
        args.extend_from_slice(&(*quantity as i16).to_le_bytes()); // quantity: INT16
        args.extend_from_slice(&index.to_le_bytes()); // index: INT32
        args.extend_from_slice(&type_id.to_le_bytes()); // typeID: INT32
    }

    args.push(initial);

    tracing::debug!(
        player_id,
        npc_entity_id,
        count,
        initial,
        "Sending onLootDisplay"
    );
    let _ = tx
        .send(CellToBaseMsg::EntityMethodCall {
            entity_id: player_id,
            method_index: crate::mercury::method_idx::ON_LOOT_DISPLAY,
            args,
        })
        .await;
}

/// Handle `lootItem(index)` cell method call.
///
/// The player picks up one item from a lootable NPC's corpse. On success:
/// 1. Remove the item from the NPC's loot list
/// 2. If it's cash (design_id=None), send `onCashChanged` to player
/// 3. If it's an item, send `onUpdateItem` to player
/// 4. Send updated loot list to all players with the loot window open
/// 5. If loot is now empty, clear INT_NormalLoot on the NPC
///
/// Reference: `python/cell/interactions/Lootable.py:onLootItem()`
#[tracing::instrument(
    name = "loot.take_item",
    level = "info",
    skip_all,
    fields(entity_id, index)
)]
pub async fn handle_loot_item(
    entity_id: u32,
    index: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Find which entity the player is looting
    let looting_target = space_mgr
        .get_entity(entity_id)
        .and_then(|e| e.looting_entity);

    let target_eid = match looting_target {
        Some(eid) => eid,
        None => {
            // Benign race: client-side "Loot All" fires `lootItem(i)` per
            // entry in the window. The server's loot-exhaustion path
            // (further down) already clears `looting_entity` and sends an
            // empty `onLootDisplay` so the client closes the window —
            // but any clicks that were in-flight before the close
            // arrives land here with no work to do. Demoted from WARN
            // because it surfaced as 2× false positives per "Loot All"
            // sequence on lomiada's 2026-06-04 session without any
            // gameplay impact, and there is no defensive action we
            // could take here (no NPC id to address a close at).
            tracing::debug!(entity_id, index, "lootItem: player not looting anything");
            return;
        }
    };

    // Validate the looter has a player_id BEFORE mutating the corpse loot
    // list. The earlier ordering removed the item first and only then
    // checked, so an invalid looter (no player_id) lost the drop forever
    // — the corpse table was already mutated.
    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(id) => id,
        None => {
            tracing::warn!(
                entity_id,
                target_eid,
                "lootItem: looter has no player_id; aborting without removing the drop"
            );
            return;
        }
    };

    // Server-authority range re-check (#446). `looting_entity` was pinned
    // at interact time, where the distance to the corpse was checked once
    // against `MAX_INTERACT_DISTANCE`. Nothing re-validates it afterward,
    // so a client that spoofs its position (or simply walks away while the
    // loot window is open) can keep calling `lootItem` from anywhere in the
    // zone — "vacuum loot" every corpse without traversing to it. Re-check
    // the LIVE distance on every take so the pin alone can't be replayed
    // from out of range. The corpse-position read is cheap (no DB, no
    // fan-out).
    //
    // This does NOT yet enforce kill-credit / loot ownership (a player who
    // dealt 0 damage can still loot a corpse they walked up to) — that's
    // the larger SGW lootability-window model, tracked as the follow-up
    // half of #446 and routed through the combat-systems advisor.
    {
        let looter_pos = space_mgr.get_entity(entity_id).map(|e| e.position);
        let corpse_pos = space_mgr.get_entity(target_eid).map(|e| e.position);
        if let (Some(lp), Some(cp)) = (looter_pos, corpse_pos) {
            let dist = lp.distance_to(&cp);
            if dist > super::dispatch::MAX_INTERACT_DISTANCE {
                tracing::warn!(
                    entity_id,
                    target_eid,
                    index,
                    dist,
                    max = super::dispatch::MAX_INTERACT_DISTANCE,
                    "lootItem rejected -- looter is out of range of the corpse \
                     (position spoof or walked away); no item removed (#446)"
                );
                return;
            }
        }
    }

    // Find and remove the loot item from the NPC. The corpse's respawn and
    // template are captured with it, so a refused grant can check that it is
    // putting the item back on the same body.
    let (removed_item, source) = {
        let target = match space_mgr.get_entity_mut(target_eid) {
            Some(e) => e,
            None => {
                tracing::warn!(
                    entity_id,
                    target_eid,
                    index,
                    "lootItem: target entity not found"
                );
                return;
            }
        };

        let pos = target.loot.iter().position(|li| li.index == index);
        let item = match pos {
            Some(i) => target.loot.remove(i),
            None => {
                tracing::warn!(entity_id, target_eid, index, "lootItem: invalid index");
                return;
            }
        };
        let source = LootGrantSource {
            corpse_id: target_eid,
            index,
            corpse_respawn_at: target.respawn_at,
            corpse_template_id: target.template_id,
        };
        (item, source)
    };

    tracing::info!(
        entity_id, target_eid, index,
        design_id = ?removed_item.design_id,
        quantity = removed_item.quantity,
        "Player looted item"
    );

    if let Some(design_id) = removed_item.design_id {
        // Item — grant via GrantItem to base for persistence + onUpdateItem.
        // The item_containers cache holds the first non-storage container the
        // item lists; INV_Main (1) when it lists none. `loot` makes the base
        // answer a refusal, so the item comes back instead of being lost.
        let container_id = space_mgr
            .item_containers
            .get(&design_id)
            .copied()
            .unwrap_or(1);
        let sent = tx
            .send(CellToBaseMsg::GrantItem {
                entity_id,
                player_id,
                item_id: design_id,
                container_id,
                count: removed_item.quantity,
                // Loot pickup is not GM-sourced — no GM feedback line.
                notify_gm: false,
                loot: Some(source),
            })
            .await;
        if sent.is_err() {
            // The base never saw the grant, so the item is still ours to put
            // back.
            restore::return_unsent(entity_id, player_id, source, removed_item, space_mgr);
        }
    } else {
        // Cash (naquadah) — send GrantCash to base for persistence + onCashChanged
        let _ = tx
            .send(CellToBaseMsg::GrantCash {
                entity_id,
                player_id,
                amount: removed_item.quantity,
                // Loot pickup is not GM-sourced — no GM feedback line.
                gm_feedback_to: None,
            })
            .await;
    }

    // Check if loot is now empty
    let loot_empty = space_mgr
        .get_entity(target_eid)
        .is_none_or(|e| e.loot.is_empty());

    if loot_empty {
        // Clear ONLY the loot bit; preserve other interaction flags (quest tags,
        // mission interactions, etc.) so the corpse retains any content state set
        // pre-death. Mirrors python `Lootable.py:204`:
        //     ent.setInteractionType(ent.interactionType & ~INT_NormalLoot)
        let flags_to_send = if let Some(target) = space_mgr.get_entity_mut(target_eid) {
            target.interaction_type_flags &= !crate::cell::abilities::INT_NORMAL_LOOT;
            if target.interaction_type_flags == 0 {
                target.interaction_type = None;
            }
            target.interaction_type_flags
        } else {
            0
        };
        // Broadcast remaining flags to witnesses (not blanket 0).
        crate::cell::abilities::send_entity_method(
            target_eid,
            crate::mercury::method_idx::INTERACTION_TYPE,
            (flags_to_send as u64).to_le_bytes().to_vec(),
            tx,
            space_mgr,
        )
        .await;

        // Send the empty loot list to the player so the loot window closes.
        // Loot.lua hides the window when getLootCount()==0 inside onLootDisplay.
        // Without this, the window stays open displaying stale data and any
        // additional "Loot All" clicks fall through to lootItem with no
        // looting_entity set (we used to log the resulting warning storm).
        send_loot_display(entity_id, target_eid as i32, 0, tx, space_mgr).await;

        // Clear looting state on the player
        if let Some(player) = space_mgr.get_entity_mut(entity_id) {
            player.looting_entity = None;
        }

        tracing::debug!(target_eid, "NPC loot exhausted — cleared interaction");
    } else {
        // Send updated loot list to refresh the open window
        send_loot_display(entity_id, target_eid as i32, 0, tx, space_mgr).await;
    }
}

#[cfg(test)]
mod restore_tests;
#[cfg(test)]
mod tests;
