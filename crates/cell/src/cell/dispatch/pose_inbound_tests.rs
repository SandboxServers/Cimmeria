//! Tests for the router's note of the `interact` packet seq, the source of the
//! pose row's `mercury_seq` (PC-26).

use cimmeria_cell_world::cell::interaction_pose::{inbound::take_inbound, InteractionKind};
use std::time::Instant;
use tokio::sync::mpsc;

use super::super::space_manager::SpaceManager;
use super::*;

/// The caller of every test here. Its own id keeps the process-global map apart from the other tests.
const CALLER: u32 = 9228;

fn world() -> SpaceManager {
    let mut mgr = crate::test_support::make_space_manager();
    mgr.create_entity(CALLER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(CALLER).unwrap();
    p.is_player = true;
    p.player_id = Some(100);
    mgr
}

async fn route(mgr: &mut SpaceManager, index: u16, args: &[u8], seq: Option<u32>) {
    let engine = cimmeria_content_engine::chain::ChainEngine::new();
    let (tx, _rx) = mpsc::channel(64);
    dispatch_cell_method(CALLER, index, args, &tx, mgr, &engine, seq).await;
}

/// Pins that the router notes the `interact` seq for the pose row. Removing the router line fails this test.
#[tokio::test]
async fn router_notes_the_interact_seq() {
    let mut mgr = world();
    route(&mut mgr, CM_INTERACT, &[0; 4], Some(777)).await;
    assert_eq!(
        take_inbound(CALLER, InteractionKind::Interact, Instant::now()),
        Some(777)
    );
}
