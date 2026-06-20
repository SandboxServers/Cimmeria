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
                instance_id: 0,
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

/// `InitPlayerState` threads the base-side player name onto the cell entity
/// (`CellEntity::character_name`). The cell otherwise has no display name for
/// a player, and the cell-side Discord seams (GM `.`-console audit trail,
/// mission/death/respawn emits) attribute events to it. The dispatcher sets
/// it before delegating to `handle_init_player_state`; dropping that line
/// would silently regress every cell-side emit to `entity:<id>` and trips
/// this guard.
#[tokio::test]
async fn init_player_state_caches_character_name_on_cell_entity() {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.is_player = true;
    }
    mgr.connect_entity(1);

    let (tx, _rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    handle_base_message(
        BaseToCellMsg::InitPlayerState {
            entity_id: 1,
            player_id: 100,
            world_name: "Castle_CellBlock".into(),
            archetype_id: 1,
            saved_missions: vec![],
            abilities: vec![],
            active_bandolier_slot: 0,
            bandolier_items: vec![],
            system_options: cimmeria_entity::cell_entity::SystemOptions::default(),
            state_field: 0,
            access_level: 0,
            character_name: Some("Daniel".into()),
        },
        &tx,
        &mut mgr,
        &engine,
        &[],
    )
    .await;

    assert_eq!(
        mgr.get_entity(1).unwrap().character_name.as_deref(),
        Some("Daniel"),
        "InitPlayerState must cache the player name on the cell entity so \
         cell-side seams can attribute events to a name rather than an id",
    );
}
