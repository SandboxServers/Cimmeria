use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use cimmeria_entity::inventory::INV_MAIN;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

/// Re-assert the document-order count of `<Exposed/>` CellMethods in
/// `SGWGmPlayer.def`. The first own exposed method (gmMissionAssign, def
/// line 65) is index 109; counting forward in document order — skipping
/// `gmSetCallback` (def line 312, which has NO `<Exposed/>`) — lands each
/// implemented method at the constant below. If this drifts, the client's
/// method table and our dispatch disagree and gm* commands silently route
/// to the wrong handler. The three pcap-anchored DONE indices
/// (133/163/190) are the alignment proof for the whole tail.
#[test]
fn gm_indices_match_def_document_order() {
    // index = 109 + zero-based document-order position among exposed methods.
    assert_eq!(GM_MISSION_CLEAR, 109 + 1, "gmMissionClear (def line 71)");
    assert_eq!(
        GM_MISSION_ADVANCE,
        109 + 7,
        "gmMissionAdvance (def line 97)"
    );
    assert_eq!(
        GM_MISSION_ABANDON,
        109 + 11,
        "gmMissionAbandon (def line 120)"
    );
    assert_eq!(GM_GIVE_XP, 109 + 23, "gmGiveXp (def line 180)");
    assert_eq!(GM_GIVE_ITEM, 109 + 24, "gmGiveItem (def line 185)");
    assert_eq!(GM_GIVE_CASH, 109 + 25, "gmGiveCash (def line 191)");
    assert_eq!(GM_REMOVE_ITEM, 109 + 26, "gmRemoveItem (def line 196)");
    assert_eq!(GM_SET_HEALTH, 109 + 38, "gmSetHealth (def line 259)");
    assert_eq!(GM_SET_HEALTH_MAX, 109 + 39, "gmSetHealthMax (def line 265)");
    assert_eq!(GM_SET_FOCUS, 109 + 40, "gmSetFocus (def line 271)");
    assert_eq!(GM_SET_FOCUS_MAX, 109 + 41, "gmSetFocusMax (def line 277)");
    assert_eq!(GM_SET_TARGET, 109 + 47, "gmSetTarget (def line 302)");
    assert_eq!(GM_DHD, 109 + 50, "gmDHD (def line 325)");
    assert_eq!(GM_GOTO_LOCATION, 109 + 53, "gmGotoLocation (def line 340)");
    assert_eq!(GM_GOTO_XYZ, 109 + 54, "gmGotoXYZ (def line 348)");
    assert_eq!(GM_DESPAWN_BY_CMD, 109 + 77, "gmDespawnByCmd (def line 461)");
    assert_eq!(GM_RESPAWN, 109 + 80, "gmRespawn (def line 478)");
    assert_eq!(GM_KILL_TARGET, 109 + 81, "gmKillTarget (def line 482)");
    assert_eq!(DESPAWN_MOB, 109 + 104, "despawnMob (def line 605)");
}

/// All implemented indices sit in the GM tail (109 or above), so the
/// dispatch-layer gate (`gm_gate::requires_gm`, which gates every index from
/// 109 up) covers them. A constant that slipped below 109 would be reachable
/// by a non-GM — this pins the invariant.
#[test]
fn implemented_indices_are_in_gm_tail() {
    const GM_TAIL_BASE: u16 = 109;
    for idx in [
        GM_MISSION_CLEAR,
        GM_MISSION_ADVANCE,
        GM_MISSION_ABANDON,
        GM_GIVE_XP,
        GM_GIVE_ITEM,
        GM_GIVE_CASH,
        GM_REMOVE_ITEM,
        GM_SET_HEALTH,
        GM_SET_HEALTH_MAX,
        GM_SET_FOCUS,
        GM_SET_FOCUS_MAX,
        GM_SET_TARGET,
        GM_DHD,
        GM_GOTO_LOCATION,
        GM_GOTO_XYZ,
        GM_DESPAWN_BY_CMD,
        GM_RESPAWN,
        GM_KILL_TARGET,
        DESPAWN_MOB,
    ] {
        assert!(
            idx >= GM_TAIL_BASE,
            "implemented gm* index {idx} must be in the GM-gated tail (>= 109)"
        );
    }
}

