//! NPC interaction handler for the CellService.
//!
//! Processes `interact(entityId)` calls from the client: validates distance,
//! looks up the NPC's interaction type, and sends the appropriate response
//! (dialog, vendor, trainer, or loot).
//!
//! Reference: `python/cell/SGWPlayer.py:1148-1203`

use tokio::sync::mpsc;

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
pub async fn handle_interact(
    entity_id: u32,
    target_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Validate player exists
    let player_pos = match space_mgr.get_entity(entity_id) {
        Some(e) => e.position,
        None => {
            tracing::warn!(entity_id, "interact: player entity not found");
            return;
        }
    };

    // Validate target exists and get interaction data
    let (target_pos, interaction_type, _npc_name) = match space_mgr.get_entity(target_entity_id) {
        Some(e) => (
            e.position,
            e.interaction_type.clone(),
            e.npc_name.clone().unwrap_or_default(),
        ),
        None => {
            tracing::debug!(entity_id, target_entity_id, "interact: target not found");
            return;
        }
    };

    // Distance check
    let dist = player_pos.distance_squared_to(&target_pos).sqrt();
    if dist > MAX_INTERACT_DISTANCE {
        tracing::debug!(entity_id, target_entity_id, dist, "interact: too far away");
        // TODO: Send feedback message "You are too far away to interact."
        return;
    }

    // Dispatch based on interaction type
    match interaction_type {
        Some(NpcInteractionType::Dialog { dialog_id }) => {
            send_dialog_display(entity_id, target_entity_id as i32, dialog_id, tx).await;
        }
        Some(NpcInteractionType::Vendor) => {
            send_store_open(entity_id, target_entity_id as i32, tx).await;
        }
        Some(NpcInteractionType::Trainer { archetype_id }) => {
            send_trainer_open(entity_id, target_entity_id as i32, archetype_id, tx).await;
        }
        Some(NpcInteractionType::Loot) => {
            send_loot_display(entity_id, target_entity_id as i32, tx).await;
        }
        None => {
            tracing::debug!(entity_id, target_entity_id, "interact: target has no interaction");
        }
    }
}

/// Send `onDialogDisplay` (flat index 105) to the player.
///
/// Wire: `entityId:i32, dialogId:i32, missionFlags:i32, isImmediate:u8, missionId:i32`.
async fn send_dialog_display(
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

/// Send `onStoreOpen` (flat index 109) to the player with empty inventory.
///
/// Wire: `entityId:i32, vendorType:i32, buyList:ARRAY, sellList:ARRAY,
///        buybackList:ARRAY, repairList:ARRAY, rechargeList:ARRAY`.
async fn send_store_open(
    player_id: u32,
    npc_entity_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let mut args = Vec::with_capacity(28);
    args.extend_from_slice(&npc_entity_id.to_le_bytes());  // EntityId
    args.extend_from_slice(&1i32.to_le_bytes());            // VendorType (1 = general)
    // 5 empty arrays (buy, sell, buyback, repair, recharge)
    for _ in 0..5 {
        args.extend_from_slice(&0u32.to_le_bytes());        // count = 0
    }

    tracing::debug!(player_id, npc_entity_id, "Sending onStoreOpen (empty)");
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: player_id,
        method_index: 109, // onStoreOpen
        args,
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
    let tree = crate::mercury_ext::archetype_ability_tree(archetype_id);
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

/// Send `onLootDisplay` (flat index 114) to the player with empty loot.
///
/// Wire: `entityId:i32, items:ARRAY<{itemId:i32, quantity:i16, index:i32, typeId:i32}>,
///        initial:i8`.
async fn send_loot_display(
    player_id: u32,
    npc_entity_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
) {
    let mut args = Vec::with_capacity(9);
    args.extend_from_slice(&npc_entity_id.to_le_bytes());  // EntityID
    args.extend_from_slice(&0u32.to_le_bytes());            // count = 0 (empty loot)
    args.push(1);                                           // Initial = 1

    tracing::debug!(player_id, npc_entity_id, "Sending onLootDisplay (empty)");
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id: player_id,
        method_index: 114, // onLootDisplay
        args,
    }).await;
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
    fn store_open_args_format() {
        let npc_id: i32 = 100_001;
        let mut args = Vec::new();
        args.extend_from_slice(&npc_id.to_le_bytes());
        args.extend_from_slice(&1i32.to_le_bytes()); // vendorType
        for _ in 0..5 {
            args.extend_from_slice(&0u32.to_le_bytes());
        }

        assert_eq!(args.len(), 28);
        assert_eq!(i32::from_le_bytes([args[0], args[1], args[2], args[3]]), npc_id);
        assert_eq!(i32::from_le_bytes([args[4], args[5], args[6], args[7]]), 1); // vendorType
        // All 5 array counts should be 0
        for i in 0..5 {
            let offset = 8 + i * 4;
            assert_eq!(u32::from_le_bytes([args[offset], args[offset+1], args[offset+2], args[offset+3]]), 0);
        }
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
        let spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
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
        let spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
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
        let spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();

        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3]).unwrap();
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3]).unwrap();
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            npc.interaction_type = Some(NpcInteractionType::Vendor);
        }

        let (tx, mut rx) = mpsc::channel(16);
        handle_interact(1, npc_id, &tx, &mut mgr).await;

        let msg = rx.try_recv().unwrap();
        match msg {
            CellToBaseMsg::EntityMethodCall { method_index, args, .. } => {
                assert_eq!(method_index, 109); // onStoreOpen
                assert_eq!(args.len(), 28);
            }
            _ => panic!("Expected EntityMethodCall"),
        }
    }

    #[tokio::test]
    async fn interact_no_response_for_hostile() {
        let mut mgr = super::super::space_manager::SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" charset="UTF-8"?>
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
