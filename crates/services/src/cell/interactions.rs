//! NPC interaction handler for the CellService.
//!
//! Processes `interact(entityId)` calls from the client: validates distance,
//! looks up the NPC's interaction type, and sends the appropriate response
//! (dialog, vendor, trainer, or loot).
//!
//! Reference: `python/cell/SGWPlayer.py:1148-1203`

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::NpcInteractionType;

use super::messages::CellToBaseMsg;
use super::space_manager::SpaceManager;

/// Maximum distance for NPC interaction (world units).
/// From `python/common/Constants.py: MAX_INTERACT_DISTANCE = 5`.
const MAX_INTERACT_DISTANCE: f32 = 5.0;

/// Handle `interact(targetEntityId)` cell method call.
///
/// Flow:
/// 1. Validate player and target entities exist
/// 2. Check distance (max 5.0 units)
/// 3. Look up target's interaction type
/// 4. Send appropriate client method response
///
/// Returns `Some(dialog_id)` if a dialog was opened (for content engine events).
pub async fn handle_interact(
    entity_id: u32,
    target_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<i32> {
    // Validate player exists
    let player_pos = match space_mgr.get_entity(entity_id) {
        Some(e) => e.position,
        None => {
            tracing::warn!(entity_id, "interact: player entity not found");
            return None;
        }
    };

    // Validate target exists and get interaction data
    let (target_pos, interaction_type, npc_name, target_template_id) = match space_mgr.get_entity(target_entity_id) {
        Some(e) => (
            e.position,
            e.interaction_type.clone(),
            e.npc_name.clone().unwrap_or_default(),
            e.template_id,
        ),
        None => {
            tracing::info!(entity_id, target_entity_id, "interact: target entity not found");
            return None;
        }
    };

    tracing::info!(
        entity_id, target_entity_id, %npc_name,
        ?interaction_type, ?target_template_id,
        "interact: target resolved"
    );

    // Distance check
    let dist = player_pos.distance_squared_to(&target_pos).sqrt();
    if dist > MAX_INTERACT_DISTANCE {
        tracing::info!(entity_id, target_entity_id, dist, "interact: too far away");
        return None;
    }

    // Check per-player available interactions (from add_dialog_set content actions).
    // These take priority over static interaction_type.
    if let Some(tmpl_id) = target_template_id {
        let dialog_id = space_mgr.get_entity(entity_id)
            .and_then(|p| p.available_interactions.get(&tmpl_id))
            .and_then(|entries| entries.first())
            .map(|&(_, dialog_id, _)| dialog_id);

        if let Some(dialog_id) = dialog_id {
            tracing::info!(entity_id, target_entity_id, tmpl_id, dialog_id, "interact: per-player dialog set → onDialogDisplay");
            send_dialog_display(entity_id, target_entity_id as i32, dialog_id, tx).await;
            return Some(dialog_id);
        } else {
            tracing::info!(entity_id, tmpl_id, "interact: no per-player interactions for template");
        }
    }

    // Dispatch based on static interaction type
    match interaction_type {
        Some(NpcInteractionType::Dialog { dialog_id }) => {
            tracing::info!(entity_id, target_entity_id, dialog_id, "interact: static dialog → onDialogDisplay");
            send_dialog_display(entity_id, target_entity_id as i32, dialog_id, tx).await;
            Some(dialog_id)
        }
        Some(NpcInteractionType::Vendor) => {
            tracing::info!(entity_id, target_entity_id, "interact: vendor → OpenVendorStore");
            send_store_open(entity_id, target_entity_id as u32, tx, space_mgr).await;
            None
        }
        Some(NpcInteractionType::Trainer { archetype_id }) => {
            tracing::info!(entity_id, target_entity_id, archetype_id, "interact: trainer → onTrainerOpen");
            send_trainer_open(entity_id, target_entity_id as i32, archetype_id, tx).await;
            None
        }
        Some(NpcInteractionType::Loot) => {
            tracing::info!(entity_id, target_entity_id, "interact: loot → onLootDisplay");
            // Track which entity the player is looting (for lootItem calls)
            if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                player.looting_entity = Some(target_entity_id);
            }
            send_loot_display(entity_id, target_entity_id as i32, 1, tx, space_mgr).await;
            None
        }
        None => {
            tracing::info!(entity_id, target_entity_id, "interact: target has no static interaction type");
            None
        }
    }
}

