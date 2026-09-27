//! DU-02b — the Castle button moves, replayed with the real cooked
//! `ButtonID` on the wire.
//!
//! Packet DU-02b moved one button onto each dialog's final screen:
//! Accept (type 2, id 8) onto 2573's screen 113558 and 5861's screen
//! 96789, and "Take Missions" (type 4, id 71) onto 2576's screen 96825.
//! The ids are the ones the game cooked; the client resolves a click to a
//! position in the screen's button array and puts the `ButtonID` stored
//! there on the wire (fact F14), so these are the exact values
//! `fire_dialog_choice` will see in production.
//!
//! The sibling files [`super::arrival`] and [`super::body`] already pin
//! what each chain resolves. They were left untouched on purpose: chains
//! match on dialog id alone (fact F9), so moving a button cannot change
//! what they resolve, and a green unmodified suite is the evidence for
//! that. What is new here is the wire value itself — these tests would be
//! the ones to fail if a future packet added the `button_id` condition
//! DU-06 proposes and got its sense backwards.
//!
//! # The negative case is not testable here, and that is the finding
//!
//! Once a dialog carries any button, closing it — Done, the title-bar X,
//! or the Decline the client draws automatically beside Accept — sends
//! **nothing at all** (fact F8). There is no `dialogButtonChoice` frame,
//! so there is no `TriggerEvent` to replay and no server-side behaviour
//! to assert. "Closing 2576 early grants nothing" is true because the
//! server never hears about it, not because a chain declined to fire, and
//! writing a replay test that fed a synthetic event would be testing a
//! packet the client does not send.
//!
//! What that leaves the player is re-entry, which IS server-side and is
//! covered: chain 1236 re-displays 2576 on any later interact while step
//! 2421 is active, and chains 1202/1203 re-display 2573/5861 while step
//! 2399 is not active. The bind that makes each NPC clickable is dropped
//! only by the accept/turn-in chains, so an early close leaves it in
//! place. Those paths are pinned in [`super::arrival`] and
//! [`super::body`].

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;

use super::{dialog_choice_ctx, engine_for, fire, summarized, with_mission, with_step, CHOICE};
use crate::test_support::require_db_or_skip;

/// The cooked `ButtonID` of the Accept button on 2573 and 5861.
const ACCEPT: i64 = 8;
/// The cooked `ButtonID` of 2576's "Take Missions" button.
const TAKE_MISSIONS: i64 = 71;

/// A `dialog_choice` context carrying the `button_id` the client really
/// sends, which `fire_dialog_choice` stamps alongside `dialog_id`
/// (`content/event_dispatch/dialog.rs`).
///
/// [`super::dialog_choice_ctx`] deliberately sets only `dialog_id`,
/// because that is all any condition can read today. This helper adds the
/// field so the replay matches the production event shape rather than the
/// minimum the engine happens to need.
fn click(dialog_id: i64, button_id: i64) -> ExecutionContext {
    let mut ctx = dialog_choice_ctx(dialog_id);
    ctx.set_param("button_id".to_string(), serde_json::json!(button_id));
    ctx
}

/// Load several seeded chains into one engine, so a single event is
/// dispatched against all of them the way the cell service does it.
///
/// The `expect` on `None` is the non-vacuity guard [`super::engine_for`]
/// carries for one chain: delete a chain's seed rows and this panics
/// rather than quietly registering fewer chains and asserting against a
/// short action list.
async fn engine_for_all(pool: &sqlx::PgPool, chain_ids: &[i32]) -> ChainEngine {
    let mut engine = ChainEngine::new();
    for &chain_id in chain_ids {
        let chain = super::super::super::engine_loader::load_single_chain_for_test(pool, chain_id)
            .await
            .unwrap_or_else(|e| panic!("DB query for chain {chain_id} must succeed: {e}"))
            .unwrap_or_else(|| panic!("chain {chain_id} must exist in seeded content_chains"));
        engine.register_chain(chain);
    }
    engine
}

