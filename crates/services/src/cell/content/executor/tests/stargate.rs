//! `Action::GrantStargateAddress` — the content port of 2009's
//! `Act_StargateAddress` node (Harset H55).
//!
//! Covers the three legs a grant has to reach (cell entity, client method
//! 66, base persistence request), the byte layout of 66, every refusal,
//! and the end-to-end contract the packet exists for: a dial to Harset is
//! refused before the grant and accepted after it.

use super::*;

use crate::cell::client_methods::gate_travel::UPDATE_STARGATE_ADDRESS;
use crate::cell::client_methods::player::ON_ERROR_CODE;
use crate::cell::spawner::StargateEntry;

/// Harset's address (`db/resources/Worlds/Seed/stargates.sql`, the row with
/// `world_id = 57`). Castle mission 708 step 4462 dials this one.
const HARSET_GATE: i32 = 3;
/// Castle's own gate, the world the player is standing in here.
const CASTLE_GATE: i32 = 2;
const PLAYER_EID: u32 = 1;
const PLAYER_ID: i32 = 42;
const CHAIN: i64 = 1357;

/// Castle + Harset, one gate each, plus the `REGION_FLAG_Stargate` volume
/// Castle really has (`point_sets` row 1002). The region matters: without
/// it `handle_dial_gate` takes the no-gate-volume fallback and travels
/// immediately instead of arming, and the dial-acceptance assertion below
/// wants the arm.
fn make_two_world_mgr() -> SpaceManager {
    use crate::cell::space_manager::{RegionData, REGION_FLAG_CLIENT_HINTED, REGION_FLAG_STARGATE};

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Castle" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
            <Space WorldName="Harset" Instanced="false" MinX="0" MaxX="1000" MinY="0" MaxY="1000" />
        </Spaces>"#;
    let cxml = r#"<?xml version="1.0"?><Spaces>
            <Space WorldName="Castle" />
            <Space WorldName="Harset" />
        </Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(cxml).unwrap();

    for (id, world) in [(CASTLE_GATE, "Castle"), (HARSET_GATE, "Harset")] {
        mgr.stargates.insert(
            id,
            StargateEntry {
                world_name: world.to_string(),
                x: 0.0,
                y: 0.0,
                z: 0.0,
                yaw: 0.0,
                address_origin: id,
                arrival: None,
                event_set_id: None,
            },
        );
    }

    let runtime_id = mgr.next_region_id;
    mgr.next_region_id += 1;
    mgr.regions.insert(
        runtime_id,
        RegionData {
            runtime_id,
            db_set_id: 1002,
            tag: "Castle.Stargate".to_string(),
            world_name: "Castle".to_string(),
            height: 10.0,
            radius: 2.5,
            flags: REGION_FLAG_CLIENT_HINTED | REGION_FLAG_STARGATE,
            points: vec![[0.0; 3]; 4],
        },
    );

    mgr.create_entity(PLAYER_EID, "Castle", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(PLAYER_EID) {
        p.is_player = true;
        p.player_id = Some(PLAYER_ID);
    }
    mgr.connect_entity(PLAYER_EID);
    mgr
}

fn grant(stargate_id: i32) -> ResolvedActions {
    ResolvedActions {
        actions: vec![(CHAIN, Action::GrantStargateAddress { stargate_id })],
        action_delays: Vec::new(),
        params: std::collections::HashMap::new(),
    }
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

/// All three legs, and the byte layout of leg 2.
///
/// `updateStargateAddress(INT32 addressId, UINT8 hasAddress, UINT8 hidden)`
/// — `entities/defs/interfaces/GateTravel.def`, matching 2009's
/// `self.client.updateStargateAddress(addressId, 1, 1 if isHidden else 0)`
/// at `deprecated/python/cell/SGWPlayer.py:626`. Six bytes, little-endian
/// id, `hasAddress = 1`, `hidden = 0` (Cimmeria has no hidden list).
#[tokio::test]
async fn a_grant_writes_the_cell_book_tells_the_client_and_asks_the_base_to_persist() {
    let mut mgr = make_two_world_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    execute_actions(
        grant(HARSET_GATE),
        PLAYER_EID,
        PLAYER_ID,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert_eq!(
        mgr.get_entity(PLAYER_EID).unwrap().known_stargates,
        vec![HARSET_GATE],
        "leg 1: the cell's own book is what `handle_dial_gate` enforces against",
    );

    let msgs = drain(&mut rx);
    assert_eq!(
        msgs.len(),
        2,
        "exactly the client notification and the persistence request; got {msgs:?}",
    );
    match &msgs[0] {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(*entity_id, PLAYER_EID);
            assert_eq!(
                *method_index, UPDATE_STARGATE_ADDRESS,
                "leg 2 is client method 66",
            );
            assert_eq!(
                args.as_slice(),
                &[0x03, 0x00, 0x00, 0x00, 0x01, 0x00],
                "updateStargateAddress is INT32 addressId (LE) + UINT8 hasAddress \
                 + UINT8 hidden; a wrong width here silently shifts the client's \
                 whole address book",
            );
        }
        other => panic!("expected the client notification first, got {other:?}"),
    }
    match &msgs[1] {
        CellToBaseMsg::GrantStargateAddress {
            entity_id,
            player_id,
            stargate_id,
        } => {
            assert_eq!(
                (*entity_id, *player_id, *stargate_id),
                (PLAYER_EID, PLAYER_ID, HARSET_GATE),
                "leg 3 carries the pair the base needs to key the UPDATE",
            );
        }
        other => panic!("expected the persistence request second, got {other:?}"),
    }
}

/// Idempotency at the cell. A re-fired chain (a replayed minigame victory,
/// a re-accepted mission) must not append twice, must not re-notify the
/// client, and must not spend a second base round trip. The DB append is
/// separately idempotent, because these are not the same lock.
#[tokio::test]
async fn a_second_grant_of_a_held_address_is_a_complete_no_op() {
    let mut mgr = make_two_world_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    execute_actions(
        grant(HARSET_GATE),
        PLAYER_EID,
        PLAYER_ID,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    assert_eq!(
        drain(&mut rx).len(),
        2,
        "precondition: the first grant lands"
    );

    execute_actions(
        grant(HARSET_GATE),
        PLAYER_EID,
        PLAYER_ID,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert_eq!(
        mgr.get_entity(PLAYER_EID).unwrap().known_stargates,
        vec![HARSET_GATE],
        "the book must not gain a duplicate -- `known_stargates` is a bare \
         integer[] with no uniqueness constraint, and the client renders it \
         in array order",
    );
    assert!(
        drain(&mut rx).is_empty(),
        "a second grant must emit neither the client notification nor the \
         persistence request",
    );
}

/// A `target_id` with no `resources.stargates` row is an authoring typo.
/// Granting it anyway would put an address in the player's DHD that the
/// dial handler is guaranteed to refuse one layer down — which in play
/// looks exactly like a server bug.
#[tokio::test]
async fn a_grant_for_an_unknown_stargate_id_is_refused_and_warns() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_two_world_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    execute_actions(grant(999), PLAYER_EID, PLAYER_ID, &tx, &mut mgr, &engine).await;

    assert!(
        mgr.get_entity(PLAYER_EID)
            .unwrap()
            .known_stargates
            .is_empty(),
        "an id with no gate must not enter the book",
    );
    assert!(drain(&mut rx).is_empty(), "and nothing must be sent");
    assert!(
        capture
            .find_event(
                Level::WARN,
                "no resources.stargates row",
                "grant_unknown_stargate",
            )
            .is_some(),
        "the refusal must be loud enough to name the seed row. Captured: {:#?}",
        capture.all(),
    );
}

/// The executor's `player_id` is `entity.player_id.unwrap_or(0)` at every
/// dispatcher, so a non-positive value means the actor has no DB
/// character. An NPC cannot own an address book; a chain that reaches
/// here fired on the wrong actor.
#[tokio::test]
async fn a_grant_by_a_non_player_actor_is_refused_and_warns() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_two_world_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    execute_actions(grant(HARSET_GATE), PLAYER_EID, 0, &tx, &mut mgr, &engine).await;

    assert!(
        mgr.get_entity(PLAYER_EID)
            .unwrap()
            .known_stargates
            .is_empty(),
        "no DB character means no address book to write",
    );
    assert!(drain(&mut rx).is_empty(), "and nothing must be sent");
    assert!(
        capture
            .find_event(
                Level::WARN,
                "carries no DB player id",
                "grant_non_player_actor",
            )
            .is_some(),
        "Captured: {:#?}",
        capture.all(),
    );
}

/// Both cell→base sends are on an expectation seam, and they fail
/// differently: a lost notification leaves an address the server accepts
/// and the client's DHD will not offer until the next map load, while a
/// lost persistence request costs the address at relog. Per
/// `docs/architecture/negative-logging-convention.md` neither may be a
/// bare `let _ = tx.send(..)`.
#[tokio::test]
async fn a_closed_cell_to_base_channel_warns_on_both_legs() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let capture = LogCapture::install();
    let mut mgr = make_two_world_mgr();
    let (tx, rx) = mpsc::channel(16);
    drop(rx);
    let engine = ChainEngine::new();

    execute_actions(
        grant(HARSET_GATE),
        PLAYER_EID,
        PLAYER_ID,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(
        capture
            .find_event(
                Level::WARN,
                "updateStargateAddress could not be enqueued",
                "grant_notify_send_failed",
            )
            .is_some(),
        "Captured: {:#?}",
        capture.all(),
    );
    assert!(
        capture
            .find_event(
                Level::ERROR,
                "the address works for this session only",
                "grant_persist_send_failed",
            )
            .is_some(),
        "Captured: {:#?}",
        capture.all(),
    );
}

/// The packet's reason for existing, end to end.
///
/// Before the grant, `handle_dial_gate` refuses (H06's CAT-O-01 gate),
/// tells the client with `onErrorCode` feedback 180
/// (`CONDITION_FEEDBACK_EntityDoesNotHaveStargateAddress`) and arms
/// nothing. After the content action runs, the same dial is accepted and
/// the 4-second dial is armed. Without this action there is no path
/// between those two states for a character who has never travelled —
/// `known_stargates` defaults to `'{}'` and nothing else writes it.
#[tokio::test]
async fn the_grant_is_what_turns_a_refused_harset_dial_into_an_accepted_one() {
    let mut mgr = make_two_world_mgr();
    let (tx, mut rx) = mpsc::channel(16);
    let engine = ChainEngine::new();

    let before = crate::cell::gate_travel::handle_dial_gate(
        PLAYER_EID,
        HARSET_GATE,
        CASTLE_GATE,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    assert!(!before, "a dial to an unheld address must be refused");
    assert!(
        mgr.gate_dial(PLAYER_EID).is_none(),
        "the refusal must land before anything is armed",
    );
    let refusal = drain(&mut rx);
    assert_eq!(refusal.len(), 1, "exactly the feedback; got {refusal:?}");
    match &refusal[0] {
        CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } => {
            assert_eq!(*method_index, ON_ERROR_CODE);
            assert_eq!(
                args.as_slice(),
                &[0x00, 0x00, 0x00, 0x00, 0x00, 0xB4, 0x00],
                "onErrorCode(UINT8 SystemID = 0, INT32 InstanceID = 0, \
                 UINT16 ErrorCodeID = 180)",
            );
        }
        other => panic!("expected the onErrorCode refusal, got {other:?}"),
    }

    execute_actions(
        grant(HARSET_GATE),
        PLAYER_EID,
        PLAYER_ID,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    drain(&mut rx);

    let after = crate::cell::gate_travel::handle_dial_gate(
        PLAYER_EID,
        HARSET_GATE,
        CASTLE_GATE,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    assert!(
        after,
        "the granted address must now pass the dial gate -- if this fails the \
         mission-708 dial step is still unreachable",
    );
    assert!(
        mgr.gate_dial(PLAYER_EID).is_some(),
        "and the dial must actually arm",
    );
}
