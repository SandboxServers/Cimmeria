//! Top-level interaction routing — `handle_interact` and `handle_initial_response`.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::NpcInteractionType;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::dialog::send_dialog_display;
use super::loot::send_loot_display;
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
#[tracing::instrument(
    name = "interaction.interact",
    level = "info",
    skip_all,
    fields(entity_id, target_entity_id, space_id = tracing::field::Empty)
)]
pub async fn handle_interact(
    entity_id: u32,
    target_entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) -> Option<i32> {
    // Validate player exists
    let (player_pos, player_space_id) = match space_mgr.get_entity(entity_id) {
        Some(e) => (e.position, e.space_id.0),
        None => {
            tracing::warn!(entity_id, "interact: player entity not found");
            return None;
        }
    };
    tracing::Span::current().record("space_id", player_space_id);

    // Validate target exists and get interaction data
    let (target_pos, interaction_type, npc_name, target_template_id) =
        match space_mgr.get_entity(target_entity_id) {
            Some(e) => (
                e.position,
                e.interaction_type.clone(),
                e.npc_name.clone().unwrap_or_default(),
                e.template_id,
            ),
            None => {
                tracing::info!(
                    entity_id,
                    target_entity_id,
                    "interact: target entity not found"
                );
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

    // Pin the interaction target on the player. Downstream dispatchers
    // that don't get the NPC entity ID on their own wire frame
    // (`handle_initial_response`, content-engine `display_dialog`
    // fired from a follow-up trigger like `OnDialogChoice`) read this
    // to fill the wire `EntityId` field of `onDialogDisplay`. The
    // dialog portrait widget binds its actor by that ID via
    // `LookupEntityListenerEntry`, so passing the player's ID there
    // (the prior bug) made the dialog speak as the player. Mirrors
    // python's `SGWPlayer.lastInteractionTarget` write in `interact()`.
    if let Some(player) = space_mgr.get_entity_mut(entity_id) {
        player.last_interaction_target = Some(target_entity_id);
    }

    // Check per-player available interactions (from add_dialog_set content actions).
    // These take priority over static interaction_type.
    if let Some(tmpl_id) = target_template_id {
        let dialog_id = space_mgr
            .get_entity(entity_id)
            .and_then(|p| p.available_interactions.get(&tmpl_id))
            .and_then(|entries| entries.first())
            .map(|&(_, dialog_id, _)| dialog_id);

        if let Some(dialog_id) = dialog_id {
            tracing::info!(
                entity_id,
                target_entity_id,
                tmpl_id,
                dialog_id,
                "interact: per-player dialog set → onDialogDisplay"
            );
            send_dialog_display(entity_id, target_entity_id as i32, dialog_id, tx).await;
            return Some(dialog_id);
        } else {
            tracing::info!(
                entity_id,
                tmpl_id,
                "interact: no per-player interactions for template"
            );
        }
    }

    // Dispatch based on static interaction type
    match interaction_type {
        Some(NpcInteractionType::Dialog { dialog_id }) => {
            tracing::info!(
                entity_id,
                target_entity_id,
                dialog_id,
                "interact: static dialog → onDialogDisplay"
            );
            send_dialog_display(entity_id, target_entity_id as i32, dialog_id, tx).await;
            Some(dialog_id)
        }
        Some(NpcInteractionType::Vendor) => {
            tracing::info!(
                entity_id,
                target_entity_id,
                "interact: vendor → OpenVendorStore"
            );
            send_store_open(entity_id, target_entity_id, tx, space_mgr).await;
            None
        }
        Some(NpcInteractionType::Trainer { archetype_id }) => {
            // Deprecated routing arm. The canonical trainer path is the
            // `template_trainer_lists` lookup in
            // `cell_methods/player/interaction.rs::dispatch`, which calls
            // `interactions::try_open_trainer` BEFORE falling through to
            // `handle_interact`. By the time control reaches this arm,
            // `try_open_trainer` has already either opened the trainer
            // (returning `true`, in which case we don't get here) or
            // determined the NPC isn't a trainer (no `trainer_ability_list_id`
            // on the template). If the NPC has the `Trainer` tag but no
            // template-registered list, the only sane action is to log
            // and drop — emitting `onTrainerOpen` from here would bypass
            // the per-archetype offering, already-known, level, and
            // prereq filters that the canonical handler enforces.
            tracing::warn!(
                target: "abilities",
                event = "trainer_deprecated_routing_arm",
                entity_id,
                target_entity_id,
                archetype_id,
                reason = "deprecated_routing_arm",
                "interact: deprecated NpcInteractionType::Trainer arm hit — \
                 template_trainer_lists is the canonical path; set \
                 entity_templates.trainer_ability_list_id on this NPC's \
                 template instead of using the Trainer interaction tag"
            );
            None
        }
        Some(NpcInteractionType::Loot) => {
            tracing::info!(
                entity_id,
                target_entity_id,
                "interact: loot → onLootDisplay"
            );
            // Track which entity the player is looting (for lootItem calls)
            if let Some(player) = space_mgr.get_entity_mut(entity_id) {
                player.looting_entity = Some(target_entity_id);
            }
            send_loot_display(entity_id, target_entity_id as i32, 1, tx, space_mgr).await;
            None
        }
        None => {
            tracing::info!(
                entity_id,
                target_entity_id,
                "interact: target has no static interaction type"
            );
            None
        }
    }
}

/// Handle initial interaction response: find a matching dialog for the given
/// `interaction_set_map_id` in the player's available interactions and display it.
///
/// Called when the client sends an `initialResponse` cell method, typically
/// after clicking an NPC whose InteractionType was set by a content chain.
#[tracing::instrument(
    name = "dialog.initial_response",
    level = "info",
    skip_all,
    fields(entity_id, interaction_set_map_id)
)]
pub async fn handle_initial_response(
    entity_id: u32,
    interaction_set_map_id: i32,
    engine: &ChainEngine,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    // Search all per-player available_interactions for a matching dialog_set_map_id
    let dialog_id = space_mgr.get_entity(entity_id).and_then(|p| {
        for entries in p.available_interactions.values() {
            for &(dsm_id, dialog_id, _) in entries {
                if dsm_id == interaction_set_map_id {
                    return Some(dialog_id);
                }
            }
        }
        None
    });

    if let Some(dialog_id) = dialog_id {
        // Resolve player_id only after we know we have a dialog to fire.
        // Falling back to 0 here would attribute the resulting content-engine
        // side effects (mission progress, chain triggers) to a non-existent
        // player. Mirrors the existing protection in `send_store_open`.
        let player_id = match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
            Some(id) => id,
            None => {
                tracing::warn!(
                    entity_id,
                    interaction_set_map_id,
                    dialog_id,
                    "handle_initial_response: missing player_id; aborting dialog open"
                );
                return;
            }
        };
        // Wire `EntityId` of `onDialogDisplay` must be the NPC the player
        // is talking to — the client passes it through
        // `LookupEntityListenerEntry` to bind the dialog portrait actor.
        // `last_interaction_target` was set by the preceding `handle_interact`
        // (the client only sends `interactionSetMapId` here, so the NPC ID
        // has to come from the per-player pin). Falling back to `entity_id`
        // (the player) makes the dialog speak as the player and blanks the
        // portrait — same shape as python's `SGWPlayer.initialResponse`
        // returning early when `lastInteractionTarget` is unset.
        let npc_entity_id = match space_mgr
            .get_entity(entity_id)
            .and_then(|p| p.last_interaction_target)
        {
            Some(id) => id as i32,
            None => {
                tracing::warn!(
                    entity_id,
                    interaction_set_map_id,
                    dialog_id,
                    "handle_initial_response: no last_interaction_target on player; \
                     aborting dialog open -- portrait would render blank against the player"
                );
                return;
            }
        };
        tracing::info!(
            entity_id,
            interaction_set_map_id,
            dialog_id,
            npc_entity_id,
            "handle_initial_response: found dialog, sending onDialogDisplay"
        );
        send_dialog_display(entity_id, npc_entity_id, dialog_id, tx).await;
        crate::cell::content::fire_dialog_open(
            entity_id, player_id, dialog_id, engine, tx, space_mgr,
        )
        .await;
    } else {
        tracing::debug!(
            entity_id,
            interaction_set_map_id,
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
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // NPC far away (distance = 100, > MAX_INTERACT_DISTANCE)
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [100.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
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
        mgr.create_entity(1, "Agnos", [1.0, 0.0, 1.0], [0.0; 3])
            .unwrap();
        // NPC at (3,0,1) — distance = 2.0, within range
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [3.0, 0.0, 1.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            npc.interaction_type = Some(NpcInteractionType::Dialog { dialog_id: 42 });
        }

        let (tx, mut rx) = mpsc::channel(16);
        handle_interact(1, npc_id, &tx, &mut mgr).await;

        // Should receive onDialogDisplay
        let msg = rx.try_recv().unwrap();
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } => {
                assert_eq!(entity_id, 1); // sent to player
                assert_eq!(method_index, crate::mercury::method_idx::ON_DIALOG_DISPLAY);
                assert_eq!(args.len(), 17);
                // Wire `EntityId` (args[0..4]) MUST be the NPC's entity id, not
                // the player's. The client uses it as the key into
                // `LookupEntityListenerEntry` to bind the dialog portrait actor;
                // passing the player here makes the dialog speak as the player
                // and blanks the portrait. See
                // `docs/reverse-engineering/findings/dialog-portrait-lookup.md`.
                let wire_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(
                    wire_entity_id as u32, npc_id,
                    "onDialogDisplay wire EntityId must be the NPC id (got {wire_entity_id}, expected {npc_id})"
                );
                let dialog_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                assert_eq!(dialog_id, 42);
            }
            _ => panic!("Expected EntityMethodCall"),
        }
    }

    /// `handle_interact` must pin `last_interaction_target` on the player so
    /// the subsequent `handle_initial_response` (which only gets
    /// `interactionSetMapId` on the wire) can recover the NPC entity id to
    /// stamp into `onDialogDisplay`. Without this pin, the chain-fired
    /// dialog path bound the player as the speaker.
    #[tokio::test]
    async fn interact_pins_last_interaction_target_on_player() {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            npc.interaction_type = Some(NpcInteractionType::Dialog { dialog_id: 42 });
        }

        let (tx, _rx) = mpsc::channel(16);
        handle_interact(1, npc_id, &tx, &mut mgr).await;

        assert_eq!(
            mgr.get_entity(1).and_then(|p| p.last_interaction_target),
            Some(npc_id),
            "handle_interact must pin last_interaction_target = NPC id on the player"
        );
    }

    /// Out-of-range interact must NOT pin `last_interaction_target` — a
    /// stale pin from a too-far click would then misroute a subsequent
    /// `initialResponse` (or chain-fired dialog) at the wrong NPC.
    #[tokio::test]
    async fn out_of_range_interact_does_not_pin_target() {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [100.0, 0.0, 0.0], [0.0; 3])
            .unwrap();

        let (tx, _rx) = mpsc::channel(16);
        handle_interact(1, npc_id, &tx, &mut mgr).await;

        assert_eq!(
            mgr.get_entity(1).and_then(|p| p.last_interaction_target),
            None,
            "out-of-range interact must leave last_interaction_target untouched"
        );
    }

    /// `handle_initial_response` must stamp the NPC entity id (from
    /// `last_interaction_target`) into the wire `EntityId` field of
    /// `onDialogDisplay`, not the player's id. Regression for the bug
    /// where the dialog opened with the player bound as the speaker
    /// actor (blank portrait, player name on every screen).
    #[tokio::test]
    async fn initial_response_uses_last_interaction_target_for_wire_entity_id() {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();

        const TEMPLATE_ID: i32 = 7;
        const DIALOG_SET_MAP_ID: i32 = 99;
        const DIALOG_ID: i32 = 4001;
        const NPC_ID: u32 = 0x1234;
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(42);
            p.last_interaction_target = Some(NPC_ID);
            p.available_interactions
                .insert(TEMPLATE_ID, vec![(DIALOG_SET_MAP_ID, DIALOG_ID, 0)]);
        }

        let (tx, mut rx) = mpsc::channel(16);
        let engine = ChainEngine::new();
        handle_initial_response(1, DIALOG_SET_MAP_ID, &engine, &tx, &mut mgr).await;

        let msg = rx.try_recv().expect("must emit onDialogDisplay");
        match msg {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => {
                assert_eq!(method_index, crate::mercury::method_idx::ON_DIALOG_DISPLAY);
                let wire_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_eq!(
                    wire_entity_id as u32, NPC_ID,
                    "initial_response must stamp last_interaction_target as wire EntityId, \
                     not the player's id (got {wire_entity_id}, expected {NPC_ID})"
                );
                let dialog_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                assert_eq!(dialog_id, DIALOG_ID);
            }
            other => panic!("expected EntityMethodCall, got {other:?}"),
        }
    }

    /// `handle_initial_response` must abort (not fire onDialogDisplay) when
    /// `last_interaction_target` is unset. Falling back to the player here
    /// is exactly the prior bug shape.
    #[tokio::test]
    async fn initial_response_aborts_when_last_interaction_target_missing() {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();

        const TEMPLATE_ID: i32 = 7;
        const DIALOG_SET_MAP_ID: i32 = 99;
        const DIALOG_ID: i32 = 4001;
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(42);
            // last_interaction_target intentionally left as None.
            p.available_interactions
                .insert(TEMPLATE_ID, vec![(DIALOG_SET_MAP_ID, DIALOG_ID, 0)]);
        }

        let (tx, mut rx) = mpsc::channel(16);
        let engine = ChainEngine::new();
        handle_initial_response(1, DIALOG_SET_MAP_ID, &engine, &tx, &mut mgr).await;

        assert!(
            rx.try_recv().is_err(),
            "initial_response must NOT emit onDialogDisplay when last_interaction_target \
             is unset -- falling back to the player blanks the portrait"
        );
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

        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.player_id = Some(42);
        }
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(npc) = mgr.get_entity_mut(npc_id) {
            npc.interaction_type = Some(NpcInteractionType::Vendor);
        }

        let (tx, mut rx) = mpsc::channel(16);
        handle_interact(1, npc_id, &tx, &mut mgr).await;

        let msg = rx.try_recv().unwrap();
        match msg {
            CellToBaseMsg::OpenVendorStore {
                entity_id,
                player_id,
                vendor_entity_id,
                ..
            } => {
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

        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        let npc_id = mgr.allocate_npc_id();
        mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // interaction_type = None (hostile)

        let (tx, mut rx) = mpsc::channel(16);
        handle_interact(1, npc_id, &tx, &mut mgr).await;

        // No response for hostile NPCs
        assert!(rx.try_recv().is_err());
    }

    /// Regression for #105 + Copilot review on PR #108: handle_initial_response
    /// must NOT fall back to `player_id = 0` when forwarding into the content
    /// engine. If the player_id is missing, the warn-and-return path must
    /// fire and no onDialogDisplay packet should be queued.
    #[tokio::test]
    async fn initial_response_skips_when_player_id_missing() {
        let mut mgr = crate::cell::space_manager::SpaceManager::new(1);
        let spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
        let cell_spaces_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces><Space WorldName="Agnos" /></Spaces>"#;
        mgr.parse_spaces_xml(spaces_xml).unwrap();
        mgr.create_startup_spaces(cell_spaces_xml).unwrap();

        // Create a player but DO NOT set player_id — default is None.
        mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
            .unwrap();

        // Seed an available_interactions entry so dialog lookup succeeds —
        // the test only fails if the function bails on missing player_id
        // BEFORE firing the dialog. Wire format of the value tuple is
        // (dialog_set_map_id, dialog_id, _).
        const TEMPLATE_ID: i32 = 7;
        const DIALOG_SET_MAP_ID: i32 = 99;
        const DIALOG_ID: i32 = 42;
        if let Some(p) = mgr.get_entity_mut(1) {
            assert!(
                p.player_id.is_none(),
                "default CellEntity must have no player_id for this test"
            );
            p.available_interactions
                .insert(TEMPLATE_ID, vec![(DIALOG_SET_MAP_ID, DIALOG_ID, 0)]);
        }

        let (tx, mut rx) = mpsc::channel(16);
        let engine = ChainEngine::new();
        handle_initial_response(1, DIALOG_SET_MAP_ID, &engine, &tx, &mut mgr).await;

        // Nothing must have been queued — the function should have warn-and-
        // returned before send_dialog_display or fire_dialog_open ran.
        assert!(
            rx.try_recv().is_err(),
            "initial_response must not send onDialogDisplay when player_id is missing"
        );
    }
}
