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
    // Cell-local success: feedback states the destination after the action.
    let fb = feedback_text(&drain(&mut rx), 1).expect("gmGotoXYZ success must feed back");
    assert!(
        fb.contains("teleported"),
        "gmGotoXYZ feedback must report the teleport, got: {fb}"
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
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::TeleportPlayer { .. })),
        "NaN coordinate must not teleport"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "NaN coordinate must feed back a rejection"
    );
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
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::GateTravel { .. })),
        "empty world must not GateTravel"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "empty world must feed back a rejection"
    );
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
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::GateTravel { .. })),
        "address 0 must not dial"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "address 0 must feed back a rejection"
    );
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
    let msgs = drain(&mut rx);
    assert!(
        !msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::TeleportPlayer { .. } | CellToBaseMsg::GateTravel { .. }
        )),
        "truncated travel args must not emit a teleport/travel action"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "truncated travel args must feed back a rejection"
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
async fn summon_player_snaps_target_via_teleport() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.get_entity_mut(1).unwrap().position = Vector3 {
        x: 7.0,
        y: 0.0,
        z: 8.0,
    };
    // A second *player* target (not an NPC) at a distinct position.
    mgr.create_entity(2, "Castle", [100.0, 0.0, 100.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(2).unwrap().is_player = true;
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "2");
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr).await);

    // Grid position moved to the caller…
    let p = mgr.get_entity(2).unwrap().position;
    assert_eq!(
        [p.x, p.y, p.z],
        [7.0, 0.0, 8.0],
        "player must move to caller"
    );
    // …and a player target needs the authoritative TeleportPlayer snap to id 2.
    let teleport = drain(&mut rx).into_iter().find_map(|m| match m {
        CellToBaseMsg::TeleportPlayer {
            entity_id: 2,
            position,
            ..
        } => Some(position),
        _ => None,
    });
    assert_eq!(
        teleport,
        Some([7.0, 0.0, 8.0]),
        "summoning a PLAYER must emit TeleportPlayer for the target at the caller's position"
    );
}

#[tokio::test]
async fn summon_missing_and_cross_space_refused() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.get_entity_mut(1).unwrap().position = Vector3 {
        x: 7.0,
        y: 0.0,
        z: 8.0,
    };
    // A target in a *different* space.
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Other" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Other" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(2, "Other", [50.0, 0.0, 60.0], [0.0; 3])
        .unwrap();
    let (tx, mut rx) = mpsc::channel(8);

    // Cross-space summon: refused — no teleport, target unmoved.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "2");
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::TeleportPlayer { .. })),
        "cross-space summon must not teleport"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "cross-space summon must feed back a rejection"
    );
    let p = mgr.get_entity(2).unwrap().position;
    assert_eq!(
        [p.x, p.y, p.z],
        [50.0, 0.0, 60.0],
        "cross-space target must be left unmoved"
    );

    // Missing target id: refused — no teleport.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "4242");
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::TeleportPlayer { .. })),
        "summon of a missing id must not teleport"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "summon of a missing id must feed back a rejection"
    );
}

#[tokio::test]
async fn goto_missing_and_cross_space_refused() {
    let mut mgr = mgr_with_player(1, "Castle");
    let caller_start = mgr.get_entity(1).unwrap().position;
    // A target in a different space.
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Other" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Other" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(2, "Other", [50.0, 0.0, 60.0], [0.0; 3])
        .unwrap();
    let (tx, mut rx) = mpsc::channel(8);

    // Cross-space goto: refused — caller not moved, no teleport.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "2");
    assert!(dispatch(1, GM_GOTO, &args, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::TeleportPlayer { .. })),
        "cross-space goto must not teleport"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "cross-space goto must feed back a rejection"
    );

    // Missing target id: refused.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "4242");
    assert!(dispatch(1, GM_GOTO, &args, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::TeleportPlayer { .. })),
        "goto of a missing id must not teleport"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "goto of a missing id must feed back a rejection"
    );

    let p = mgr.get_entity(1).unwrap().position;
    assert_eq!(
        [p.x, p.y, p.z],
        [caller_start.x, caller_start.y, caller_start.z],
        "a refused goto must leave the caller unmoved"
    );
}

/// G1 witness-visibility guard: a summoned idle NPC must be broadcast to the
/// caller-witness at the caller's position on the next AoI tick. The NPC is
/// first brought into the witness set (tick 1 → EnteredAoI), then summoned to
/// the caller, then tick 2 must emit `EntityMoved` for the NPC to the caller
/// carrying the caller's position. Without the summon's grid-position update,
/// the EntityMoved would carry the NPC's old spawn position — so this pins both
/// that the NPC stays in AoI and that it's broadcast at the new spot.
#[tokio::test]
async fn summoned_npc_is_broadcast_to_caller_witness() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.connect_entity(1); // AoI walks players; ensure the caller is tracked
    mgr.get_entity_mut(1).unwrap().position = Vector3 {
        x: 5.0,
        y: 0.0,
        z: 5.0,
    };
    // NPC spawned within the caller's AoI radius so tick 1 makes it a witness.
    mgr.spawn_npc(50, "Castle", [6.0, 0.0, 6.0], [0.0; 3])
        .unwrap();

    // Tick 1: NPC enters AoI (becomes a witness of the caller).
    let _ = mgr.compute_aoi_changes();

    // Summon the NPC to the caller.
    let (tx, _rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "50");
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr).await);

    // Tick 2: NPC is still in AoI → EntityMoved to the caller with the caller's pos.
    let moved = mgr.compute_aoi_changes().into_iter().find_map(|m| match m {
        CellToBaseMsg::EntityMoved {
            witness_id: 1,
            entity_id: 50,
            position,
            ..
        } => Some(position),
        _ => None,
    });
    assert_eq!(
        moved,
        Some([5.0, 0.0, 5.0]),
        "summoned NPC must be broadcast to the caller-witness at the caller's position"
    );
}

#[tokio::test]
async fn goto_summon_reject_non_numeric() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "SomeName");
    assert!(dispatch(1, GM_GOTO, &args, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::TeleportPlayer { .. })),
        "non-numeric gmGoto must not teleport"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "non-numeric gmGoto must feed back a rejection"
    );
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "SomeName");
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::TeleportPlayer { .. })),
        "non-numeric gmSummon must not teleport"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "non-numeric gmSummon must feed back a rejection"
    );
}
