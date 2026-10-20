//! The offer and the abandon of mission 567 (CS-08b2): the offer is never
//! live in world 68, and an abandon takes the item and the turn-in bind back.
//! A child of [`super`] so it shares that harness.

use super::{
    abandon, arrive, bound_at_copplemann, completed, cross, engine_with, log_in, mission_567,
    mission_of, on_the_leg, staged, step_of, ALL_CHAINS, CASTLE, CMD_CENTER, ENTITY, HARSET_CHAINS,
    OFFER_SET, STEP_EXIT, STEP_TURN_IN, TURN_IN_SET,
};
use crate::test_support::require_db_or_skip;

// ── The offer never reaches world 68 ─────────────────────────────────────

/// **An offer left open when the player leaves Castle is unbound by the
/// crossing, and nothing binds it in world 68.** The login binds 1000018; the
/// Harset crossing (10240, 567 not active) removes it; a Command Center
/// login on the arrival side binds nothing. Revert proof: delete 10240 and
/// the bind survives the crossing.
#[tokio::test]
async fn live_db_mission_567_castle_offer_is_never_live_in_world_68() {
    let pool = require_db_or_skip!();
    let mut ids = ALL_CHAINS.to_vec();
    ids.extend(HARSET_CHAINS);
    let engine = engine_with(&pool, &ids).await;
    let mut castle = staged(&pool, CASTLE, vec![completed(701), completed(703)]).await;
    log_in(&engine, &mut castle, CASTLE).await;
    assert_eq!(
        bound_at_copplemann(&castle),
        vec![OFFER_SET],
        "the open offer is bound in Castle"
    );

    let out = cross(&engine, &mut castle, "Harset").await;

    assert!(out.is_empty(), "the unbind sends nothing; got {out:?}");
    assert!(
        bound_at_copplemann(&castle).is_empty(),
        "the crossing must unbind the open offer"
    );
    assert_eq!(step_of(&castle), None, "no mission was started");

    let mut harset = arrive(&pool, &castle, CMD_CENTER).await;
    log_in(&engine, &mut harset, CMD_CENTER).await;
    assert!(
        bound_at_copplemann(&harset).is_empty(),
        "no chain binds the offer in world 68; got {:?}",
        bound_at_copplemann(&harset)
    );
}

/// **A crossing with 567 running leaves a bound offer alone (seal c2d).** The
/// state is reachable: the offer is bound at login, then a GM-granted 567 is
/// added at 2012 without the accept chain's unbind. The crossing advances to
/// 4039 (10241) and 10240 stays quiet. Revert proof: delete 10240's
/// `mission_status 567 not_active` condition and the bind is removed.
#[tokio::test]
async fn live_db_mission_567_castle_crossing_with_567_running_keeps_the_offer_bound() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, CASTLE, vec![completed(701), completed(703)]).await;
    log_in(&engine, &mut mgr, CASTLE).await;
    assert_eq!(
        bound_at_copplemann(&mgr),
        vec![OFFER_SET],
        "the offer is open"
    );
    mgr.get_entity_mut(ENTITY)
        .unwrap()
        .missions
        .add_mission(mission_567(STEP_EXIT, 2350));

    let out = cross(&engine, &mut mgr, "Harset").await;

    assert_eq!(out, ["step 4039"], "10241 advances and nothing else fires");
    assert_eq!(step_of(&mgr), Some(STEP_TURN_IN));
    assert_eq!(
        bound_at_copplemann(&mgr),
        vec![OFFER_SET],
        "10240 must not unbind while 567 is running"
    );
}

// ── The abandon ──────────────────────────────────────────────────────────

/// **An abandon in Castle takes the item back and reopens the offer.** At 2012
/// 10242 removes 2698 x1 and 10243 binds 1000018 again. Revert proof: delete
/// the `remove_item` row of 10242 and the trace loses the removal; delete
/// 10243 and the offer stays closed.
#[tokio::test]
async fn live_db_mission_567_castle_abandon_in_castle_reopens_the_offer() {
    let pool = require_db_or_skip!();
    let engine = engine_with(&pool, &ALL_CHAINS).await;
    let mut mgr = staged(&pool, CASTLE, on_the_leg(STEP_EXIT, 2350)).await;

    let out = abandon(&engine, &mut mgr).await;

    assert_eq!(out, ["remove 2698 x1"], "the item is taken back once");
    assert!(mission_of(&mgr).is_none(), "567 must be gone");
    assert_eq!(
        bound_at_copplemann(&mgr),
        vec![OFFER_SET],
        "the offer reopens in Castle"
    );
}

/// **An abandon at 4039 in the Command Center clears the stale turn-in and
/// reopens no offer.** 6504 binds 2817; the abandon removes it and 2698, and
/// 10243 (world 8 only) stays quiet. Revert proof: drop 10243's world
/// condition and the offer binds in world 68; delete the `remove_dialog_set`
/// row of 10242 and 2817 stays bound.
#[tokio::test]
async fn live_db_mission_567_castle_abandon_in_world_68_clears_turn_in_only() {
    let pool = require_db_or_skip!();
    let mut ids = ALL_CHAINS.to_vec();
    ids.extend(HARSET_CHAINS);
    let engine = engine_with(&pool, &ids).await;
    let mut mgr = staged(&pool, CMD_CENTER, on_the_leg(STEP_TURN_IN, 4652)).await;
    log_in(&engine, &mut mgr, CMD_CENTER).await;
    assert_eq!(
        bound_at_copplemann(&mgr),
        vec![TURN_IN_SET],
        "6504 binds the turn-in on the Command Center load"
    );

    let out = abandon(&engine, &mut mgr).await;

    assert_eq!(out, ["remove 2698 x1"], "the item is taken back once");
    assert!(
        bound_at_copplemann(&mgr).is_empty(),
        "neither the turn-in nor the offer may remain; got {:?}",
        bound_at_copplemann(&mgr)
    );
}
