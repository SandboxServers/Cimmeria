use super::super::*; // gm module: dispatch + GM_* constants
use super::*; // shared helpers from tests/mod.rs
use crate::cell::messages::CellToBaseMsg;
use tokio::sync::mpsc;

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

#[tokio::test]
async fn kill_target_rejects_bad_ids_and_missing_target() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut _rx) = mpsc::channel(8);
    // Truncated INT64.
    assert!(dispatch(1, GM_KILL_TARGET, &[], &tx, &mut mgr).await);
    // Out-of-u32-range target.
    assert!(dispatch(1, GM_KILL_TARGET, &(i64::MAX).to_le_bytes(), &tx, &mut mgr).await);
    // Well-formed but nonexistent target — no panic.
    assert!(dispatch(1, GM_KILL_TARGET, &4242i64.to_le_bytes(), &tx, &mut mgr).await);
}

#[tokio::test]
async fn despawn_rejects_truncated_invalid_and_missing() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, _rx) = mpsc::channel(8);
    assert!(dispatch(1, GM_DESPAWN_BY_CMD, &[], &tx, &mut mgr).await);
    assert!(dispatch(1, GM_DESPAWN_BY_CMD, &0i32.to_le_bytes(), &tx, &mut mgr).await);
    assert!(dispatch(1, GM_DESPAWN_BY_CMD, &4242i32.to_le_bytes(), &tx, &mut mgr).await);
}

#[tokio::test]
async fn set_target_rejects_malformed_and_non_numeric() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    // Empty args → WSTRING parse fails.
    assert!(dispatch(1, GM_SET_TARGET, &[], &tx, &mut mgr).await);
    // Non-numeric name (no name→id resolution in the cell).
    let mut args = Vec::new();
    write_wstring_arg(&mut args, "SomeMobName");
    assert!(dispatch(1, GM_SET_TARGET, &args, &tx, &mut mgr).await);
    assert_eq!(
        mgr.get_entity(1).unwrap().current_target_id,
        None,
        "malformed/non-numeric must not set a target"
    );
    assert!(
        drain(&mut rx).is_empty(),
        "malformed gmSetTarget must emit nothing"
    );
}
