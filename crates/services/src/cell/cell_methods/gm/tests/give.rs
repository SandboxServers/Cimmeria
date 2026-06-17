use super::super::*; // gm module: dispatch + GM_* constants
use super::*; // shared helpers from tests/mod.rs
use crate::cell::messages::CellToBaseMsg;
use cimmeria_entity::inventory::INV_MAIN;
use tokio::sync::mpsc;

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

#[tokio::test]
async fn give_handlers_reject_truncated_args() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    // Each needs an INT32 (or WSTRING+…); empty args must no-op.
    for idx in [GM_GIVE_XP, GM_GIVE_CASH, GM_GIVE_ITEM, GM_REMOVE_ITEM] {
        assert!(dispatch(1, idx, &[], &tx, &mut mgr).await);
    }
    // gmRemoveItem with only the INT32 (missing the INT16 quantity) is also short.
    assert!(dispatch(1, GM_REMOVE_ITEM, &7i32.to_le_bytes(), &tx, &mut mgr).await);
    assert!(
        drain(&mut rx).is_empty(),
        "truncated give/remove args must emit nothing"
    );
}

#[tokio::test]
async fn give_handlers_require_player_id() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.get_entity_mut(1).unwrap().player_id = None;
    let (tx, mut rx) = mpsc::channel(8);

    assert!(dispatch(1, GM_GIVE_XP, &100i32.to_le_bytes(), &tx, &mut mgr).await);
    assert!(dispatch(1, GM_GIVE_CASH, &100i32.to_le_bytes(), &tx, &mut mgr).await);
    assert!(dispatch(1, GM_GIVE_ITEM, &give_item_args("5", 1), &tx, &mut mgr).await);
    let mut rm = 5i32.to_le_bytes().to_vec();
    rm.extend_from_slice(&1i16.to_le_bytes());
    assert!(dispatch(1, GM_REMOVE_ITEM, &rm, &tx, &mut mgr).await);
    assert!(
        drain(&mut rx).is_empty(),
        "no player_id must block every give/remove"
    );
}

#[tokio::test]
async fn remove_item_rejects_nonpositive_item_id() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);
    let mut args = 0i32.to_le_bytes().to_vec();
    args.extend_from_slice(&5i16.to_le_bytes());
    assert!(dispatch(1, GM_REMOVE_ITEM, &args, &tx, &mut mgr).await);
    assert!(rx.try_recv().is_err(), "item_id 0 must not remove");
}

/// Build `(INT32 disciplineId, INT32 expertise)`.
fn give_expertise_args(discipline_id: i32, expertise: i32) -> Vec<u8> {
    let mut args = discipline_id.to_le_bytes().to_vec();
    args.extend_from_slice(&expertise.to_le_bytes());
    args
}

#[tokio::test]
async fn gm_give_expertise_emits_grant() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    let args = give_expertise_args(7, 25);
    assert!(dispatch(1, GM_GIVE_EXPERTISE, &args, &tx, &mut mgr).await);
    match rx
        .try_recv()
        .expect("gmGiveExpertise must emit GrantExpertise")
    {
        CellToBaseMsg::GrantExpertise {
            entity_id,
            player_id,
            discipline_id,
            amount,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(player_id, 100);
            assert_eq!(discipline_id, 7);
            assert_eq!(amount, 25);
        }
        other => panic!("expected GrantExpertise, got {other:?}"),
    }
}

#[tokio::test]
async fn gm_give_expertise_rejects_nonpositive_fields() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    // Non-positive amount.
    assert!(
        dispatch(
            1,
            GM_GIVE_EXPERTISE,
            &give_expertise_args(7, 0),
            &tx,
            &mut mgr
        )
        .await
    );
    assert!(rx.try_recv().is_err(), "expertise 0 must not grant");

    // Non-positive discipline id.
    assert!(
        dispatch(
            1,
            GM_GIVE_EXPERTISE,
            &give_expertise_args(0, 10),
            &tx,
            &mut mgr
        )
        .await
    );
    assert!(rx.try_recv().is_err(), "discipline id 0 must not grant");

    // Truncated (missing the second INT32).
    assert!(dispatch(1, GM_GIVE_EXPERTISE, &7i32.to_le_bytes(), &tx, &mut mgr).await);
    assert!(
        rx.try_recv().is_err(),
        "truncated expertise args must not grant"
    );
}

#[tokio::test]
async fn gm_give_applied_science_emits_grant_and_rejects_nonpositive() {
    let mut mgr = mgr_with_player(1, "Castle");
    let (tx, mut rx) = mpsc::channel(8);

    assert!(
        dispatch(
            1,
            GM_GIVE_APPLIED_SCIENCE_POINTS,
            &15i32.to_le_bytes(),
            &tx,
            &mut mgr
        )
        .await
    );
    match rx
        .try_recv()
        .expect("gmGiveAppliedSciencePoints must emit GrantAppliedSciencePoints")
    {
        CellToBaseMsg::GrantAppliedSciencePoints {
            entity_id,
            player_id,
            amount,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(player_id, 100);
            assert_eq!(amount, 15);
        }
        other => panic!("expected GrantAppliedSciencePoints, got {other:?}"),
    }

    assert!(
        dispatch(
            1,
            GM_GIVE_APPLIED_SCIENCE_POINTS,
            &0i32.to_le_bytes(),
            &tx,
            &mut mgr
        )
        .await
    );
    assert!(rx.try_recv().is_err(), "zero ASP must not grant");
}

#[tokio::test]
async fn crafting_grants_require_player_id() {
    let mut mgr = mgr_with_player(1, "Castle");
    mgr.get_entity_mut(1).unwrap().player_id = None;
    let (tx, mut rx) = mpsc::channel(8);

    assert!(
        dispatch(
            1,
            GM_GIVE_EXPERTISE,
            &give_expertise_args(7, 25),
            &tx,
            &mut mgr
        )
        .await
    );
    assert!(
        dispatch(
            1,
            GM_GIVE_APPLIED_SCIENCE_POINTS,
            &15i32.to_le_bytes(),
            &tx,
            &mut mgr
        )
        .await
    );
    assert!(
        drain(&mut rx).is_empty(),
        "no player_id must block both crafting grants"
    );
}
