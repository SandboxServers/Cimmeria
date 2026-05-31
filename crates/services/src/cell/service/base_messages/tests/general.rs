use super::*;

#[tokio::test]
async fn destroy_entity_flushes_dirty_bandolier_and_destroys_entity() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 10,
                clip_size: 30,
                default_ammo_type: 1,
                current_ammo: 17,
                cur_ammo_type: 1,
            },
        );
        e.bandolier_ammo_dirty.insert(0);
    }

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::DestroyEntity { entity_id: 1 },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // A BandolierAmmoUpdate must be sent exactly once while handling destroy.
    let mut flush_count = 0u32;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::BandolierAmmoUpdate { player_id, .. } = msg {
            assert_eq!(player_id, 100);
            flush_count += 1;
        }
    }
    assert_eq!(
        flush_count, 1,
        "DestroyEntity must flush exactly one BandolierAmmoUpdate before tearing down"
    );
    assert!(
        mgr.get_entity(1).is_none(),
        "entity must be destroyed after flush"
    );
}

#[tokio::test]
async fn entity_move_updates_position_in_space_manager() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::EntityMove {
            entity_id: 1,
            position: [10.0, 20.0, 30.0],
            direction: [0, 0, 0],
            velocity: [1.0, 2.0, 3.0],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let entity = mgr.get_entity(1).unwrap();
    assert_eq!(entity.position.x, 10.0);
    assert_eq!(entity.position.y, 20.0);
    assert_eq!(entity.position.z, 30.0);
    assert_eq!(entity.velocity, [1.0, 2.0, 3.0]);
}

/// EntityMove with an out-of-bounds client position must:
///   1. NOT advance `cell_entity.position` (so AoI rebroadcasts the
///      last-valid position on the next tick to all witnesses).
///   2. Emit a `CellToBaseMsg::TeleportPlayer` with `position` and
///      `prev_pos` BOTH set to the last-valid position so the base
///      handler composes `BASEMSG_FORCED_POSITION` (via the existing
///      `compose_forced_position_body`) and the offending client snaps
///      back to where it was.
///   3. Emit a `warn!` with the canonical negative-log fields
///      `reason = "bounds"`, `entity_id`, `client_*`, `last_valid_*`,
///      `bounds_min_*`, `bounds_max_*`.
///
/// Reverting the validator call or the snap-back send breaks the
/// `CellToBaseMsg::TeleportPlayer { position == prev_pos == SPAWN_POS }`
/// shape; reverting the warn! upgrade to trace! breaks the LogCapture
/// assertion. The test catches all three regression shapes at once.
#[tokio::test]
async fn entity_move_out_of_bounds_rejects_and_emits_teleport_player_snap_back() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    let spawn_pos = [10.0_f32, 5.0, 20.0];
    mgr.create_entity(7777, "Castle_CellBlock", spawn_pos, [0.0; 3])
        .unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    // Client claims a position far outside the fallback AABB
    // ([-10_000, 10_000] on every axis).
    let attacker_pos = [50_000.0_f32, 5.0, 20.0];

    handle_base_message(
        BaseToCellMsg::EntityMove {
            entity_id: 7777,
            position: attacker_pos,
            direction: [0, 0, 0],
            velocity: [0.0; 3],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // 1. cell entity position must be unchanged
    let entity = mgr.get_entity(7777).unwrap();
    assert_eq!(
        entity.position.x, spawn_pos[0],
        "rejected client position must not have advanced cell_entity.position"
    );
    assert_eq!(entity.position.y, spawn_pos[1]);
    assert_eq!(entity.position.z, spawn_pos[2]);

    // 2. exactly one TeleportPlayer must have been queued for the
    //    base, snapping the player back to spawn (position == prev_pos
    //    == spawn). This is what makes the wire packet to the client
    //    a `BASEMSG_FORCED_POSITION` body containing the spawn coords
    //    in both the position and prev-position slots — see
    //    `compose_forced_position_body` wire layout.
    let mut saw_snap_back = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::TeleportPlayer {
            entity_id,
            position,
            prev_pos,
            ..
        } = msg
        {
            assert_eq!(entity_id, 7777);
            assert_eq!(position, spawn_pos, "snap target must be last-valid spawn");
            assert_eq!(
                prev_pos, spawn_pos,
                "snap prev_pos must also be last-valid spawn — a zero-distance \
                 interpolation on the client so no visible jitter"
            );
            saw_snap_back = true;
        }
    }
    assert!(
        saw_snap_back,
        "out-of-bounds client position must queue a TeleportPlayer snap-back \
         to the base; without it the player desyncs (server has SPAWN_POS, \
         client keeps believing it's at attacker_pos)"
    );

    // 3. canonical negative-log shape — pin level + the `reason` field
    //    so a generic level revert AND a reason-removal revert both
    //    trip this guard. The exact log message text is allowed to
    //    drift; field names are the contract.
    let event = capture
        .find_event(Level::WARN, "movement.bounds_violation", "bounds")
        .expect(
            "warn-level movement.bounds_violation with reason='bounds' must fire \
             — the negative-log convention pins this field shape across the \
             validator's evolution",
        );
    assert!(
        event.has_field("entity_id", "7777"),
        "warn log must carry entity_id={{7777}}; got {event:#?}"
    );
    // bounds_min/max as f32 go through record_debug (`?`-formatted) —
    // any non-zero number suffices; we assert the field is present so
    // a regression that drops the field name fires.
    assert!(
        event.fields.contains_key("bounds_min_x"),
        "warn log must carry bounds_min_x; got {event:#?}"
    );
    assert!(
        event.fields.contains_key("bounds_max_x"),
        "warn log must carry bounds_max_x; got {event:#?}"
    );
    assert!(
        event.fields.contains_key("last_valid_x"),
        "warn log must carry last_valid_x; got {event:#?}"
    );
    assert!(
        event.fields.contains_key("client_x"),
        "warn log must carry client_x; got {event:#?}"
    );
}

