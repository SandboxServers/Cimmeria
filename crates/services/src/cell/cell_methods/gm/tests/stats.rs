use super::super::*; // gm module: dispatch + GM_* constants
use super::*; // shared helpers from tests/mod.rs
use crate::cell::messages::CellToBaseMsg;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

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
        msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index: 20,
                ..
            }
        )),
        "gmSetHealth must emit an onStatUpdate to the player"
    );
    // Cell-local success: feedback states what happened.
    let fb = feedback_text(&msgs, 1).expect("gmSetHealth success must feed back");
    assert!(
        fb.contains("set to 75"),
        "gmSetHealth feedback must report the value, got: {fb}"
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
    let msgs = drain(&mut rx);
    assert!(
        !msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::EntityMethodCall {
                method_index: 20,
                ..
            }
        )),
        "negative amount must not emit an onStatUpdate"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "negative amount must feed back a rejection"
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
    let msgs = drain(&mut rx);
    assert!(
        !msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::EntityMethodCall {
                method_index: 20,
                ..
            }
        )),
        "cross-space target must not emit an onStatUpdate"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "cross-space target must feed back a rejection"
    );
}

#[tokio::test]
async fn set_stat_rejects_truncated_args_and_missing_stat() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    // No amount.
    assert!(dispatch(1, GM_SET_HEALTH, &[], &tx, &mut mgr).await);
    // Amount but no INT64 target.
    assert!(dispatch(1, GM_SET_FOCUS, &10i32.to_le_bytes(), &tx, &mut mgr).await);
    // Target id that doesn't resolve to any entity.
    assert!(dispatch(1, GM_SET_HEALTH, &set_stat_args(10, 9999), &tx, &mut mgr).await);
    let msgs = drain(&mut rx);
    assert!(
        !msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::EntityMethodCall {
                method_index: 20,
                ..
            }
        )),
        "truncated / unresolved set-stat must not emit an onStatUpdate"
    );
    assert!(
        feedback_text(&msgs, 1).is_some(),
        "truncated / unresolved set-stat must feed back a rejection"
    );
}

#[tokio::test]
async fn set_focus_max_then_current_applies() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    // FOCUS default max is 0, so raise the ceiling first, then set current.
    assert!(dispatch(1, GM_SET_FOCUS_MAX, &set_stat_args(50, 0), &tx, &mut mgr).await);
    assert!(dispatch(1, GM_SET_FOCUS, &set_stat_args(30, 0), &tx, &mut mgr).await);
    let f = mgr
        .get_entity(1)
        .unwrap()
        .stats
        .get(cimmeria_entity::stats::FOCUS)
        .unwrap();
    assert_eq!(f.max, 50);
    assert_eq!(f.cur, 30);
    assert!(
        !drain(&mut rx).is_empty(),
        "set-focus must broadcast onStatUpdate"
    );
}
