//! Relog, abandon and re-accept for mission 568 (Tollana, world 19).
//!
//! Spec: `docs/analysis/tollana-continuation/w1-packets.md` (T-03a4). The
//! bind ladder and the abandon rows are the F5 expected shapes. Every walk
//! runs through the executor rig in `rig.rs`, so the binds are the real
//! per-player dialog-set state after each step.

use cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_COMPLETED};

use super::rig::{rig, Rig, PLAYER};
use crate::test_support::require_db_or_skip;

/// Assert slot 420 holds the expected binds, and never more than one.
fn assert_ladder(r: &Rig, step: &str, want: &[i32]) {
    let binds = r.binds(PLAYER);
    assert!(
        binds.len() <= 1,
        "{step}: slot 420 holds two binds: {binds:?}"
    );
    assert_eq!(binds, want, "{step}: slot 420 binds");
}

/// Click the Copplemann and close the 2153 offer, which accepts 568.
async fn accept(r: &mut Rig) {
    assert!(r.click(PLAYER, "Tollana_Copplemann").await);
    r.close(PLAYER, 2153).await;
}

#[tokio::test]
async fn live_db_568_slot_420_never_holds_two_binds() {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool).await;
    r.enter(PLAYER).await;
    assert_ladder(&r, "entry", &[2656]);
    accept(&mut r).await;
    assert_ladder(&r, "accept", &[4959]);
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Nest").await);
    assert_ladder(&r, "nest", &[4959]);
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Core").await);
    assert_ladder(&r, "core", &[6393]);
    r.close(PLAYER, 2154).await;
    assert_ladder(&r, "turn-in", &[]);
}

#[tokio::test]
async fn live_db_568_entry_repaints_each_state() {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool).await;
    r.enter(PLAYER).await;
    assert_ladder(&r, "S0", &[2656]);
    accept(&mut r).await;
    r.enter(PLAYER).await;
    assert_ladder(&r, "S1964", &[4959]);
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Fungus").await);
    r.enter(PLAYER).await;
    assert_ladder(&r, "S1965", &[4959]);
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Core").await);
    r.enter(PLAYER).await;
    assert_ladder(&r, "S1966", &[6393]);
    r.close(PLAYER, 2154).await;
    r.enter(PLAYER).await;
    assert_eq!(
        r.status(PLAYER),
        Some(MISSION_COMPLETED),
        "turn-in completes"
    );
    assert_ladder(&r, "Done", &[]);
}

#[tokio::test]
async fn live_db_568_abandon_then_reaccept() {
    let pool = require_db_or_skip!();
    let mut r = rig(&pool).await;
    r.enter(PLAYER).await;
    accept(&mut r).await;
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Fungus").await);
    assert_eq!(r.step(PLAYER), Some(1965), "Fungus moves 1964 to 1965");
    r.abandon(PLAYER).await;
    assert_eq!(r.status(PLAYER), None, "abandon drops mission 568");
    assert_eq!(r.binds(PLAYER), vec![2656], "abandon repaints the offer");
    assert!(r.click(PLAYER, "Tollana_Copplemann").await);
    r.close(PLAYER, 2153).await;
    assert_eq!(r.status(PLAYER), Some(MISSION_ACTIVE), "re-accept");
    assert_eq!(r.step(PLAYER), Some(1964), "re-accept starts at 1964");
    assert_eq!(r.binds(PLAYER), vec![4959], "progress bind after re-accept");

    // Separately: abandon from the Core step on a fresh rig.
    let mut r = rig(&pool).await;
    r.enter(PLAYER).await;
    accept(&mut r).await;
    assert!(r.click(PLAYER, "Tollana_FoodUnit_Core").await);
    assert_eq!(r.step(PLAYER), Some(1966), "Core moves to 1966");
    r.abandon(PLAYER).await;
    assert_eq!(r.status(PLAYER), None, "abandon from 1966 drops 568");
    assert_eq!(r.binds(PLAYER), vec![2656], "abandon at 1966 repaints");
}
