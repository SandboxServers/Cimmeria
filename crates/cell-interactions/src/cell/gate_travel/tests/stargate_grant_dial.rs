//! A granted stargate address is what turns a refused Harset dial into an
//! accepted one (Harset H55).
//!
//! Cut from `cimmeria-cell-content`'s `executor::tests::stargate` in wave C3
//! of the services crate split (docs/architecture/services-crate-split.md):
//! it drives the gate dial, `cell::gate_travel::handle_dial_gate`, which sits
//! above the content crate. It stayed in `cimmeria-services` as
//! `cell::content_tests::stargate_grant_dial` until wave C4 moved the gate
//! dial here. `make_two_world_mgr`, `grant` and `drain` are copies of that
//! file's; `execute_actions` is the content crate's `test-support` hook.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use tokio::sync::mpsc;

use crate::cell::client_methods::player::ON_ERROR_CODE;
use crate::cell::content::execute_actions;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
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

/// The packet's reason for existing, end to end.
///
/// Before the grant, `handle_dial_gate` refuses (H06's CAT-O-01 gate),
/// tells the client with a feedback line and `onErrorCode` feedback 180
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
    // #727: the text line (method 28) first, then the address-book
    // `onErrorCode`.
    assert_eq!(refusal.len(), 2, "exactly the feedback; got {refusal:?}");
    assert!(
        matches!(
            &refusal[0],
            CellToBaseMsg::EntityMethodCall {
                method_index: 28,
                ..
            }
        ),
        "the refusal's text line comes first; got {refusal:?}"
    );
    match &refusal[1] {
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