/// Wire-format byte contract: the snap-back path's `TeleportPlayer`
/// fields are exactly the (entity_id, space_id, last_valid, last_valid)
/// quadruple that `compose_forced_position_body` then encodes into a
/// `BASEMSG_FORCED_POSITION (0x31)` body — the only authoritative
/// move primitive per the movement-teleport-advisor's invariant.
///
/// The 50-byte composed body is checked against the canonical encoding
/// so a refactor that swapped any of the 4 fields (e.g. accidentally
/// sending the rejected client_pos instead of last_valid) trips here
/// instead of producing a desync in production.
#[tokio::test]
async fn snap_back_message_routes_through_compose_forced_position_body() {
    use crate::mercury::compose_forced_position_body;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    let spawn_pos = [7.0_f32, 8.0, 9.0];
    let space_id = mgr
        .create_entity(0xCAFE, "Castle_CellBlock", spawn_pos, [0.0; 3])
        .unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::EntityMove {
            entity_id: 0xCAFE,
            position: [1.0e9, 0.0, 0.0], // far out-of-bounds attacker pos
            direction: [0, 0, 0],
            velocity: [0.0; 3],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let snap = loop {
        match rx.try_recv() {
            Ok(CellToBaseMsg::TeleportPlayer {
                entity_id,
                space_id,
                position,
                prev_pos,
            }) => break (entity_id, space_id, position, prev_pos),
            Ok(_) => continue,
            Err(_) => panic!("expected exactly one TeleportPlayer in the queue"),
        }
    };
    let (snap_eid, snap_space, snap_pos, snap_prev) = snap;
    assert_eq!(snap_eid, 0xCAFE);
    assert_eq!(snap_space, space_id);
    assert_eq!(snap_pos, spawn_pos);
    assert_eq!(snap_prev, spawn_pos);

    // Byte-exact: composing `BASEMSG_FORCED_POSITION` from the queued
    // fields must produce the canonical 50-byte body. A reordering
    // (e.g. swapping position/prev_pos) or width drift in any field
    // would shift bytes here.
    let body = compose_forced_position_body(snap_eid, snap_space, snap_pos, snap_prev);
    assert_eq!(body.len(), 50, "FORCED_POSITION body is 50 bytes");
    assert_eq!(
        body[0], 0x31,
        "first byte is BASEMSG_FORCED_POSITION (0x31)"
    );
    // The position triple at offsets 13..25 must match spawn_pos
    // little-endian (entity_id 4 + space_id 4 + vehicle_id 4 + tag 1).
    assert_eq!(&body[13..17], &spawn_pos[0].to_le_bytes());
    assert_eq!(&body[17..21], &spawn_pos[1].to_le_bytes());
    assert_eq!(&body[21..25], &spawn_pos[2].to_le_bytes());
    // The prev-position triple at offsets 25..37 must also be
    // spawn_pos (zero-distance interpolation — no client jitter).
    assert_eq!(&body[25..29], &spawn_pos[0].to_le_bytes());
    assert_eq!(&body[29..33], &spawn_pos[1].to_le_bytes());
    assert_eq!(&body[33..37], &spawn_pos[2].to_le_bytes());
}

/// After a bounds rejection, the next AoI tick must rebroadcast the
/// **last-valid** position to witnesses — NOT the rejected client
/// position. This is the AoI-refresh contract for the snap-back:
/// because we deliberately did NOT advance `cell_entity.position`, the
/// AoI loop reads the unchanged last-valid pos and fans it out as the
/// new `EntityMoved` to every player whose witness set contains the
/// offender.
///
/// Without this contract, witnesses would briefly render the offender
/// at the rejected coordinates between the bounds-reject moment and
/// the next legitimate position update — the failure mode the agent's
/// "forgetting to force an AoI refresh after a teleport" rule flags.
#[tokio::test]
async fn snap_back_triggers_aoi_refresh_to_last_valid_position() {
    // Use a non-instanced world so both entities share one space and
    // can be in each other's AoI. Castle_CellBlock is instanced —
    // every `create_entity` there builds a fresh space.
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos"/></Spaces>"#,
    )
    .unwrap();

    let offender_spawn = [10.0_f32, 0.0, 10.0];
    let witness_spawn = [12.0_f32, 0.0, 12.0]; // inside default AoI radius
    mgr.create_entity(100, "Agnos", witness_spawn, [0.0; 3])
        .unwrap();
    mgr.create_entity(200, "Agnos", offender_spawn, [0.0; 3])
        .unwrap();
    mgr.connect_entity(100);
    mgr.connect_entity(200);

    // First tick — populate witness sets so the (100, 200) pair is
    // tracked as "in both previous and current AoI" on the next tick.
    let _ = mgr.compute_aoi_changes();

    let (tx, _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    // Offender tries to warp to (1e9, 0, 1e9). With the validator in
    // place this is rejected and `cell_entity.position` stays at
    // `offender_spawn`, so witness 100's AoI on the next tick still
    // contains 200 (no LeftAoI fires).
    //
    // Without the validator (the revert case), `cell_entity.position`
    // jumps to (1e9, 0, 1e9), 200 falls out of 100's AoI, the next
    // tick fires `LeftAoI` instead of `EntityMoved`, and the regression
    // guard below trips on "expected EntityMoved, got LeftAoI".
    handle_base_message(
        BaseToCellMsg::EntityMove {
            entity_id: 200,
            position: [1.0e9_f32, 0.0, 1.0e9],
            direction: [0, 0, 0],
            velocity: [0.0; 3],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // Next AoI tick — witness 100 must receive an EntityMoved for 200
    // carrying the LAST-VALID position (offender_spawn), not the
    // rejected attacker position. AoI tick emits EntityMoved for every
    // pair that survived in the witness set, including same-position
    // entities — see `compute_aoi_changes`.
    let events = mgr.compute_aoi_changes();
    let moved_for_witness: Vec<[f32; 3]> = events
        .iter()
        .filter_map(|e| match e {
            CellToBaseMsg::EntityMoved {
                witness_id,
                entity_id,
                position,
                ..
            } if *witness_id == 100 && *entity_id == 200 => Some(*position),
            _ => None,
        })
        .collect();

    // 200 must NOT have left 100's AoI — that would mean the rejected
    // attacker position was written and the entity warped outside the
    // AoI radius. A regression that bypasses the validator trips here.
    let saw_leave = events.iter().any(|e| {
        matches!(
            e,
            CellToBaseMsg::LeftAoI {
                witness_id: 100,
                entity_id: 200,
            }
        )
    });
    assert!(
        !saw_leave,
        "AoI must NOT fire LeftAoI for the offender — that would mean the \
         rejected attacker position was written to cell_entity.position and \
         the spatial grid registered 200 at (1e9, 0, 1e9), warping it out of \
         witness 100's AoI. The validator's reject path must leave the cell \
         entity at its last-valid position. Events: {events:?}"
    );
    assert_eq!(
        moved_for_witness.len(),
        1,
        "AoI must emit exactly one EntityMoved for the offender→witness pair \
         after the snap-back; got {} events overall: {events:?}",
        moved_for_witness.len()
    );
    assert_eq!(
        moved_for_witness[0], offender_spawn,
        "AoI fan-out after snap-back must carry the LAST-VALID position \
         (offender_spawn), not the rejected client position. Got {:?}; a \
         regression that wrote the rejected pos into cell_entity.position \
         would leak the attacker_pos to witnesses here.",
        moved_for_witness[0]
    );
}

/// **Canonical authorized-teleport non-anomaly regression guard.**
///
/// The most-flagged failure mode for movement validators: an
/// authoritative server-side teleport (ring transport, respawn,
/// content-engine teleport, `handle_teleport_player`'s
/// `BASEMSG_FORCED_POSITION` snap) leaves the validator in a state
/// that then rejects the next legitimate client position update — so
/// the player gets snapped back to where they were before the
/// authorized move.
///
/// The PR1 contract: bounds-check looks at the **current space's**
/// AABB on every check, so as long as the authorized destination is
/// inside the same space's bounds, the subsequent client position
/// update near it must be accepted.
///
/// PR3 will extend the regression coverage to the speed and
/// teleport-detection layers via a per-entity authorized-teleport
/// allowlist. PR1 only pins the bounds-layer contract here.
///
/// Reverting `apply_client_position_update`'s seam to silently bypass
/// the validator would still pass this test (it tests the accept
/// side); reverting the post-teleport position write inside
/// `update_entity_position` itself would push the post-teleport
/// client coordinate >0.1 units from the cell entity's stale pos
/// and... still accept under bounds alone. PR3 is where this guard
/// graduates from "smoke" to "fire on revert of last_pos reseed".
#[tokio::test]
async fn test_authorized_teleport_does_not_trigger_bounds_anomaly() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(7777, "Castle_CellBlock", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    // Step 1: server-authoritative teleport via the unchecked path —
    // same shape every authorized-teleport path uses today:
    //   - handle_teleport_player → update_entity_position + bundle
    //   - ring transport dispatch → update_entity_position + emit
    //   - respawn → update_entity_position + emit
    //   - content executor transport → update_entity_position + emit
    let teleport_dst = [500.0_f32, 0.0, 500.0];
    mgr.update_entity_position(7777, teleport_dst, [0, 0, 0], [0.0; 3]);

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    // Step 2: client sends a small-delta position update right next
    // to the teleport destination — the legitimate continuation of
    // a normal teleport (client continues at FORCED_POSITION + smoothing).
    let client_pos = [500.1_f32, 0.0, 500.1];
    handle_base_message(
        BaseToCellMsg::EntityMove {
            entity_id: 7777,
            position: client_pos,
            direction: [0, 0, 0],
            velocity: [0.0; 3],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // No TeleportPlayer snap-back must have been queued — a snap-back
    // on a legitimate post-teleport update IS the canonical failure mode.
    let mut spurious_snap = None;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::TeleportPlayer { position, .. } = msg {
            spurious_snap = Some(position);
        }
    }
    assert!(
        spurious_snap.is_none(),
        "authorized teleport followed by a small-delta client update must \
         NOT queue a snap-back TeleportPlayer; doing so is the canonical \
         false-positive: the player gets warped back to where they were \
         before the authorized move. Got spurious snap to {spurious_snap:?}"
    );

    // The cell entity must now hold the client's post-teleport pos.
    let entity = mgr.get_entity(7777).unwrap();
    assert_eq!(entity.position.x, 500.1);
    assert_eq!(entity.position.y, 0.0);
    assert_eq!(entity.position.z, 500.1);
}

/// `InventoryItemMoveApplied` with target=bandolier (3) and source≠3
/// fires the `OnItemEquipped` content event. Pin the dispatch path
/// end-to-end by registering a chain that reacts with
/// `IncrementCounter` and asserting the entity's counter moved.
#[tokio::test]
async fn item_move_applied_into_bandolier_fires_equip_event() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
    }

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 9100,
        name: "test: bandolier-equip → bump".to_string(),
        enabled: true,
        trigger: Trigger::OnItemEquipped { item_id: Some(55) },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: "test_bandolier_equip".to_string(),
            amount: 1,
        }],
        priority: 0,
    });

    let (tx, _rx) = mpsc::channel(16);
    handle_base_message(
        BaseToCellMsg::InventoryItemMoveApplied {
            entity_id: 1,
            item_id: 0xABCD,
            type_id: 55,
            source_container_id: 1, // backpack
            target_container_id: 3, // bandolier
            swapped_item_id: None,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let entity = mgr.get_entity(1).expect("entity must still exist");
    assert_eq!(
        entity.counters.get("test_bandolier_equip"),
        Some(&1),
        "move into bandolier (target=3, source≠3) must fire OnItemEquipped",
    );
}

/// A move WITHIN the bandolier (source=3, target=3 — the player
/// reordering their bandolier slots) must NOT fire `OnItemEquipped`.
/// Without this guard, every drag between bandolier slots would
/// re-fire equip chains and re-grant whatever they grant.
#[tokio::test]
async fn item_move_within_bandolier_does_not_fire_equip_event() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
    }

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 9101,
        name: "test: any equip → bump".to_string(),
        enabled: true,
        trigger: Trigger::OnItemEquipped { item_id: None },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: "test_within_bandolier".to_string(),
            amount: 1,
        }],
        priority: 0,
    });

    let (tx, _rx) = mpsc::channel(16);
    handle_base_message(
        BaseToCellMsg::InventoryItemMoveApplied {
            entity_id: 1,
            item_id: 0xABCD,
            type_id: 55,
            source_container_id: 3, // bandolier → bandolier
            target_container_id: 3,
            swapped_item_id: None,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let entity = mgr.get_entity(1).expect("entity must still exist");
    assert!(
        !entity.counters.contains_key("test_within_bandolier"),
        "bandolier-internal move must not fire OnItemEquipped; got {:?}",
        entity.counters,
    );
}

/// A move OUT of the bandolier (source=3, target=1) must not fire
/// `OnItemEquipped` either — that's an unequip, not an equip.
#[tokio::test]
async fn item_move_out_of_bandolier_does_not_fire_equip_event() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
    }

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 9102,
        name: "test: any equip → bump".to_string(),
        enabled: true,
        trigger: Trigger::OnItemEquipped { item_id: None },
        conditions: vec![],
        actions: vec![Action::IncrementCounter {
            counter_name: "test_unequip_path".to_string(),
            amount: 1,
        }],
        priority: 0,
    });

    let (tx, _rx) = mpsc::channel(16);
    handle_base_message(
        BaseToCellMsg::InventoryItemMoveApplied {
            entity_id: 1,
            item_id: 0xABCD,
            type_id: 55,
            source_container_id: 3, // bandolier → backpack
            target_container_id: 1,
            swapped_item_id: None,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    let entity = mgr.get_entity(1).expect("entity must still exist");
    assert!(
        !entity.counters.contains_key("test_unequip_path"),
        "unequip (source=3, target=1) must not fire OnItemEquipped",
    );
}

/// `BaseToCellMsg::AdvanceRingDestination` is the cross-world ring
/// transport's deferred-load callback. After the source ring's
/// `Effect::TeleportCrossWorld` fires, the destination ring's FSM
/// sits in `RemoteLoadWait` until base sends this message back
/// (after the destination world's `onClientReady`). The handler
/// must forward to `ring_transport::handle_remote_player_loaded`
/// without crashing when the destination ring isn't loaded — the
/// integration-shaped fail-soft path that lets a ring not pre-loaded
/// in this cell instance be a quiet no-op rather than a panic.
#[tokio::test]
async fn advance_ring_destination_forwards_without_panic_when_ring_absent() {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    // Player is on Castle but no ring transporter region 34 is
    // loaded — handle_remote_player_loaded must short-circuit
    // gracefully rather than panic on the missing region lookup.
    mgr.create_entity(2, "Castle", [466.365, 70.397, 991.466], [0.0; 3])
        .unwrap();

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::AdvanceRingDestination {
            entity_id: 2,
            region_id: 34,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // The entity must still exist — AdvanceRingDestination doesn't
    // tear anything down on its own; it just records a load on the
    // destination ring's FSM (which is a no-op when the ring isn't
    // loaded). Pinning post-state-equality guards against a future
    // refactor that accidentally couples the dispatcher to entity
    // teardown.
    assert!(
        mgr.get_entity(2).is_some(),
        "AdvanceRingDestination must not destroy the recipient entity"
    );
}

#[tokio::test]
async fn minigame_result_victory_fires_on_victory_chains() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        // Set HP below max so ChangeStat +10 actually advances
        if let Some(h) = e.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            h.update(0, 50, 100);
        }
        e.stats.clear_dirty();
    }
    mgr.connect_entity(1);

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 9999,
        name: "test-victory-chain".into(),
        enabled: true,
        trigger: Trigger::OnInteractTag {
            entity_tag: "__unused__".into(),
        },
        conditions: vec![],
        actions: vec![Action::ChangeStat {
            stat_id: cimmeria_entity::stats::HEALTH,
            min: None,
            max: None,
            use_ammo_stat: None,
            set_to_max: None,
            amount: Some(10),
        }],
        priority: 1,
    });

    let (tx, mut rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::MinigameResult {
            entity_id: 1,
            result_code: 1, // victory
            on_victory_chains: vec![9999],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // ChangeStat emits onStatUpdate via the executor
    let msg = rx
        .try_recv()
        .expect("victory chain must fire and produce onStatUpdate");
    match msg {
        crate::cell::messages::CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(method_index, crate::mercury::method_idx::ON_STAT_UPDATE);
        }
        other => panic!("expected EntityMethodCall(onStatUpdate), got {other:?}"),
    }
}