/// Clicking Accept on 2573's final screen accepts 701 for a Human.
#[tokio::test]
async fn accept_on_2573_final_screen_accepts_701_for_a_human() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1204).await;

    let mut ctx = click(2573, ACCEPT);
    with_mission(&mut ctx, 701, "not_active");

    assert_eq!(
        summarized(&fire(&engine, CHOICE, &ctx), 1204),
        vec![
            "accept_mission(701)",
            "remove_dialog_set(3062, slot=149)",
            "add_dialog_set(3062, slot=48)",
        ],
        "the Accept button DU-02b left on screen 113558 sends ButtonID {ACCEPT}; that click \
         must accept 701 and hand the topic to Copplemann",
    );
}

/// Clicking Accept on 5861's final screen accepts 701 for a Jaffa.
///
/// This is the one that was genuinely broken. Accept shipped on screens
/// 96782-96786 of eight and the final screen 96789 was bare, so a Jaffa
/// who read the briefing to the end had nothing to press: Done is a
/// close, a close on a dialog that HAS buttons sends nothing, and
/// mission 701 was unacceptable for that player. Castle audit D-CA13.
#[tokio::test]
async fn accept_on_5861_final_screen_accepts_701_for_a_jaffa() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1205).await;

    let mut ctx = click(5861, ACCEPT);
    with_mission(&mut ctx, 701, "not_active");

    assert_eq!(
        summarized(&fire(&engine, CHOICE, &ctx), 1205),
        vec![
            "accept_mission(701)",
            "remove_dialog_set(3062, slot=149)",
            "add_dialog_set(3062, slot=48)",
            "display_dialog(5862)",
        ],
        "the Accept button DU-02b put on screen 96789 must accept 701 and play the \
         Moh'katan radio call",
    );
}

/// Clicking "Take Missions" on 2576's final screen runs the whole
/// turn-in: 701 completes and 702 and 703 are accepted, from one event.
///
/// All three chains are registered in one engine because that is the
/// shape production has — `fire_dialog_choice` dispatches a single event
/// against every chain — and because the split across 1237/1238/1239 only
/// works if their conditions are evaluated against the same pre-action
/// snapshot. A per-chain engine would pass even if 1237's
/// `complete_mission` starved the other two.
#[tokio::test]
async fn take_missions_on_2576_final_screen_runs_the_whole_turn_in() {
    let pool = require_db_or_skip!();
    let engine = engine_for_all(&pool, &[1237, 1238, 1239]).await;

    let mut ctx = click(2576, TAKE_MISSIONS);
    with_step(&mut ctx, 701, 2421, "active");
    with_mission(&mut ctx, 702, "not_active");
    with_mission(&mut ctx, 703, "not_active");
    let resolved = fire(&engine, CHOICE, &ctx);

    assert_eq!(
        summarized(&resolved, 1237),
        vec!["remove_dialog_set(3063, slot=48)", "complete_mission(701)"],
        "ButtonID {TAKE_MISSIONS} on screen 96825 must drop the turn-in topic and complete 701",
    );
    assert_eq!(
        summarized(&resolved, 1238),
        vec!["accept_mission(702)"],
        "the same click must hand out Rescue Dr. Zuritska",
    );
    assert_eq!(
        summarized(&resolved, 1239),
        vec!["accept_mission(703)"],
        "the same click must hand out Payback — 2576's button text is plural for a reason",
    );
}

/// The wire `button_id` gates nothing today, and that is load-bearing.
///
/// Chains match on dialog id alone; there is no authorable `button_id`
/// condition (fact F9, `content-engine/src/triggers/matching.rs`). Two
/// consequences the whole packet rests on: moving a button without
/// changing its `ButtonID` cannot change which chain fires, and the
/// close sentinel `-1` that a zero-button dialog sends reaches the same
/// chain as a real click. Packet DU-06 proposes adding the condition —
/// this test is what turns "DU-06 landed and inverted the sense" from a
/// silent progression break into a failure.
#[tokio::test]
async fn the_turn_in_does_not_filter_on_button_id_today() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 1237).await;

    let expected = vec!["remove_dialog_set(3063, slot=48)", "complete_mission(701)"];
    // The real click, an unrelated id, and F8's close sentinel.
    for button_id in [TAKE_MISSIONS, ACCEPT, -1] {
        let mut ctx = click(2576, button_id);
        with_step(&mut ctx, 701, 2421, "active");
        assert_eq!(
            summarized(&fire(&engine, CHOICE, &ctx), 1237),
            expected,
            "chain 1237 resolved differently for button_id {button_id}; no condition may \
             read button_id until DU-06 lands one deliberately",
        );
    }
}
