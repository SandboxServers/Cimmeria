//! The Debug Area dial hub (DA-07): a GM at the hub's DHD may dial every
//! gate the server can enter; nobody may dial the hub; non-GMs get nothing.
//! See [`super::super::dial_hub`] and the hub refusal in
//! [`super::super::address_book`].

use super::super::dial_feedback::{refusal_messages, DialRefusal};
use super::super::dial_hub::{top_up_gm_dial_hub, update_stargate_address_args, HubTopUp};
use super::*;
use crate::cell::client_methods::gate_travel::UPDATE_STARGATE_ADDRESS;

/// The Debug Area gate's real id (`resources.stargates` row 29).
const HUB: i32 = 29;
/// Harset (non-instanced, has a startup space): offered.
const HARSET: i32 = 3;
/// SGC_W1 (instanced, no startup space): offered — an instance is created
/// on arrival.
const SGC_W1: i32 = 27;
/// Hebridan (a `resources.worlds` row with no map this server loads):
/// left out.
const HEBRIDAN: i32 = 11;

const GM: u32 = 2;
const PLAYER: u32 = 0;

fn gate(id: i32, world: &str, hub: bool) -> (i32, StargateEntry) {
    (
        id,
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
        },
    )
}

/// Debug Area + Harset + an instanced SGC_W1, the hub gate, one gate on
/// each of the other two worlds and one on a world no space exists for.
/// The caller stands in the Debug Area with `access_level`.
fn hub_manager(access_level: u32) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="DebugArea" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" />
            <Space WorldName="Harset" Instanced="false" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" />
            <Space WorldName="SGC_W1" Instanced="true" MinX="-2000" MaxX="2000" MinY="-2000" MaxY="2000" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="DebugArea" />
            <Space WorldName="Harset" />
        </Spaces>"#,
    )
    .unwrap();
    mgr.stargates.extend([
        gate(HUB, "DebugArea", true),
        gate(HARSET, "Harset", false),
        gate(SGC_W1, "SGC_W1", false),
        gate(HEBRIDAN, "Hebridan", false),
    ]);
    mgr.create_entity(1, "DebugArea", [251.0, 8.0, -962.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(1);
    mgr.get_entity_mut(1).unwrap().access_level = access_level;
    // The fallback path, so an accepted dial is visible as a GateTravel in
    // one call. The arm-then-cross path is covered by `dial_timer`.
    strip_stargate_regions(&mut mgr);
    mgr
}

fn update_ids(sent: &[CellToBaseMsg]) -> Vec<i32> {
    sent.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } if *method_index == UPDATE_STARGATE_ADDRESS => {
                assert_eq!(args.len(), 6, "updateStargateAddress is INT32 + 2 x UINT8");
                Some(i32::from_le_bytes(args[..4].try_into().unwrap()))
            }
            _ => None,
        })
        .collect()
}

fn gate_travels(sent: &[CellToBaseMsg]) -> Vec<String> {
    sent.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::GateTravel {
                target_world_name, ..
            } => Some(target_world_name.clone()),
            _ => None,
        })
        .collect()
}

/// A GM who opens the hub's DHD holds every gate on an enterable world —
/// a startup space or an instanced world — and the client is told about
/// each one. Not the hub itself, and not a gate on a world with no space.
///
/// Reverting the `world_is_enterable` filter puts Hebridan in the book;
/// reverting the hub skip puts 29 there; removing the send leaves the DHD
/// listing nothing new.
#[tokio::test]
async fn a_gm_at_the_hub_is_granted_every_enterable_gate_and_the_client_is_told() {
    let capture = crate::test_support::LogCapture::install();
    let mut mgr = hub_manager(GM);
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    let topped = top_up_gm_dial_hub(1, HUB, &tx, &mut mgr).await;

    assert_eq!(
        topped,
        Some(HubTopUp {
            granted: vec![HARSET, SGC_W1],
            already_known: 0,
            unenterable: vec![HEBRIDAN],
        })
    );
    let mut book = mgr.get_entity(1).unwrap().known_stargates.clone();
    book.sort_unstable();
    assert_eq!(book, vec![HARSET, SGC_W1]);

    let sent = dial_feedback::drain(&mut rx);
    assert_eq!(update_ids(&sent), vec![HARSET, SGC_W1]);
    // Byte-exact: addressId, hasAddress = 1, hidden = 0 — the shape the
    // content verb `grant_stargate_address` sends and 2009 sent on every add.
    assert_eq!(
        update_stargate_address_args(HARSET),
        [3, 0, 0, 0, 1, 0],
        "updateStargateAddress(INT32 3, UINT8 1, UINT8 0)"
    );

    assert!(
        capture
            .find_event(tracing::Level::WARN, "debug dial hub", "gm_dial_hub_grant")
            .is_some(),
        "the grant is an audit event and must WARN with reason=gm_dial_hub_grant"
    );
}

