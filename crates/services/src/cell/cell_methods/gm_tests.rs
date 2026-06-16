use super::*;
use crate::cell::space_manager::SpaceManager;

/// Re-assert the document-order count of `<Exposed/>` CellMethods in
/// `SGWGmPlayer.def`. The first own exposed method (gmMissionAssign, def
/// line 65) is index 109; counting forward in document order — skipping
/// `gmSetCallback` (def line 312, which has NO `<Exposed/>`) — lands each
/// implemented method at the constant below. If this drifts, the client's
/// method table and our dispatch disagree and gm* commands silently route
/// to the wrong handler.
#[test]
fn gm_indices_match_def_document_order() {
    // 109 + offset, where offset is the zero-based document-order position
    // among exposed methods (gmMissionAssign = 0).
    assert_eq!(
        GM_GIVE_ITEM,
        109 + 24,
        "gmGiveItem is the 25th exposed (def line 185)"
    );
    assert_eq!(
        GM_GOTO_XYZ,
        109 + 54,
        "gmGotoXYZ is the 55th exposed (def line 348)"
    );
    assert_eq!(
        GM_KILL_TARGET,
        109 + 81,
        "gmKillTarget is the 82nd exposed (def line 482)"
    );
}

/// All implemented indices sit in the GM tail (109 or above), so the
/// dispatch-layer gate (`gm_gate::requires_gm`, which gates every index
/// from 109 up) covers them. A constant that slipped below 109 would be
/// reachable by a non-GM — this pins the invariant. The `109` literal here
/// is the same SGWGmPlayer base the gate uses (the
/// `gm_gate::SGWGMPLAYER_CELL_METHOD_BASE` constant); keep them in lockstep.
#[test]
fn implemented_indices_are_in_gm_tail() {
    const GM_TAIL_BASE: u16 = 109;
    for idx in [GM_GIVE_ITEM, GM_GOTO_XYZ, GM_KILL_TARGET] {
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

/// Helper to build the `gmGiveItem` arg buffer: WSTRING DesignId + INT32 qty.
fn give_item_args(design_id: &str, qty: i32) -> Vec<u8> {
    let mut args = Vec::new();
    crate::mercury::write_wstring(&mut args, design_id);
    args.extend_from_slice(&qty.to_le_bytes());
    args
}

#[tokio::test]
async fn gm_give_item_emits_grant_with_clamped_qty() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    // Request 5000 — must clamp to GM_GIVE_ITEM_MAX_QTY (1000).
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
            assert_eq!(
                count, GM_GIVE_ITEM_MAX_QTY,
                "quantity must clamp to the cap"
            );
        }
        other => panic!("expected GrantItem, got {other:?}"),
    }
}

#[tokio::test]
async fn gm_give_item_rejects_non_numeric_and_nonpositive_qty() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    // Non-numeric design id → no grant.
    let args = give_item_args("AmberVial", 1);
    assert!(dispatch(1, GM_GIVE_ITEM, &args, &tx, &mut mgr).await);
    assert!(
        rx.try_recv().is_err(),
        "non-numeric DesignId must not grant"
    );

    // Quantity 0 → no grant.
    let args = give_item_args("1234", 0);
    assert!(dispatch(1, GM_GIVE_ITEM, &args, &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err(), "quantity 0 must not grant");
}

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
    // Spatial grid updated.
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
async fn gm_kill_target_kills_npc_in_same_space() {
    let mut mgr = mgr_with_player(1, "Castle");
    // NPC at id 2 in the same space.
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    let (tx, mut _rx) = mpsc::channel(32);

    let args = 2i64.to_le_bytes();
    assert!(dispatch(1, GM_KILL_TARGET, &args, &tx, &mut mgr).await);

    let npc = mgr.get_entity(2).unwrap();
    assert!(
        crate::cell::combat::is_dead_state(npc.state_field),
        "gmKillTarget must mark the NPC dead"
    );
}

#[tokio::test]
async fn gm_kill_target_refuses_player() {
    let mut mgr = mgr_with_player(1, "Castle");
    // A second player (not an NPC) at id 2.
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(e) = mgr.get_entity_mut(2) {
        e.is_player = true;
        e.player_id = Some(200);
    }
    let (tx, mut _rx) = mpsc::channel(32);

    let args = 2i64.to_le_bytes();
    assert!(dispatch(1, GM_KILL_TARGET, &args, &tx, &mut mgr).await);

    let victim = mgr.get_entity(2).unwrap();
    assert!(
        !crate::cell::combat::is_dead_state(victim.state_field),
        "gmKillTarget must refuse a player target"
    );
}

/// An unimplemented 109+ index returns `false` so the router falls
/// through to its (already-authorized) warn arm — no panic.
#[tokio::test]
async fn unimplemented_gm_index_returns_false() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, _rx) = mpsc::channel(8);
    // 142 = gmSetGodMode — in the tail, not implemented here.
    assert!(!dispatch(1, 142, &[], &tx, &mut mgr).await);
}