/// Send `onDialogDisplay` (flat index 105) to the player.
///
/// Wire: `entityId:i32, dialogId:i32, missionFlags:i32, isImmediate:u8, missionId:i32`.
pub async fn send_dialog_display(
    player_id: u32,
    npc_entity_id: i32,
    dialog_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let mut args = Vec::with_capacity(17);
    args.extend_from_slice(&npc_entity_id.to_le_bytes());  // EntityId
    args.extend_from_slice(&dialog_id.to_le_bytes());       // DialogID
    args.extend_from_slice(&0i32.to_le_bytes());            // MissionFlags
    args.push(1);                                           // IsImmediate
    args.extend_from_slice(&0i32.to_le_bytes());            // aMissionId

    tracing::debug!(player_id, npc_entity_id, dialog_id, "Sending onDialogDisplay");
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: player_id,
        method_index: 105, // onDialogDisplay
        args,
    }).await;
}

/// Open a vendor store for a player by requesting store data from base.
///
/// Sets the vendor_entity on the player entity and sends OpenVendorStore
/// to BaseApp, which will load the full vendor store data and send it to the client.
async fn send_store_open(
    player_id: u32,
    vendor_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Store vendor entity ID on player for reference during vendor operations
    if let Some(player) = space_mgr.get_entity_mut(player_id) {
        player.vendor_entity = Some(vendor_entity_id);
    }

    // Get vendor template ID from the vendor entity
    let vendor_template_id = space_mgr.get_entity(vendor_entity_id)
        .and_then(|e| e.template_id);

    // Get player_id for base app message
    let player_db_id = match space_mgr.get_entity(player_id).and_then(|e| e.player_id) {
        Some(id) => id,
        None => {
            tracing::warn!(player_id, vendor_entity_id, "send_store_open: missing player_id; aborting vendor open");
            return;
        }
    };

    let vendor_entity_id_i32 = match i32::try_from(vendor_entity_id) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!(player_id, vendor_entity_id, "send_store_open: vendor entity id exceeds i32; aborting");
            return;
        }
    };

    tracing::info!(player_id, vendor_entity_id, ?vendor_template_id, "Opening vendor store");
    let _ = tx.send(CellToBaseMsg::OpenVendorStore {
        entity_id: player_id,
        player_id: player_db_id,
        vendor_entity_id: vendor_entity_id_i32,
        vendor_template_id,
    }).await;
}