fn mgr_with_player(eid: u32, world: &str) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = format!(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="{world}" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#
    );
    mgr.parse_spaces_xml(&xml).unwrap();
    mgr.create_startup_spaces(&format!(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="{world}" /></Spaces>"#
    ))
    .unwrap();
    mgr.create_entity(eid, world, [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(eid) {
        e.is_player = true;
        e.player_id = Some(100);
        e.access_level = 2; // GameMaster
    }
    mgr
}

/// Drain all currently-queued messages from the channel.
fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn write_wstring_arg(buf: &mut Vec<u8>, s: &str) {
    crate::mercury::write_wstring(buf, s);
}

// ── give / remove ────────────────────────────────────────────────────────────

fn give_item_args(design_id: &str, qty: i32) -> Vec<u8> {
    let mut args = Vec::new();
    write_wstring_arg(&mut args, design_id);
    args.extend_from_slice(&qty.to_le_bytes());
    args
}

#[tokio::test]
async fn gm_give_item_emits_grant_with_clamped_qty() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    // Request 5000 — must clamp to the cap (1000).
    let args = give_item_args("1234", 5000);
    assert!(dispatch(1, GM_GIVE_ITEM, &args, &tx, &mut mgr).await);

    match rx.try_recv().expect("gmGiveItem must emit GrantItem") {
        CellToBaseMsg::GrantItem {
            entity_id,
            player_id,
            item_id,
            container_id,
            count,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(player_id, 100);
            assert_eq!(item_id, 1234);
            assert_eq!(container_id, INV_MAIN);
            assert_eq!(count, 1000, "quantity must clamp to the cap");
        }
        other => panic!("expected GrantItem, got {other:?}"),
    }
}

#[tokio::test]
async fn gm_give_item_rejects_non_numeric_and_nonpositive_qty() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    let args = give_item_args("AmberVial", 1);
    assert!(dispatch(1, GM_GIVE_ITEM, &args, &tx, &mut mgr).await);
    assert!(
        rx.try_recv().is_err(),
        "non-numeric DesignId must not grant"
    );

    let args = give_item_args("1234", 0);
    assert!(dispatch(1, GM_GIVE_ITEM, &args, &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err(), "quantity 0 must not grant");
}

#[tokio::test]
async fn gm_give_xp_emits_grant_and_rejects_nonpositive() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    assert!(dispatch(1, GM_GIVE_XP, &500i32.to_le_bytes(), &tx, &mut mgr).await);
    match rx.try_recv().expect("gmGiveXp must emit GrantXP") {
        CellToBaseMsg::GrantXP {
            entity_id,
            xp_amount,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(xp_amount, 500);
        }
        other => panic!("expected GrantXP, got {other:?}"),
    }

    // Non-positive must not grant (a negative i32 → u64 would be absurd).
    assert!(dispatch(1, GM_GIVE_XP, &(-5i32).to_le_bytes(), &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err(), "negative XP must not grant");
}

#[tokio::test]
async fn gm_give_cash_emits_grant_and_rejects_nonpositive() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    assert!(dispatch(1, GM_GIVE_CASH, &250i32.to_le_bytes(), &tx, &mut mgr).await);
    match rx.try_recv().expect("gmGiveCash must emit GrantCash") {
        CellToBaseMsg::GrantCash {
            entity_id,
            player_id,
            amount,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(player_id, 100);
            assert_eq!(amount, 250);
        }
        other => panic!("expected GrantCash, got {other:?}"),
    }

    assert!(dispatch(1, GM_GIVE_CASH, &0i32.to_le_bytes(), &tx, &mut mgr).await);
    assert!(
        rx.try_recv().is_err(),
        "zero cash must not grant (give is additive-only)"
    );
}

