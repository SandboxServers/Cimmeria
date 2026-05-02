//! Top-level interaction routing — `handle_interact` and `handle_initial_response`.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::NpcInteractionType;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::dialog::send_dialog_display;
use super::loot::send_loot_display;
use super::trainer::send_trainer_open;
use super::vendor::send_store_open;

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
        crate::cell::content::fire_dialog_open(entity_id, player_id, dialog_id, engine, tx, space_mgr).await;
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

    #[tokio::test]
    async fn interact_requires_target_in_range() {
        // Create a space manager with entities
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
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
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
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
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
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
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
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
