//! Two players on Tollana share one mission 568 chain set, but each
//! player's mission state and slot-420 binds are their own.
//!
//! Spec: `docs/analysis/tollana-continuation/w1-packets.md` (T-03a4). The
//! walks run through the executor rig in `rig.rs` for both players.

use cimmeria_entity::missions::MISSION_ACTIVE;

use super::rig::{rig, Rig, PLAYER, PLAYER_B};
use crate::test_support::require_db_or_skip;

/// Click the Copplemann and close the 2153 offer for `who`, accepting 568.
async fn accept(r: &mut Rig, who: u32) {
    assert!(r.click(who, "Tollana_Copplemann").await);
    r.close(who, 2153).await;
}

#[tokio::test]
async fn live_db_568_two_players_are_isolated() {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool).await;
    r.enter(PLAYER).await;
    r.enter(PLAYER_B).await;

    // Player A accepts and clicks the Core: A reaches 1966.
    accept(&mut r, PLAYER).await;
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Core").await);
    assert_eq!(r.step(PLAYER), Some(1966), "A is at the Core step");
    assert_eq!(r.binds(PLAYER), vec![6393], "A holds the turn-in bind");

    // Player B has no 568 state and an untouched offer.
    assert_eq!(r.status(PLAYER_B), None, "B has no mission 568");
    assert_eq!(r.binds(PLAYER_B), vec![2656], "B still sees the offer");

    // B clicks the Core with no mission: nothing changes for B.
    assert!(r.click(PLAYER_B, "Tollana_FoodUnit_Core").await);
    assert_eq!(r.status(PLAYER_B), None, "B's Core click starts nothing");
    assert_eq!(r.binds(PLAYER_B), vec![2656], "B's offer is unchanged");

    // B can still accept, and A is unaffected.
    accept(&mut r, PLAYER_B).await;
    assert_eq!(r.status(PLAYER_B), Some(MISSION_ACTIVE), "B accepts 568");
    assert_eq!(r.step(PLAYER_B), Some(1964), "B starts at 1964");
    assert_eq!(r.step(PLAYER), Some(1966), "A stays at 1966");
    assert_eq!(r.binds(PLAYER), vec![6393], "A keeps the turn-in bind");
}