/// Send `onTrainerOpen` (flat index 113) to the player.
///
/// Wire: `trainerEntityId:i32, abilities:ARRAY<{abilityId:i32, trainable:u8}>,
///        costToRespec:i32`.
async fn send_trainer_open(
    player_id: u32,
    npc_entity_id: i32,
    archetype_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    // Get the ability tree for this archetype so we know what to offer
    let tree = crate::mercury::archetype_ability_tree(archetype_id);
    let all_abilities: Vec<i32> = tree.trees.iter().flatten().copied().collect();

    // TODO: Check which abilities the player already knows and mark as trainable
    // For now, mark all as trainable (1)
    let count = all_abilities.len() as u32;
    let mut args = Vec::with_capacity(8 + all_abilities.len() * 5);
    args.extend_from_slice(&npc_entity_id.to_le_bytes());  // TrainerID
    args.extend_from_slice(&count.to_le_bytes());           // ability count
    for ability_id in &all_abilities {
        args.extend_from_slice(&ability_id.to_le_bytes());  // abilityID
        args.push(1);                                       // trainable = true
    }
    args.extend_from_slice(&1000i32.to_le_bytes());         // CostToRespec

    tracing::debug!(player_id, npc_entity_id, archetype_id, count, "Sending onTrainerOpen");
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: player_id,
        method_index: 113, // onTrainerOpen
        args,
    }).await;
}

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
async fn send_loot_display(
    player_id: u32,
    npc_entity_id: i32,
    initial: u8,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &SpaceManager,
) {
    // Read loot items from the target entity
    let loot_items: Vec<(Option<i32>, i32, i32)> = space_mgr.get_entity(npc_entity_id as u32)
        .map(|e| e.loot.iter().map(|li| (li.design_id, li.quantity, li.index)).collect())
        .unwrap_or_default();

    let count = loot_items.len() as u32;
    // Per item: 4 (itemID) + 2 (quantity i16) + 4 (index) + 4 (typeID) = 14 bytes
    let mut args = Vec::with_capacity(4 + 4 + loot_items.len() * 14 + 1);
    args.extend_from_slice(&npc_entity_id.to_le_bytes());  // EntityID
    args.extend_from_slice(&count.to_le_bytes());           // ARRAY count

    for (design_id, quantity, index) in &loot_items {
        let item_id = design_id.unwrap_or(0); // 0 = naquadah (cash)
        let type_id = if design_id.is_some() { 1i32 } else { 2i32 }; // LOOT_Item=1, LOOT_Cash=2
        args.extend_from_slice(&item_id.to_le_bytes());          // itemID: INT32
        args.extend_from_slice(&(*quantity as i16).to_le_bytes()); // quantity: INT16
        args.extend_from_slice(&index.to_le_bytes());             // index: INT32
        args.extend_from_slice(&type_id.to_le_bytes());           // typeID: INT32
    }

    args.push(initial);

    tracing::debug!(
        player_id, npc_entity_id, count, initial,
        "Sending onLootDisplay"
    );
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: player_id,
        method_index: crate::mercury::method_idx::ON_LOOT_DISPLAY,
        args,
    }).await;
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
    let looting_target = space_mgr.get_entity(entity_id)
        .and_then(|e| e.looting_entity);

    let target_eid = match looting_target {
        Some(eid) => eid,
        None => {
            tracing::warn!(entity_id, index, "lootItem: player not looting anything");
            return;
        }
    };

    // Find and remove the loot item from the NPC
    let removed_item = {
        let target = match space_mgr.get_entity_mut(target_eid) {
            Some(e) => e,
            None => {
                tracing::warn!(entity_id, target_eid, index, "lootItem: target entity not found");
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

    // Grant the loot to the player via BaseApp persistence handlers
    let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
        Some(id) => id,
        None => {
            tracing::warn!(
                entity_id, target_eid,
                "lootItem: dropping loot grant — looter has no player_id (would otherwise misroute to player_id=0)"
            );
            return;
        }
    };

    if removed_item.design_id.is_none() {
        // Cash (naquadah) — send GrantCash to base for persistence + onCashChanged
        let _ = tx.send(CellToBaseMsg::GrantCash {
            entity_id,
            player_id,
            amount: removed_item.quantity,
        }).await;
    } else {
        // Item — grant via GrantItem to base for persistence + onUpdateItem
        let design_id = removed_item.design_id.unwrap();
        // Look up preferred container from item_containers cache, default to INV_Main (1)
        let container_id = space_mgr.item_containers.get(&design_id).copied().unwrap_or(1);
        let _ = tx.send(CellToBaseMsg::GrantItem {
            entity_id,
            player_id,
            item_id: design_id,
            container_id,
            count: removed_item.quantity,
        }).await;
    }

    // Check if loot is now empty
    let loot_empty = space_mgr.get_entity(target_eid)
        .map_or(true, |e| e.loot.is_empty());

    if loot_empty {
        // Clear loot interaction on the NPC
        if let Some(target) = space_mgr.get_entity_mut(target_eid) {
            target.interaction_type_flags = 0;
            target.interaction_type = None;
        }
        // Broadcast InteractionType(0) to witnesses
        super::abilities::send_entity_method(
            target_eid, crate::mercury::method_idx::INTERACTION_TYPE,
            0u64.to_le_bytes().to_vec(),
            tx, space_mgr,
        ).await;

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

/// Handle initial interaction response: find a matching dialog for the given
/// `interaction_set_map_id` in the player's available interactions and display it.
///
/// Called when the client sends an `initialResponse` cell method, typically
/// after clicking an NPC whose InteractionType was set by a content chain.
pub async fn handle_initial_response(
    entity_id: u32,
    interaction_set_map_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Search all per-player available_interactions for a matching dialog_set_map_id
    let dialog_id = space_mgr.get_entity(entity_id)
        .and_then(|p| {
            for entries in p.available_interactions.values() {
                for &(dsm_id, dialog_id, _) in entries {
                    if dsm_id == interaction_set_map_id {
                        return Some(dialog_id);
                    }
                }
            }
            None
        });

    let player_id = space_mgr.get_entity(entity_id)
        .and_then(|e| e.player_id)
        .unwrap_or(0);

    if let Some(dialog_id) = dialog_id {
        tracing::info!(
            entity_id, interaction_set_map_id, dialog_id,
            "handle_initial_response: found dialog, sending onDialogDisplay"
        );
        send_dialog_display(entity_id, entity_id as i32, dialog_id, tx).await;
        super::content::fire_dialog_open(entity_id, player_id, dialog_id, engine, tx, space_mgr).await;
    } else {
        tracing::debug!(
            entity_id, interaction_set_map_id,
            "handle_initial_response: no matching dialog_set_map_id in available_interactions"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_display_args_format() {
        let mut args = Vec::new();
        let npc_id: i32 = 100_000;
        let dialog_id: i32 = 42;
        args.extend_from_slice(&npc_id.to_le_bytes());
        args.extend_from_slice(&dialog_id.to_le_bytes());
        args.extend_from_slice(&0i32.to_le_bytes()); // missionFlags
        args.push(1); // isImmediate
        args.extend_from_slice(&0i32.to_le_bytes()); // missionId

        assert_eq!(args.len(), 17);
        assert_eq!(i32::from_le_bytes([args[0], args[1], args[2], args[3]]), npc_id);
        assert_eq!(i32::from_le_bytes([args[4], args[5], args[6], args[7]]), dialog_id);
        assert_eq!(args[12], 1); // isImmediate
    }

    #[test]
    fn trainer_open_args_format() {
        let npc_id: i32 = 100_002;
        let abilities = vec![597i32, 603, 604];
        let count = abilities.len() as u32;

        let mut args = Vec::new();
        args.extend_from_slice(&npc_id.to_le_bytes());
        args.extend_from_slice(&count.to_le_bytes());
        for &ab in &abilities {
            args.extend_from_slice(&ab.to_le_bytes());
            args.push(1); // trainable
        }
        args.extend_from_slice(&1000i32.to_le_bytes());

        // 4 (npc) + 4 (count) + 3*(4+1) + 4 (respec) = 27
        assert_eq!(args.len(), 27);
        assert_eq!(u32::from_le_bytes([args[4], args[5], args[6], args[7]]), 3);
        // First ability
        assert_eq!(i32::from_le_bytes([args[8], args[9], args[10], args[11]]), 597);
        assert_eq!(args[12], 1); // trainable
    }

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

    #[tokio::test]
    async fn interact_requires_target_in_range() {
        // Create a space manager with entities
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();

        // Player at origin
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3]).unwrap();
        // NPC far away (distance = 100, > MAX_INTERACT_DISTANCE)
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [100.0, 0.0, 0.0], [0.0; 3]).unwrap();
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            npc.interaction_type = Some(NpcInteractionType::Dialog { dialog_id: 1 });
        }

        let (tx, mut rx) = mpsc::channel(16);
        handle_interact(1, npc_id, &tx, &mut mgr).await;

        // Should NOT send any response (too far)
        assert!(rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn interact_sends_dialog_when_nearby() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();

        // Player at (1,0,1)
        mgr.create_entity(1, "Agnos", [1.0, 0.0, 1.0], [0.0; 3]).unwrap();
        // NPC at (3,0,1) — distance = 2.0, within range
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [3.0, 0.0, 1.0], [0.0; 3]).unwrap();
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            npc.interaction_type = Some(NpcInteractionType::Dialog { dialog_id: 42 });
        }

        let (tx, mut rx) = mpsc::channel(16);
        handle_interact(1, npc_id, &tx, &mut mgr).await;

        // Should receive onDialogDisplay
        let msg = rx.try_recv().unwrap();
        match msg {
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, args } => {
                assert_eq!(entity_id, 1); // sent to player
                assert_eq!(method_index, 105); // onDialogDisplay
                assert_eq!(args.len(), 17);
                let dialog_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                assert_eq!(dialog_id, 42);
            }
            _ => panic!("Expected EntityMethodCall"),
        }
    }

    #[tokio::test]
    async fn interact_sends_vendor_when_nearby() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();

        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(42);
        }
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3]).unwrap();
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            npc.interaction_type = Some(NpcInteractionType::Vendor);
        }

        let (tx, mut rx) = mpsc::channel(16);
        handle_interact(1, npc_id, &tx, &mut mgr).await;

        let msg = rx.try_recv().unwrap();
        match msg {
            CellToBaseMsg::OpenVendorStore { entity_id, player_id, vendor_entity_id, .. } => {
                assert_eq!(entity_id, 1);
                assert_eq!(player_id, 42);
                assert_eq!(vendor_entity_id as u32, npc_id);
            }
            other => panic!("Expected OpenVendorStore, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn interact_no_response_for_hostile() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();

        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3]).unwrap();
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3]).unwrap();
        // interaction_type = None (hostile)

        let (tx, mut rx) = mpsc::channel(16);
        handle_interact(1, npc_id, &tx, &mut mgr).await;

        // No response for hostile NPCs
        assert!(rx.try_recv().is_err());
    }
}
