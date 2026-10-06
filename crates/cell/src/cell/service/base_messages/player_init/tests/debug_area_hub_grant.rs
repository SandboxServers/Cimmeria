//! DA-F3: a GM's world entry into the Debug Area grants the hub's gates.
//!
//! Sent with `onDisplayDHD`, the grants listed as "Unknown" on the first
//! DHD open: the client resolves each new address from its cooked cache
//! asynchronously and never redraws an open DHD. The world-entry pass in
//! `handle_init_player_state` sends them while the GM is still walking to
//! the DHD. Every test drives the real `BaseToCellMsg::InitPlayerState`
//! dispatch, so the database address-book stamp in the dispatch arm runs
//! first, as in production. Removing the `top_up_gm_on_hub_world_entry`
//! call fails the first test; moving the stamp after the grant fails the
//! last; the second keeps the pass off every other world.

use super::super::*;
use crate::cell::client_methods::gate_travel::UPDATE_STARGATE_ADDRESS;
use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::cell::spawner::StargateEntry;

const HUB: i32 = 29;
const HARSET: i32 = 3;
const GM_LEVEL: u32 = 2;

fn gate(world: &str, hub: bool) -> StargateEntry {
    StargateEntry {
        world_name: world.to_string(),
        x: 0.0,
        y: 0.0,
        z: 0.0,
        yaw: 0.0,
        address_origin: 2,
        arrival: None,
        event_set_id: None,
        debug_dial_hub: hub,
    }
}

/// Player 1 standing in `world`, with the Debug Area hub (29) and Harset (3)
/// in the gate table; both worlds are startup spaces, so Harset is enterable.
fn make_mgr(world: &str) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="DebugArea" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" />
            <Space WorldName="Harset" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="DebugArea" /><Space WorldName="Harset" /></Spaces>"#,
    )
    .unwrap();
    mgr.stargates.extend([
        (HUB, gate("DebugArea", true)),
        (HARSET, gate("Harset", false)),
    ]);
    mgr.create_entity(1, world, [0.0; 3], [0.0; 3]).unwrap();
    mgr.connect_entity(1);
    mgr
}

/// Send `InitPlayerState` through the cell's base-message dispatcher with
/// `known_stargates` as the database row's address book, and return the gate
/// ids the client was told about (single calls or a batch).
async fn init_with_book(
    mgr: &mut SpaceManager,
    world: &str,
    access_level: u32,
    known_stargates: Vec<i32>,
) -> Vec<i32> {
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(256);
    let msg = BaseToCellMsg::InitPlayerState {
        entity_id: 1,
        player_id: 100,
        account_id: 10,
        world_name: world.into(),
        archetype_id: 1,
        saved_missions: vec![],
        abilities: vec![],
        active_bandolier_slot: 0,
        bandolier_items: vec![],
        system_options: cimmeria_entity::cell_entity::SystemOptions::default(),
        access_level,
        known_stargates,
        tree_progress: Default::default(),
        level: 1,
        character_name: Some("Tester".into()),
        body_set: None,
        looted_containers: vec![],
        shown_tutorials: Vec::new(),
    };
    super::super::super::handle_base_message(msg, &tx, mgr, &engine, &[]).await;
    let mut granted = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        let calls = match msg {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => vec![(method_index, args)],
            CellToBaseMsg::EntityMethodCallBatch { calls, .. } => calls,
            _ => continue,
        };
        for (method_index, args) in calls {
            if method_index == UPDATE_STARGATE_ADDRESS {
                granted.push(i32::from_le_bytes(args[..4].try_into().unwrap()));
            }
        }
    }
    granted
}

#[tokio::test]
async fn a_gm_entering_the_debug_area_is_granted_the_hub_gates_at_world_entry() {
    let mut mgr = make_mgr("DebugArea");

    let granted = init_with_book(&mut mgr, "DebugArea", GM_LEVEL, vec![]).await;

    assert_eq!(
        granted,
        vec![HARSET],
        "the client is told at world entry, not when the DHD opens"
    );
    assert_eq!(mgr.get_entity(1).unwrap().known_stargates, vec![HARSET]);
}

#[tokio::test]
async fn world_entry_grants_nothing_off_the_hub_world_or_to_a_non_gm() {
    let mut mgr = make_mgr("Harset");
    assert!(
        init_with_book(&mut mgr, "Harset", GM_LEVEL, vec![])
            .await
            .is_empty(),
        "a GM entering a world with no hub gate keeps their own book"
    );

    let mut mgr = make_mgr("DebugArea");
    assert!(
        init_with_book(&mut mgr, "DebugArea", 0, vec![])
            .await
            .is_empty(),
        "a non-GM in the hub's world gets nothing"
    );
    assert!(mgr.get_entity(1).unwrap().known_stargates.is_empty());
}

/// The ordering the grant relies on: the dispatch arm stamps the database
/// address book *before* the handler grants, so the GM ends up holding both
/// their own gate and the hub's grants. If the stamp moved after the grant,
/// the in-memory grant would be wiped while the client still lists the
/// gates, and every dial would be refused as an unknown address.
#[tokio::test]
async fn the_database_book_is_stamped_before_the_world_entry_grant() {
    const OWN_GATE: i32 = 15;
    let mut mgr = make_mgr("DebugArea");

    let granted = init_with_book(&mut mgr, "DebugArea", GM_LEVEL, vec![OWN_GATE]).await;

    assert_eq!(granted, vec![HARSET]);
    let mut book = mgr.get_entity(1).unwrap().known_stargates.clone();
    book.sort_unstable();
    assert_eq!(
        book,
        vec![HARSET, OWN_GATE],
        "own gate kept, hub grant added"
    );
}
