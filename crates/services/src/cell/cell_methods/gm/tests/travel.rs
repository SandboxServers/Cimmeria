use super::super::*; // gm module: dispatch + GM_* constants
use super::*; // shared helpers from tests/mod.rs
use crate::cell::messages::CellToBaseMsg;
use cimmeria_common::Vector3;
use tokio::sync::mpsc;

#[tokio::test]
async fn gm_goto_xyz_updates_position_and_emits_teleport() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    let mut args = Vec::new();
    for c in [10.0f32, 20.0, 30.0] {
        args.extend_from_slice(&c.to_le_bytes());
    }
    assert!(dispatch(1, GM_GOTO_XYZ, &args, &tx, &mut mgr).await);

    match rx.try_recv().expect("gmGotoXYZ must emit TeleportPlayer") {
        CellToBaseMsg::TeleportPlayer {
            entity_id,
            position,
            prev_pos,
            ..
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(position, [10.0, 20.0, 30.0]);
            assert_eq!(prev_pos, [0.0, 0.0, 0.0], "prev_pos is the spawn origin");
        }
        other => panic!("expected TeleportPlayer, got {other:?}"),
    }
    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        [e.position.x, e.position.y, e.position.z],
        [10.0, 20.0, 30.0]
    );
}

#[tokio::test]
async fn gm_goto_xyz_rejects_non_finite() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    let mut args = Vec::new();
    args.extend_from_slice(&f32::NAN.to_le_bytes());
    args.extend_from_slice(&0.0f32.to_le_bytes());
    args.extend_from_slice(&0.0f32.to_le_bytes());
    assert!(dispatch(1, GM_GOTO_XYZ, &args, &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err(), "NaN coordinate must not teleport");
}

#[tokio::test]
async fn gm_goto_location_emits_gate_travel_and_destroys_entity() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    let mut args = Vec::new();
    write_wstring_arg(&mut args, "Abydos");
    for c in [1.0f32, 2.0, 3.0] {
        args.extend_from_slice(&c.to_le_bytes());
    }
    assert!(dispatch(1, GM_GOTO_LOCATION, &args, &tx, &mut mgr).await);

    match rx.try_recv().expect("gmGotoLocation must emit GateTravel") {
        CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name,
            position,
            ..
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(target_world_name, "Abydos");
            assert_eq!(position, [1.0, 2.0, 3.0]);
        }
        other => panic!("expected GateTravel, got {other:?}"),
    }
    assert!(
        mgr.get_entity(1).is_none(),
        "entity must be torn out of the space before GateTravel"
    );
}

#[tokio::test]
async fn gm_goto_location_rejects_empty_world() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "");
    for c in [1.0f32, 2.0, 3.0] {
        args.extend_from_slice(&c.to_le_bytes());
    }
    assert!(dispatch(1, GM_GOTO_LOCATION, &args, &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err(), "empty world must not GateTravel");
    assert!(
        mgr.get_entity(1).is_some(),
        "entity must survive a rejected goto"
    );
}

#[tokio::test]
async fn gm_dhd_list_request_is_noop() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    // Address 0 = "request list" — unsupported without a feedback channel.
    assert!(dispatch(1, GM_DHD, &[0u8], &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err(), "address 0 must not dial");
}

#[tokio::test]
async fn travel_handlers_reject_truncated_args() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    assert!(dispatch(1, GM_GOTO_XYZ, &[], &tx, &mut mgr).await);
    assert!(dispatch(1, GM_GOTO_LOCATION, &[], &tx, &mut mgr).await);
    assert!(dispatch(1, GM_DHD, &[], &tx, &mut mgr).await);
    // goto_location with a world name but no coords.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "Abydos");
    assert!(dispatch(1, GM_GOTO_LOCATION, &args, &tx, &mut mgr).await);
    assert!(
        drain(&mut rx).is_empty(),
        "truncated travel args must emit nothing"
    );
    assert!(
        mgr.get_entity(1).is_some(),
        "rejected travel must not destroy the caller"
    );
}

#[tokio::test]
async fn goto_teleports_caller_to_target() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.create_entity(2, "Castle", [50.0, 0.0, 60.0], [0.0; 3])
        .unwrap();
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "2");
    assert!(dispatch(1, GM_GOTO, &args, &tx, &mut mgr).await);
    // Caller's grid position moved to the target.
    let p = mgr.get_entity(1).unwrap().position;
    assert_eq!([p.x, p.y, p.z], [50.0, 0.0, 60.0]);
    assert!(
        drain(&mut rx)
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::TeleportPlayer { entity_id: 1, .. })),
        "gmGoto must snap the caller via TeleportPlayer"
    );
}

#[tokio::test]
async fn summon_moves_npc_to_caller() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.get_entity_mut(1).unwrap().position = Vector3 {
        x: 7.0,
        y: 0.0,
        z: 8.0,
    };
    mgr.spawn_npc(50, "Castle", [100.0, 0.0, 100.0], [0.0; 3])
        .unwrap();
    let (tx, _rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "50");
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr).await);
    let p = mgr.get_entity(50).unwrap().position;
    assert_eq!(
        [p.x, p.y, p.z],
        [7.0, 0.0, 8.0],
        "NPC must be moved to the caller"
    );
}

#[tokio::test]
async fn goto_summon_reject_non_numeric() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "SomeName");
    assert!(dispatch(1, GM_GOTO, &args, &tx, &mut mgr).await);
    assert!(drain(&mut rx).is_empty(), "non-numeric gmGoto must no-op");
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "SomeName");
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr).await);
    assert!(drain(&mut rx).is_empty(), "non-numeric gmSummon must no-op");
}
