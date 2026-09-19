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
    assert!(dispatch(1, GM_GOTO_XYZ, &args, &tx, &mut mgr, &test_engine()).await);

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
    assert!(dispatch(1, GM_GOTO_XYZ, &args, &tx, &mut mgr, &test_engine()).await);
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
    assert!(dispatch(1, GM_GOTO_LOCATION, &args, &tx, &mut mgr, &test_engine()).await);

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
    assert!(dispatch(1, GM_GOTO_LOCATION, &args, &tx, &mut mgr, &test_engine()).await);
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
    assert!(dispatch(1, GM_DHD, &[0u8], &tx, &mut mgr, &test_engine()).await);
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
    assert!(dispatch(1, GM_GOTO_XYZ, &[], &tx, &mut mgr, &test_engine()).await);
    assert!(dispatch(1, GM_GOTO_LOCATION, &[], &tx, &mut mgr, &test_engine()).await);
    assert!(dispatch(1, GM_DHD, &[], &tx, &mut mgr, &test_engine()).await);
    // goto_location with a world name but no coords.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "Abydos");
    assert!(dispatch(1, GM_GOTO_LOCATION, &args, &tx, &mut mgr, &test_engine()).await);
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
    assert!(dispatch(1, GM_GOTO, &args, &tx, &mut mgr, &test_engine()).await);
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
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr, &test_engine()).await);
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
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr, &test_engine()).await);

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
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr, &test_engine()).await);
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
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr, &test_engine()).await);
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
    assert!(dispatch(1, GM_GOTO, &args, &tx, &mut mgr, &test_engine()).await);
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
    assert!(dispatch(1, GM_GOTO, &args, &tx, &mut mgr, &test_engine()).await);
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
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr, &test_engine()).await);

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

/// Facing-preservation regression guard for the **native** `gm*` travel
/// handlers.
///
/// `SpaceManager::update_entity_position` writes `direction` unconditionally
/// from its `[i8; 3]` parameter, so every handler that passed `[0, 0, 0]` to
/// move an entity also silently snapped that entity's facing to north. The
/// dot-command travel path worked around that by hand-restoring the captured
/// direction; these native handlers never did, so a GM using the client's own
/// `gmGotoXYZ` / `gmGoto` / `gmSummon` was re-faced on every teleport. The
/// fix is `update_position_preserving_facing`, which never writes `direction`
/// at all.
///
/// Reverting any of the three handlers to `update_entity_position(…, [0, 0,
/// 0], …)` zeroes the facing asserted below and this guard fires. The
/// non-zero, non-uniform facing is deliberate: a `[0, 0, 0]` or symmetric
/// value would still match after the bug was reintroduced.
#[tokio::test]
async fn native_gm_travel_preserves_facing() {
    const FACING: Vector3 = Vector3 {
        x: 0.0,
        y: 137.0,
        z: 0.0,
    };

    // ── gmGotoXYZ: the caller moves itself ──────────────────────────────
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.get_entity_mut(1).unwrap().direction = FACING;
    let (tx, _rx) = mpsc::channel(8);
    let mut args = Vec::new();
    for c in [10.0f32, 20.0, 30.0] {
        args.extend_from_slice(&c.to_le_bytes());
    }
    assert!(dispatch(1, GM_GOTO_XYZ, &args, &tx, &mut mgr, &test_engine()).await);
    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        [e.position.x, e.position.y, e.position.z],
        [10.0, 20.0, 30.0],
        "precondition: gmGotoXYZ must actually have moved the caller"
    );
    assert_eq!(
        e.direction, FACING,
        "gmGotoXYZ must not re-face the GM it teleports"
    );

    // ── gmGoto: the caller moves itself to another entity ───────────────
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.get_entity_mut(1).unwrap().direction = FACING;
    mgr.create_entity(2, "Castle", [50.0, 0.0, 60.0], [0.0; 3])
        .unwrap();
    let (tx, _rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "2");
    assert!(dispatch(1, GM_GOTO, &args, &tx, &mut mgr, &test_engine()).await);
    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        [e.position.x, e.position.y, e.position.z],
        [50.0, 0.0, 60.0],
        "precondition: gmGoto must actually have moved the caller"
    );
    assert_eq!(
        e.direction, FACING,
        "gmGoto must not re-face the GM it teleports"
    );

    // ── gmSummon: somebody *else* is moved — their facing is the one at
    // risk, and it is the one a witness renders.
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.get_entity_mut(1).unwrap().position = Vector3 {
        x: 7.0,
        y: 0.0,
        z: 8.0,
    };
    mgr.spawn_npc(50, "Castle", [100.0, 0.0, 100.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(50).unwrap().direction = FACING;
    let (tx, _rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "50");
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr, &test_engine()).await);
    let e = mgr.get_entity(50).unwrap();
    assert_eq!(
        [e.position.x, e.position.y, e.position.z],
        [7.0, 0.0, 8.0],
        "precondition: gmSummon must actually have moved the target"
    );
    assert_eq!(
        e.direction, FACING,
        "gmSummon must not re-face the entity it summons"
    );
}