/// The grant is what the one dial check then accepts: a GM in the Debug
/// Area dials Harset and travels. Without the top-up the same dial is
/// refused (control, second half).
#[tokio::test]
async fn a_gm_in_the_debug_area_can_dial_any_granted_gate() {
    let mut mgr = hub_manager(GM);
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    assert!(
        !handle_dial_gate(1, HARSET, HUB, &tx, &mut mgr, &engine()).await,
        "control: before the DHD is opened the GM does not hold Harset"
    );
    dial_feedback::drain(&mut rx);

    top_up_gm_dial_hub(1, HUB, &tx, &mut mgr).await;
    dial_feedback::drain(&mut rx);
    assert!(handle_dial_gate(1, HARSET, HUB, &tx, &mut mgr, &engine()).await);
    assert_eq!(gate_travels(&dial_feedback::drain(&mut rx)), vec!["Harset"]);
}

/// A non-GM who opens the hub's DHD gets nothing, so a gate they do not
/// hold stays undialable. Dropping the `is_gm` check grants them Harset and
/// the dial below travels.
#[tokio::test]
async fn a_non_gm_at_the_hub_gets_no_addresses_and_cannot_dial() {
    let capture = crate::test_support::LogCapture::install();
    let mut mgr = hub_manager(PLAYER);
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    assert_eq!(top_up_gm_dial_hub(1, HUB, &tx, &mut mgr).await, None);
    assert!(mgr.get_entity(1).unwrap().known_stargates.is_empty());
    assert!(update_ids(&dial_feedback::drain(&mut rx)).is_empty());
    assert!(capture
        .find_event(tracing::Level::INFO, "non-GM", "dial_hub_not_gm")
        .is_some());

    assert!(!handle_dial_gate(1, HARSET, HUB, &tx, &mut mgr, &engine()).await);
    let sent = dial_feedback::drain(&mut rx);
    assert!(gate_travels(&sent).is_empty(), "a non-GM must not travel");
    assert_eq!(
        dial_feedback::feedback_lines(&sent),
        vec![DialRefusal::UnknownAddress.text()]
    );
}

/// Outbound only: nobody dials INTO the hub, even a GM standing on another
/// world with the hub's id in their book (a crafted `onDialGate`, a stale
/// row, a grant path that forgot the filter). The refusal is the unknown-
/// address one, byte for byte, so the hub's id cannot be probed for.
///
/// Removing the `debug_dial_hub` arm from `player_knows_stargate` makes this
/// dial travel to the Debug Area.
#[tokio::test]
async fn nobody_can_dial_the_hub_even_with_its_id_in_the_book() {
    let mut mgr = hub_manager(GM);
    mgr.destroy_entity(1);
    mgr.create_entity(1, "Harset", [0.0; 3], [0.0; 3]).unwrap();
    mgr.connect_entity(1);
    let entity = mgr.get_entity_mut(1).unwrap();
    entity.access_level = GM;
    entity.known_stargates = vec![HUB, HARSET, SGC_W1];
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    assert!(!handle_dial_gate(1, HUB, HARSET, &tx, &mut mgr, &engine()).await);

    let sent = dial_feedback::drain(&mut rx);
    assert!(
        gate_travels(&sent).is_empty(),
        "the hub is never a destination"
    );
    assert!(
        mgr.gate_dial(1).is_none(),
        "nothing may be armed toward the hub"
    );
    let expected: Vec<(u16, Vec<u8>)> = refusal_messages(DialRefusal::UnknownAddress);
    let actual: Vec<(u16, Vec<u8>)> = sent
        .into_iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                method_index, args, ..
            } => Some((method_index, args)),
            _ => None,
        })
        .collect();
    assert_eq!(
        actual, expected,
        "a hub refusal must be byte-identical to an unknown-address refusal"
    );
}

/// The top-up only happens at a hub: a GM opening an ordinary gate's DHD
/// keeps their own book (gmDHD is the GM tool there).
#[tokio::test]
async fn a_gm_at_an_ordinary_gate_gets_no_top_up() {
    let mut mgr = hub_manager(GM);
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    assert_eq!(top_up_gm_dial_hub(1, HARSET, &tx, &mut mgr).await, None);
    assert!(mgr.get_entity(1).unwrap().known_stargates.is_empty());
    assert!(dial_feedback::drain(&mut rx).is_empty());
}

/// Opening the DHD again grants nothing twice: no duplicate in the book
/// (the DHD would list it twice) and no repeated client method.
#[tokio::test]
async fn reopening_the_hub_dhd_grants_nothing_twice() {
    let mut mgr = hub_manager(GM);
    mgr.get_entity_mut(1).unwrap().known_stargates = vec![HARSET];
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);

    let first = top_up_gm_dial_hub(1, HUB, &tx, &mut mgr).await.unwrap();
    assert_eq!(first.granted, vec![SGC_W1]);
    assert_eq!(first.already_known, 1);
    assert_eq!(update_ids(&dial_feedback::drain(&mut rx)), vec![SGC_W1]);

    let second = top_up_gm_dial_hub(1, HUB, &tx, &mut mgr).await.unwrap();
    assert!(second.granted.is_empty());
    assert_eq!(second.already_known, 2);
    assert!(update_ids(&dial_feedback::drain(&mut rx)).is_empty());
    let book = &mgr.get_entity(1).unwrap().known_stargates;
    assert_eq!(book.len(), 2, "no duplicates: {book:?}");
}
