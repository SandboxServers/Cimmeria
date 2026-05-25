//! Tests for `BaseToCellMsg::RequestEntityUpdate` — issue #289 / audit N3.
//!
//! Bug shape these guards prevent: client sends `requestEntityUpdate` for an
//! entity it's missing (because `createEntity` was dropped on the wire past
//! the 20-retry cap), and the server's handler either (a) silently drops the
//! request, leaving the NPC permanently invisible, or (b) re-emits state for
//! entities the requesting witness shouldn't be able to see.

use super::*;
use cimmeria_common::EntityId;

/// Witness has target entity in AoI → handler emits exactly one EnteredAoI
/// for that entity with the entity's current state (class_id, position,
/// level, npc_data). This is the recovery path the issue describes.
#[tokio::test]
async fn request_entity_update_reemits_entered_aoi_for_witnessed_entity() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    // Witness (player) at (0,0,0)
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(w) = mgr.get_entity_mut(1) {
        w.is_player = true;
        w.player_id = Some(100);
        // Simulate the AoI tick having already added entity 42 to this witness.
        w.witnesses.insert(EntityId(42));
    }
    // Target NPC entity 42 at (5,0,0) with concrete values that should surface
    // in the re-emitted EnteredAoI.
    mgr.create_entity(42, "Castle_CellBlock", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(t) = mgr.get_entity_mut(42) {
        t.is_player = false;
        t.class_id = 0x04;
        t.level = 7;
        t.faction = 2;
        t.alignment = 1;
        t.entity_flags = 0xABCD;
        t.speaker_id = Some(156); // Vala -- arbitrary, just to verify pass-through
        t.body_set = Some("BS_HumanFemale.BS_HumanFemale".to_string());
    }

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::RequestEntityUpdate {
            witness_id: 1,
            entity_ids: vec![42],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // Expect exactly one EnteredAoI carrying entity 42's state.
    let mut entered = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EnteredAoI { .. } = msg {
            entered.push(msg);
        }
    }
    assert_eq!(
        entered.len(),
        1,
        "exactly one EnteredAoI re-emitted for the requested+witnessed entity"
    );
    match &entered[0] {
        CellToBaseMsg::EnteredAoI {
            witness_id,
            entity_id,
            class_id,
            position,
            level,
            npc_data,
            ..
        } => {
            assert_eq!(*witness_id, 1);
            assert_eq!(*entity_id, 42);
            assert_eq!(*class_id, 0x04);
            assert_eq!(*position, [5.0, 0.0, 0.0]);
            assert_eq!(*level, 7);
            let d = npc_data.as_ref().expect("NPC must carry npc_data");
            assert_eq!(d.faction, 2);
            assert_eq!(d.alignment, 1);
            assert_eq!(d.entity_flags, 0xABCD);
            assert_eq!(d.speaker_id, Some(156));
            assert_eq!(d.body_set.as_deref(), Some("BS_HumanFemale.BS_HumanFemale"));
        }
        other => panic!("unexpected message variant: {other:?}"),
    }
}

/// Security guard: witness does NOT have target in AoI. Re-emit MUST be
/// suppressed — otherwise a malicious client can probe any entity id and
/// receive its full state. Regression for the obvious authorization gap.
#[tokio::test]
async fn request_entity_update_drops_when_entity_not_in_witness_aoi() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(w) = mgr.get_entity_mut(1) {
        w.is_player = true;
        w.player_id = Some(100);
        // witnesses is intentionally empty — entity 42 exists but is NOT in
        // this player's AoI.
    }
    mgr.create_entity(42, "Castle_CellBlock", [500.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::RequestEntityUpdate {
            witness_id: 1,
            entity_ids: vec![42],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let mut entered = 0u32;
    while let Ok(msg) = rx.try_recv() {
        if matches!(msg, CellToBaseMsg::EnteredAoI { .. }) {
            entered += 1;
        }
    }
    assert_eq!(
        entered, 0,
        "out-of-AoI request must NOT emit EnteredAoI (anti-probe)"
    );
}

/// Mixed request: one entity in AoI, one out of AoI, one unknown id. Only
/// the in-AoI id should re-emit; the other two are dropped without error.
#[tokio::test]
async fn request_entity_update_filters_mixed_request_to_in_aoi_only() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(w) = mgr.get_entity_mut(1) {
        w.is_player = true;
        w.player_id = Some(100);
        w.witnesses.insert(EntityId(42)); // 42 is in AoI
                                          // 43 is NOT in AoI; 99 doesn't exist at all.
    }
    mgr.create_entity(42, "Castle_CellBlock", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.create_entity(43, "Castle_CellBlock", [500.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::RequestEntityUpdate {
            witness_id: 1,
            entity_ids: vec![42, 43, 99],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let mut emitted_ids = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EnteredAoI { entity_id, .. } = msg {
            emitted_ids.push(entity_id);
        }
    }
    assert_eq!(
        emitted_ids,
        vec![42],
        "only the in-AoI requested entity should re-emit; out-of-AoI and unknown ids dropped"
    );
}

/// Unknown witness entity (e.g. a request from a stale connection) drops
/// the whole request without panic and without emitting anything.
#[tokio::test]
async fn request_entity_update_drops_when_witness_missing() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    // No entity 999 exists.

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::RequestEntityUpdate {
            witness_id: 999,
            entity_ids: vec![42],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let count = std::iter::from_fn(|| rx.try_recv().ok()).count();
    assert_eq!(
        count, 0,
        "unknown witness must produce no cell→base traffic"
    );
}