#[tokio::test]
async fn goto_summon_reject_non_numeric() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "SomeName");
    assert!(dispatch(1, GM_GOTO, &args, &tx, &mut mgr, &test_engine()).await);
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
    assert!(dispatch(1, GM_SUMMON, &args, &tx, &mut mgr, &test_engine()).await);
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

/// CA10 changed what `gmDHD` does. It routes through `handle_dial_gate`,
/// so on a world with a `REGION_FLAG_Stargate` volume it now ARMS a dial
/// — the GM has to walk into the gate — while on a world without one it
/// still travels on the dial. A GM who expects the old instant warp needs
/// this difference to be deliberate and pinned, not discovered in-game.
///
/// Reverting the arm branch makes the first half emit `GateTravel`
/// immediately and leaves `gate_dial(1)` empty.
#[tokio::test]
async fn gm_dhd_arms_a_dial_where_a_gate_volume_exists_and_travels_where_none_does() {
    use crate::cell::space_manager::{RegionData, REGION_FLAG_CLIENT_HINTED, REGION_FLAG_STARGATE};
    use crate::cell::spawner::StargateEntry;

    const DEST_ADDR: i32 = 3;

    // Castle is the GM's world; the gate they dial leads elsewhere.
    fn mgr_with_destination() -> SpaceManager {
        let mut mgr = mgr_with_player(1, "Castle");
        mgr.stargates.insert(
            DEST_ADDR,
            StargateEntry {
                world_name: "Harset".to_string(),
                x: 1.0,
                y: 2.0,
                z: 3.0,
                yaw: 0.0,
                address_origin: 18,
                arrival: None,
                event_set_id: None,
            },
        );
        mgr
    }

    // ── with a gate volume: arms, does not travel ──
    let mut mgr = mgr_with_destination();
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

    let (tx, mut rx) = mpsc::channel(16);
    assert!(dispatch(1, GM_DHD, &[DEST_ADDR as u8], &tx, &mut mgr, &test_engine()).await);

    let msgs = drain(&mut rx);
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::GateTravel { .. })),
        "with a gate volume, gmDHD must arm the dial rather than warp"
    );
    assert!(
        mgr.get_entity(1).is_some(),
        "arming must not tear the GM out of their space"
    );
    let dial = mgr.gate_dial(1).expect("gmDHD must arm a dial");
    assert_eq!(dial.target_address_id, DEST_ADDR);
    assert_eq!(dial.target_world_name, "Harset");

    // ── without a gate volume: travels on the dial, as before ──
    let mut mgr = mgr_with_destination();
    let (tx, mut rx) = mpsc::channel(16);
    assert!(dispatch(1, GM_DHD, &[DEST_ADDR as u8], &tx, &mut mgr, &test_engine()).await);

    let msgs = drain(&mut rx);
    let travelled = msgs.iter().any(|m| {
        matches!(m, CellToBaseMsg::GateTravel { target_world_name, .. }
            if target_world_name == "Harset")
    });
    assert!(
        travelled,
        "with no gate volume to walk into, gmDHD must still travel on the \
         dial — otherwise a GM on those worlds can never leave. Got {msgs:?}"
    );
    assert!(mgr.gate_dial(1).is_none(), "the fallback arms nothing");
}

/// `gmDHD` reaches `handle_dial_gate`, which enforces the caller's address
/// book (CAT-O-01). A GM debugging a world they have never visited does not
/// hold its address, so the arm grants it for the session first — without
/// that, H06's dial gate silently broke a GM command.
///
/// Deleting the grant block in `handle_dhd` fails this: the dial is refused,
/// no `GateTravel` is emitted and the feedback reports a refusal.
#[tokio::test]
async fn gm_dhd_grants_the_address_it_needs_and_dials() {
    use crate::cell::spawner::StargateEntry;

    const DEST_ADDR: i32 = 7;

    // Castle has no `REGION_FLAG_Stargate` volume in this fixture, so the
    // dial takes the CA10 immediate-travel fallback and the `GateTravel`
    // below is observable in one call.
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.stargates.insert(
        DEST_ADDR,
        StargateEntry {
            world_name: "Agnos".to_string(),
            x: 1.0,
            y: 2.0,
            z: 3.0,
            yaw: 0.0,
            address_origin: 4,
            arrival: None,
            event_set_id: None,
        },
    );
    assert!(
        mgr.get_entity(1).unwrap().known_stargates.is_empty(),
        "the GM starts without the address — that is the point of the test"
    );

    let (tx, mut rx) = mpsc::channel(16);
    assert!(dispatch(1, GM_DHD, &[DEST_ADDR as u8], &tx, &mut mgr, &test_engine()).await);

    let msgs = drain(&mut rx);
    assert!(
        msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::GateTravel { target_world_name, .. } if target_world_name == "Agnos"
        )),
        "a GM dial must not be blocked by the player-facing address book. Got {msgs:?}"
    );
}