#[tokio::test]
async fn gm_remove_item_emits_remove_and_rejects_nonpositive() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    // ItemID (INT32) + quantity (INT16).
    let mut args = 42i32.to_le_bytes().to_vec();
    args.extend_from_slice(&3i16.to_le_bytes());
    assert!(dispatch(1, GM_REMOVE_ITEM, &args, &tx, &mut mgr).await);
    match rx
        .try_recv()
        .expect("gmRemoveItem must emit RemoveInventoryItem")
    {
        CellToBaseMsg::RemoveInventoryItem {
            entity_id,
            player_id,
            item_id,
            quantity,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(player_id, 100);
            assert_eq!(item_id, 42);
            assert_eq!(quantity, 3);
        }
        other => panic!("expected RemoveInventoryItem, got {other:?}"),
    }

    // Non-positive quantity must not remove.
    let mut args = 42i32.to_le_bytes().to_vec();
    args.extend_from_slice(&0i16.to_le_bytes());
    assert!(dispatch(1, GM_REMOVE_ITEM, &args, &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err(), "quantity 0 must not remove");
}

// ── stats ────────────────────────────────────────────────────────────────────

/// Build `(INT32 Amount, INT64 TargetId)`.
fn set_stat_args(amount: i32, target: i64) -> Vec<u8> {
    let mut args = amount.to_le_bytes().to_vec();
    args.extend_from_slice(&target.to_le_bytes());
    args
}

#[tokio::test]
async fn gm_set_health_self_mutates_stat_and_emits_update() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    // Target 0 → self. Default HEALTH max is 100; set to 75.
    assert!(dispatch(1, GM_SET_HEALTH, &set_stat_args(75, 0), &tx, &mut mgr).await);

    assert_eq!(
        mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
        75,
        "health current must be set"
    );
    let msgs = drain(&mut rx);
    assert!(
        msgs.iter()
            .any(|m| matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: 1, .. })),
        "gmSetHealth must emit an onStatUpdate to the player"
    );
}

#[tokio::test]
async fn gm_set_health_rejects_negative() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    assert!(dispatch(1, GM_SET_HEALTH, &set_stat_args(-10, 0), &tx, &mut mgr).await);
    assert_eq!(
        mgr.get_entity(1).unwrap().stats.get(HEALTH).unwrap().cur,
        100,
        "negative amount must not mutate"
    );
    assert!(
        drain(&mut rx).is_empty(),
        "negative amount must emit nothing"
    );
}

#[tokio::test]
async fn gm_set_health_cross_space_target_refused() {
    let mut mgr = mgr_with_player(1, "Castle");
    // NPC in a *different* space.
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Other" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Other" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(2, "Other", [0.0; 3], [0.0; 3]).unwrap();
    let (tx, mut rx) = mpsc::channel(8);

    assert!(dispatch(1, GM_SET_HEALTH, &set_stat_args(50, 2), &tx, &mut mgr).await);
    assert!(
        drain(&mut rx).is_empty(),
        "cross-space target must be refused"
    );
}

// ── missions ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn gm_mission_clear_rejects_non_numeric_design_id() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "FindAmbernol");
    assert!(dispatch(1, GM_MISSION_CLEAR, &args, &tx, &mut mgr).await);
    assert!(
        rx.try_recv().is_err(),
        "non-numeric DesignID must not emit a mission update"
    );
}

#[tokio::test]
async fn gm_mission_advance_truncated_step_is_noop() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    // Numeric DesignID but missing the INT32 step → no panic, no emit.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "1001");
    assert!(dispatch(1, GM_MISSION_ADVANCE, &args, &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err(), "missing step must not advance");
}

// ── travel ────────────────────────────────────────────────────────────────────

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

// ── world / entity ops ─────────────────────────────────────────────────────────

