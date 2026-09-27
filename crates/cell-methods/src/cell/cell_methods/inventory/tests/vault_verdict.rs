//! BV-03 smoke: the vault verdict the cell attaches to every forwarded
//! inventory request. The session is opened by a real right-click on a
//! record-spawned Banker, through the player dispatcher; the moves, uses
//! and removals go through the inventory dispatcher from their wire method
//! indices, as in a live cell. Each request must carry a fresh check: open
//! next to the Banker, `banker_out_of_range` (with the distance) after the
//! player walks away, `no_vault_session` without a session.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::cell_entity::VaultScope;
use cimmeria_entity::interaction_flags::INT_BANKER;
use cimmeria_wire::cell::vault::VaultAccess;

use super::super::dispatch::dispatch;
use super::super::{MOVE_ITEM, REMOVE_ITEM, USE_ITEM};
use crate::cell::cell_methods::player::{dispatch as player_dispatch, INTERACT};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{make_space_manager, npc_spawn_record};

const PLAYER: u32 = 1;

/// The player (player_id 100) at the origin and a personal Banker at
/// `(3, 0, 0)`, within the interact distance (5).
fn stage() -> (SpaceManager, u32) {
    let mut mgr = make_space_manager();
    mgr.create_entity(PLAYER, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    {
        let player = mgr.get_entity_mut(PLAYER).unwrap();
        player.is_player = true;
        player.player_id = Some(100);
    }
    mgr.connect_entity(PLAYER);
    let banker = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(
        banker,
        &npc_spawn_record("Agnos", [3.0, 0.0, 0.0], INT_BANKER, VaultScope::Personal),
    )
    .unwrap();
    (mgr, banker)
}

async fn open_at_banker(mgr: &mut SpaceManager, banker: u32) {
    let (tx, _rx) = mpsc::channel(64);
    assert!(
        player_dispatch(
            PLAYER,
            INTERACT,
            &(banker as i32).to_le_bytes(),
            &tx,
            mgr,
            &ChainEngine::new()
        )
        .await
    );
    assert!(
        mgr.get_entity(PLAYER).unwrap().vault_session.is_some(),
        "the Banker click opened the session"
    );
}

/// Every message one inventory call forwarded.
async fn call(mgr: &mut SpaceManager, method: u16, args: &[u8]) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(16);
    dispatch(PLAYER, method, args, &tx, mgr, &ChainEngine::new()).await;
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        out.push(msg);
    }
    out
}

/// `moveItem(item 7001 -> vault slot 1)`, wire layout: item, bag, 1-indexed
/// slot, quantity.
fn move_args() -> Vec<u8> {
    [7001i32, 17, 1, -1]
        .iter()
        .flat_map(|v| v.to_le_bytes())
        .collect()
}

fn move_verdict(sent: &[CellToBaseMsg]) -> VaultAccess {
    match sent {
        [CellToBaseMsg::MoveInventoryItem { vault, .. }] => *vault,
        other => panic!("expected one MoveInventoryItem, got {other:?}"),
    }
}

/// Next to the Banker the move carries an open verdict naming it; after a
/// walk of 10 units, the next move carries `banker_out_of_range` with the
/// distance. Fails if the verdict is taken once at open (CAT-D-02, the loot
/// pin that never re-checked) or not attached at all.
#[tokio::test]
async fn every_move_carries_a_fresh_proximity_verdict() {
    let (mut mgr, banker) = stage();
    open_at_banker(&mut mgr, banker).await;

    let near = move_verdict(&call(&mut mgr, MOVE_ITEM, &move_args()).await);
    assert!(near.is_open(), "next to the Banker: {near:?}");
    assert_eq!(near.banker_id(), Some(banker));
    assert!(
        near.distance().is_some_and(|d| (d - 3.0).abs() < 0.01),
        "{near:?}"
    );

    mgr.update_entity_position(PLAYER, [13.0, 0.0, 0.0], [0; 3], [0.0; 3]);
    let far = move_verdict(&call(&mut mgr, MOVE_ITEM, &move_args()).await);
    assert_eq!(far.reason(), Some("banker_out_of_range"), "{far:?}");
    assert_eq!(far.banker_id(), Some(banker));
    assert!(
        far.distance().is_some_and(|d| (d - 10.0).abs() < 0.01),
        "{far:?}"
    );

    // Walking back re-opens it: the verdict is the position now, not a
    // latch.
    mgr.update_entity_position(PLAYER, [1.0, 0.0, 0.0], [0; 3], [0.0; 3]);
    assert!(move_verdict(&call(&mut mgr, MOVE_ITEM, &move_args()).await).is_open());
}

/// No session: `no_vault_session` on the move.
#[tokio::test]
async fn a_move_without_a_session_carries_no_vault_session() {
    let (mut mgr, _banker) = stage();
    let verdict = move_verdict(&call(&mut mgr, MOVE_ITEM, &move_args()).await);
    assert_eq!(verdict, VaultAccess::NO_SESSION);
}

/// `useItem` and `removeItem` carry the same verdict, so the base can
/// refuse a banked item used or dropped away from the Banker.
#[tokio::test]
async fn use_and_remove_carry_the_verdict() {
    let (mut mgr, banker) = stage();
    open_at_banker(&mut mgr, banker).await;

    let use_args: Vec<u8> = [7001i32, 0].iter().flat_map(|v| v.to_le_bytes()).collect();
    match call(&mut mgr, USE_ITEM, &use_args).await.as_slice() {
        [CellToBaseMsg::UseInventoryItem { vault, .. }] => assert!(vault.is_open(), "{vault:?}"),
        other => panic!("expected one UseInventoryItem, got {other:?}"),
    }

    mgr.update_entity_position(PLAYER, [13.0, 0.0, 0.0], [0; 3], [0.0; 3]);
    let mut remove_args = 7001i32.to_le_bytes().to_vec();
    remove_args.extend_from_slice(&1i16.to_le_bytes());
    match call(&mut mgr, REMOVE_ITEM, &remove_args).await.as_slice() {
        [CellToBaseMsg::RemoveInventoryItem { vault, .. }] => {
            assert_eq!(vault.reason(), Some("banker_out_of_range"), "{vault:?}")
        }
        other => panic!("expected one RemoveInventoryItem, got {other:?}"),
    }
}