#[tokio::test]
async fn minigame_result_defeat_does_not_fire_chains() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
    }
    mgr.connect_entity(1);

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 9999,
        name: "test-victory-chain".into(),
        enabled: true,
        trigger: Trigger::OnInteractTag {
            entity_tag: "__unused__".into(),
        },
        conditions: vec![],
        actions: vec![Action::ChangeStat {
            stat_id: cimmeria_entity::stats::HEALTH,
            min: None,
            max: None,
            use_ammo_stat: None,
            set_to_max: None,
            amount: Some(10),
        }],
        priority: 1,
    });

    let (tx, mut rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::MinigameResult {
            entity_id: 1,
            result_code: 0, // defeat
            on_victory_chains: vec![9999],
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    assert!(
        rx.try_recv().is_err(),
        "defeat (result_code != 1) must not fire victory chains"
    );
}

#[tokio::test]
async fn item_used_fires_on_item_use_content_event() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
        e.player_id = Some(100);
        // Set HP below max so ChangeStat +10 actually advances
        if let Some(h) = e.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            h.update(0, 50, 100);
        }
        e.stats.clear_dirty();
    }
    mgr.connect_entity(1);

    let mut engine = ChainEngine::new();
    engine.register_chain(Chain {
        id: 5001,
        name: "test-item-use-chain".into(),
        enabled: true,
        trigger: Trigger::OnItemUse { item_id: 42 },
        conditions: vec![],
        actions: vec![Action::ChangeStat {
            stat_id: cimmeria_entity::stats::HEALTH,
            min: None,
            max: None,
            use_ammo_stat: None,
            set_to_max: None,
            amount: Some(10),
        }],
        priority: 1,
    });

    let (tx, mut rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::ItemUsed {
            entity_id: 1,
            instance_id: 1001,
            type_id: 42,
            target_id: 0,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    // ChangeStat emits onStatUpdate via the executor
    let msg = rx
        .try_recv()
        .expect("ItemUsed must fire OnItemUse chain and produce onStatUpdate");
    match msg {
        crate::cell::messages::CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(method_index, crate::mercury::method_idx::ON_STAT_UPDATE);
        }
        other => panic!("expected EntityMethodCall(onStatUpdate), got {other:?}"),
    }
}

#[tokio::test]
async fn item_used_drops_event_when_no_player_id() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = false; // NPC — no player_id
    }
    mgr.connect_entity(1);

    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);
    handle_base_message(
        BaseToCellMsg::ItemUsed {
            entity_id: 1,
            instance_id: 1001,
            type_id: 42,
            target_id: 0,
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    assert!(
        rx.try_recv().is_err(),
        "entity without player_id must drop ItemUsed event silently"
    );
}