#[tokio::test]
async fn gm_kill_target_kills_npc_in_same_space() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    let (tx, mut _rx) = mpsc::channel(32);

    assert!(dispatch(1, GM_KILL_TARGET, &2i64.to_le_bytes(), &tx, &mut mgr).await);
    let npc = mgr.get_entity(2).unwrap();
    assert!(
        crate::cell::combat::is_dead_state(npc.state_field),
        "gmKillTarget must mark the NPC dead"
    );
}

#[tokio::test]
async fn gm_kill_target_refuses_player() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(2) {
        e.is_player = true;
        e.player_id = Some(200);
    }
    let (tx, mut _rx) = mpsc::channel(32);

    assert!(dispatch(1, GM_KILL_TARGET, &2i64.to_le_bytes(), &tx, &mut mgr).await);
    let victim = mgr.get_entity(2).unwrap();
    assert!(
        !crate::cell::combat::is_dead_state(victim.state_field),
        "gmKillTarget must refuse a player target"
    );
}

#[tokio::test]
async fn gm_despawn_removes_npc_but_refuses_player() {
    let mut mgr = mgr_with_player(1, "Castle");
    // NPC at 2, player at 3.
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(3, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(3) {
        e.is_player = true;
        e.player_id = Some(300);
    }
    let (tx, _rx) = mpsc::channel(8);

    // Player target refused.
    assert!(dispatch(1, GM_DESPAWN_BY_CMD, &3i32.to_le_bytes(), &tx, &mut mgr).await);
    assert!(
        mgr.get_entity(3).is_some(),
        "gmDespawn must refuse a player target"
    );

    // NPC despawned (despawnMob alias hits the same handler).
    assert!(dispatch(1, DESPAWN_MOB, &2i32.to_le_bytes(), &tx, &mut mgr).await);
    assert!(
        mgr.get_entity(2).is_none(),
        "despawnMob must remove the NPC"
    );
}

#[tokio::test]
async fn gm_respawn_requires_player_id() {
    let mut mgr = mgr_with_player(1, "Castle");
    // Strip the player_id so the caller looks like an NPC.
    mgr.get_entity_mut(1).unwrap().player_id = None;
    let (tx, mut rx) = mpsc::channel(16);

    assert!(dispatch(1, GM_RESPAWN, &[], &tx, &mut mgr).await);
    assert!(
        rx.try_recv().is_err(),
        "gmRespawn must no-op for a caller with no player_id"
    );
}

#[tokio::test]
async fn gm_respawn_runs_for_player() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(16);

    assert!(dispatch(1, GM_RESPAWN, &[], &tx, &mut mgr).await);
    // The respawn sequence always opens by closing the Defeat Window, so at
    // least one message must have been emitted.
    assert!(
        !drain(&mut rx).is_empty(),
        "gmRespawn must drive the respawn sequence for a player"
    );
}

#[tokio::test]
async fn gm_set_target_sets_and_clears() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    // Numeric id form: set target 42.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "42");
    assert!(dispatch(1, GM_SET_TARGET, &args, &tx, &mut mgr).await);
    assert_eq!(mgr.get_entity(1).unwrap().current_target_id, Some(42));
    assert!(
        drain(&mut rx)
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: 1, .. })),
        "gmSetTarget must emit onTargetUpdate to the owner"
    );

    // "0" clears the target.
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "0");
    assert!(dispatch(1, GM_SET_TARGET, &args, &tx, &mut mgr).await);
    assert_eq!(
        mgr.get_entity(1).unwrap().current_target_id,
        None,
        "gmSetTarget(0) must clear the target"
    );
}

/// An unimplemented 109+ index returns `false` so the router falls through to
/// its (already-authorized) warn arm — no panic.
#[tokio::test]
async fn unimplemented_gm_index_returns_false() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, _rx) = mpsc::channel(8);
    // 142 = gmSetGodMode — in the tail, not implemented here.
    assert!(!dispatch(1, 142, &[], &tx, &mut mgr).await);
}
