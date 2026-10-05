//! DA-F3: a GM's world entry into the Debug Area grants the hub's gates.
//!
//! Sent with `onDisplayDHD`, the grants listed as "Unknown" on the first
//! DHD open: the client resolves each new address from its cooked cache
//! asynchronously and never redraws an open DHD. The world-entry pass in
//! `handle_init_player_state` sends them while the GM is still walking to
//! the DHD. Removing the `top_up_gm_on_hub_world_entry` call fails the first
//! test; the second keeps the pass off every other world.

use super::super::*;
use crate::cell::client_methods::gate_travel::UPDATE_STARGATE_ADDRESS;
use crate::cell::messages::CellToBaseMsg;
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

async fn init(mgr: &mut SpaceManager, world: &str, access_level: u32) -> Vec<i32> {
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(256);
    handle_init_player_state(
        1,
        100,
        world.into(),
        1,
        vec![],
        vec![],
        0,
        vec![],
        cimmeria_entity::cell_entity::SystemOptions::default(),
        access_level,
        &tx,
        mgr,
        &engine,
    )
    .await;
    let mut granted = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
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

    let granted = init(&mut mgr, "DebugArea", GM_LEVEL).await;

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
        init(&mut mgr, "Harset", GM_LEVEL).await.is_empty(),
        "a GM entering a world with no hub gate keeps their own book"
    );

    let mut mgr = make_mgr("DebugArea");
    assert!(
        init(&mut mgr, "DebugArea", 0).await.is_empty(),
        "a non-GM in the hub's world gets nothing"
    );
    assert!(mgr.get_entity(1).unwrap().known_stargates.is_empty());
}
