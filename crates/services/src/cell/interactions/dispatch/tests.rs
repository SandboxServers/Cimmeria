//! Tests for the interaction dispatch entry points (`handle_interact` and
//! `handle_initial_response`). They drive the public functions end-to-end
//! against a real `SpaceManager`, so they exercise both submodules.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::NpcInteractionType;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;

use super::{handle_initial_response, handle_interact};

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

/// The deprecated `NpcInteractionType::Trainer` arm in `handle_interact`
/// must log a `reason=deprecated_routing_arm` WARN and emit no wire
/// frame when an NPC carries the legacy tag but is NOT registered in
/// `template_trainer_lists`. The canonical trainer path runs upstream
/// in `cell_methods/player/interaction.rs::dispatch` and short-circuits
/// before reaching `handle_interact` whenever the NPC's template has
/// a `trainer_ability_list_id` — so by the time control reaches this
/// arm, there is no list to send and the only sane action is to log.
///
/// Pins the post-rewrite behavior: WARN level, `reason` field set to
/// the canonical key, no `onTrainerOpen` queued.
#[tokio::test]
async fn deprecated_trainer_arm_warns_without_emitting_wire_frame() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();

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

    // NPC inside MAX_INTERACT_DISTANCE, carrying the deprecated tag but
    // with NO `template_trainer_lists` registration — the canonical
    // upstream path would not have routed this NPC through
    // `try_open_trainer`, so the fall-through here is the only signal.
    let npc_id = mgr.allocate_npc_id();
    mgr.spawn_npc(npc_id, "Agnos", [2.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.template_id = Some(999); // intentionally NOT in template_trainer_lists
        npc.interaction_type = Some(NpcInteractionType::Trainer { archetype_id: 2 });
    }

    let (tx, mut rx) = mpsc::channel(16);
    let result = handle_interact(1, npc_id, &tx, &mut mgr).await;

    assert_eq!(result, None, "deprecated trainer arm must return None");

    // No wire frame queued — the arm only logs and drops.
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            assert_ne!(
                method_index,
                crate::mercury::method_idx::ON_TRAINER_OPEN,
                "deprecated trainer arm must NOT emit onTrainerOpen — that's \
                 exactly the double-frame bug the consolidation removed"
            );
        }
    }

    assert!(
        capture
            .find_event(
                Level::WARN,
                "deprecated NpcInteractionType::Trainer arm hit",
                "deprecated_routing_arm",
            )
            .is_some(),
        "deprecated trainer arm must WARN with reason=deprecated_routing_arm; \
         reverting to the old wording (or dropping the log) breaks operator \
         diagnosability when a content edit accidentally sets the Trainer tag"
    );
}
