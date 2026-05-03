//! Loot display + `lootItem` handler — `onLootDisplay` (flat 114) and the
//! `lootItem(index)` cell method.

use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

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
            tracing::warn!(entity_id, index, "lootItem: player not looting anything");
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

    // Find and remove the loot item from the NPC
    let removed_item = {
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
        match pos {
            Some(i) => target.loot.remove(i),
            None => {
                tracing::warn!(entity_id, target_eid, index, "lootItem: invalid index");
                return;
            }
        }
    };

    tracing::info!(
        entity_id, target_eid, index,
        design_id = ?removed_item.design_id,
        quantity = removed_item.quantity,
        "Player looted item"
    );

    if let Some(design_id) = removed_item.design_id {
        // Item — grant via GrantItem to base for persistence + onUpdateItem
        // Look up preferred container from item_containers cache, default to INV_Main (1)
        let container_id = space_mgr
            .item_containers
            .get(&design_id)
            .copied()
            .unwrap_or(1);
        let _ = tx
            .send(CellToBaseMsg::GrantItem {
                entity_id,
                player_id,
                item_id: design_id,
                container_id,
                count: removed_item.quantity,
            })
            .await;
    } else {
        // Cash (naquadah) — send GrantCash to base for persistence + onCashChanged
        let _ = tx
            .send(CellToBaseMsg::GrantCash {
                entity_id,
                player_id,
                amount: removed_item.quantity,
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
mod tests {
    #[test]
    fn loot_display_args_format() {
        let npc_id: i32 = 100_003;
        let mut args = Vec::new();
        args.extend_from_slice(&npc_id.to_le_bytes());
        args.extend_from_slice(&0u32.to_le_bytes()); // empty loot
        args.push(1); // initial

        assert_eq!(args.len(), 9);
        assert_eq!(u32::from_le_bytes([args[4], args[5], args[6], args[7]]), 0);
        assert_eq!(args[8], 1);
    }

    /// Regression for #106 + Copilot review on PR #108: validate looter
    /// has a player_id BEFORE removing the loot from the corpse. If the
    /// player_id check ever moves back below the mutation, the drop is
    /// gone and no grant fires.
    #[tokio::test]
    async fn loot_item_with_no_player_id_preserves_corpse_loot() {
        use super::super::super::space_manager::SpaceManager;
        use super::*;
        use cimmeria_entity::cell_entity::LootItem;
        use tokio::sync::mpsc;

        let mut mgr = SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();

        // Looter — leave player_id as the default (None).
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();

        // Seed loot on the corpse and mark the player as looting it.
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            npc.loot.push(LootItem {
                design_id: None,
                quantity: 50,
                index: 1,
            });
        }
        if let Some(p) = mgr.get_entity_mut(1) {
            p.looting_entity = Some(npc_id);
            assert!(
                p.player_id.is_none(),
                "default CellEntity must have no player_id for this test"
            );
        }

        let (tx, mut rx) = mpsc::channel(16);
        handle_loot_item(1, 1, &tx, &mut mgr).await;

        // Corpse loot must still be present (the bug removed it before the
        // player_id check, leaving the player with nothing AND the corpse
        // empty).
        let loot_after = mgr.get_entity(npc_id).map(|e| e.loot.len()).unwrap_or(0);
        assert_eq!(
            loot_after, 1,
            "corpse loot must be intact when looter has no player_id"
        );

        // No GrantCash / GrantItem must have been queued.
        while let Ok(msg) = rx.try_recv() {
            match msg {
                CellToBaseMsg::GrantCash { .. } | CellToBaseMsg::GrantItem { .. } => {
                    panic!("no grant message should fire when looter has no player_id");
                }
                _ => {}
            }
        }
    }
}
