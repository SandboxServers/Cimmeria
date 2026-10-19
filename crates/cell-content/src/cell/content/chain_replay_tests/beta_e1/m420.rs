//! Beta Site E1 Reclamation (UB-31, mission 420): the blurb bind (23340), the
//! offer (23341), the Accept (23342), the five scanner pads (23343-23347), the
//! relog restore and clear (23348, 23351), and the Mimir turn-in and "not yet"
//! pair (23349, 23350). Seeded in
//! `db/resources/Content/Seed/beta_e1_m420_chains.sql`.
//!
//! Omega UO-16 is HELD (HELD-B16), so no live Asgard reaches Mimir through
//! Beta yet. These tests replay archetype 5 (D-UB13).

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, fire, login_ctx, summarized, whole_seed, with_counter, with_mission, with_step,
    ALL_ARCHETYPES, ASGARD, BETA,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const BLURB: i64 = 23340;
const OFFER: i64 = 23341;
const ACCEPT: i64 = 23342;
const SCANNER_1: i64 = 23343;
const RELOG_TURN_IN: i64 = 23348;
const TURN_IN: i64 = 23349;
const NOT_YET: i64 = 23350;
const RELOG_CLEAR: i64 = 23351;
const M420_CHAINS: std::ops::RangeInclusive<i64> = 23340..=23351;
/// The Beta block (23310-23359) that the mission 481, 420 and travel chains share.
const BETA_BLOCK: std::ops::RangeInclusive<i64> = 23310..=23359;
const MIMIR: &str = "BetaE1_Mimir";
const TURN_IN_DIALOG: i64 = 1953;

/// The Beta block chains that `resolved` matched, for the exclusivity checks.
fn beta_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| BETA_BLOCK.contains(id))
        .collect()
}

/// The mission 420 chains that `resolved` matched.
fn m420_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    beta_matched(resolved)
        .into_iter()
        .filter(|id| M420_CHAINS.contains(id))
        .collect()
}

/// An `interact_tag` context on `tag`. `None` leaves the archetype unset.
fn click(tag: &str, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    ctx
}

/// A `dialog_choice` context on `dialog_id`. `None` leaves the archetype unset.
fn choice(dialog_id: i64, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    ctx
}

/// Every race that is not Asgard: `None` and archetypes 0-4, 6-8.
fn non_asgard() -> Vec<Option<i64>> {
    std::iter::once(None)
        .chain(
            ALL_ARCHETYPES
                .iter()
                .copied()
                .filter(|a| *a != ASGARD)
                .map(Some),
        )
        .collect()
}

/// Mission 481 done, 420 not started: the state that opens the offer.
fn set_offer_open(ctx: &mut ExecutionContext) {
    with_mission(ctx, 481, "completed");
    with_mission(ctx, 420, "not_active");
}

/// Mission 420 at S1455 (the quota) with `counter` scanners counted.
fn set_quota(ctx: &mut ExecutionContext, counter: i64) {
    with_mission(ctx, 481, "completed");
    with_mission(ctx, 420, "active");
    with_step(ctx, 420, 1455, "active");
    with_counter(ctx, "beta_420_scanners", counter);
}

/// Mission 481 at its Mimir step S1762, 420 not started (the 481 turn-in state).
fn set_s1762(ctx: &mut ExecutionContext) {
    with_mission(ctx, 481, "active");
    with_step(ctx, 481, 1762, "active");
    with_mission(ctx, 420, "not_active");
}

/// Mission 420 done, mission 481 done.
fn set_finished(ctx: &mut ExecutionContext) {
    with_mission(ctx, 481, "completed");
    with_mission(ctx, 420, "completed");
}

/// Revert: drop 23340's `mission_completed 481` trigger row or its 481 gate.
/// Then the blurb bind is missing after 481 completes in the same session.
#[tokio::test]
async fn blurb_binds_on_login_and_on_481_completion() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut login = login_ctx(BETA, Some(ASGARD));
    set_offer_open(&mut login);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &login);
    assert_eq!(
        summarized(&resolved, BLURB),
        vec!["add_dialog_set(2344, slot=2305)".to_string()],
    );
    assert_no_deferred_actions(&resolved, BLURB);
    assert_eq!(beta_matched(&resolved), BTreeSet::from([BLURB]));

    let mut done = ExecutionContext::new();
    done.set_param("mission_id".to_string(), serde_json::json!(481));
    done.set_param("archetype".to_string(), serde_json::json!(ASGARD));
    set_offer_open(&mut done);
    let resolved = fire(&engine, TriggerType::MissionCompleted, &done);
    assert_eq!(
        summarized(&resolved, BLURB),
        vec!["add_dialog_set(2344, slot=2305)".to_string()],
    );
    assert_eq!(beta_matched(&resolved), BTreeSet::from([BLURB]));
}

/// Revert: drop 23340's archetype row. Then a non-Asgard login would bind the
/// blurb, and an active 420 or an unfinished 481 would bind it too.
#[tokio::test]
async fn blurb_bind_ignores_other_races_and_states() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        let mut ctx = login_ctx(BETA, archetype);
        set_offer_open(&mut ctx);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
        assert!(summarized(&resolved, BLURB).is_empty(), "{archetype:?}");
    }
    let mut mid_quest = login_ctx(BETA, Some(ASGARD));
    set_quota(&mut mid_quest, 0);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &mid_quest);
    assert!(summarized(&resolved, BLURB).is_empty());
    let mut unfinished = login_ctx(BETA, Some(ASGARD));
    with_mission(&mut unfinished, 481, "active");
    with_mission(&mut unfinished, 420, "not_active");
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &unfinished);
    assert!(summarized(&resolved, BLURB).is_empty());
}

/// Revert: drop 23341's `display_dialog 1953` or its 481 gate. The offer then
/// shows early, or never.
#[tokio::test]
async fn offer_pitch_shows_the_offer_to_asgard_only() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = click(MIMIR, Some(ASGARD));
    set_offer_open(&mut ctx);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_eq!(
        summarized(&resolved, OFFER),
        vec!["display_dialog(1953)".to_string()],
    );
    assert_no_deferred_actions(&resolved, OFFER);
    assert_eq!(beta_matched(&resolved), BTreeSet::from([OFFER]));
    for archetype in non_asgard() {
        let mut ctx = click(MIMIR, archetype);
        set_offer_open(&mut ctx);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(summarized(&resolved, OFFER).is_empty(), "{archetype:?}");
    }
    let mut accepted = click(MIMIR, Some(ASGARD));
    set_quota(&mut accepted, 0);
    let resolved = fire(&engine, TriggerType::InteractTag, &accepted);
    assert!(summarized(&resolved, OFFER).is_empty());
}

/// Revert: drop 23342's `remove_dialog_set 2344` or its `add_dialog_set 2343`,
/// or move the accept off the offer 1953.
#[tokio::test]
async fn accept_binds_the_turn_in_and_drops_the_blurb() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = choice(TURN_IN_DIALOG, Some(ASGARD));
    set_offer_open(&mut ctx);
    let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
    assert_eq!(
        summarized(&resolved, ACCEPT),
        vec![
            "accept_mission(420)".to_string(),
            "remove_dialog_set(2344, slot=2305)".to_string(),
            "add_dialog_set(2343, slot=2305)".to_string(),
        ],
    );
    assert_no_deferred_actions(&resolved, ACCEPT);
    assert_eq!(beta_matched(&resolved), BTreeSet::from([ACCEPT]));

    for archetype in non_asgard() {
        let mut ctx = choice(TURN_IN_DIALOG, archetype);
        set_offer_open(&mut ctx);
        let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
        assert!(summarized(&resolved, ACCEPT).is_empty(), "{archetype:?}");
    }
    let mut mid_quest = choice(TURN_IN_DIALOG, Some(ASGARD));
    set_quota(&mut mid_quest, 0);
    let resolved = fire(&engine, TriggerType::DialogChoice, &mid_quest);
    assert!(summarized(&resolved, ACCEPT).is_empty());
}

/// Revert: drop the `step_status 420 1455` gate from one scanner chain, or
/// the `once` flag on its trigger. Each pad then counts more than once.
#[tokio::test]
async fn scanner_chains_are_once_per_player() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for n in 1..=5_i64 {
        let chain = SCANNER_1 + n - 1;
        let tag = format!("BetaE1_AsgardScanner_{n}");
        let mut ctx = click(&tag, Some(ASGARD));
        set_quota(&mut ctx, 0);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert_eq!(
            summarized(&resolved, chain),
            vec![
                "add_item(2647, 1)".to_string(),
                "increment_counter(beta_420_scanners)".to_string(),
            ],
        );
        assert_no_deferred_actions(&resolved, chain);
        assert_eq!(beta_matched(&resolved), BTreeSet::from([chain]));

        let once: bool = sqlx::query_scalar(
            "SELECT once FROM resources.content_triggers \
             WHERE chain_id = $1 AND event_type = 'interact_tag'",
        )
        .bind(chain as i32)
        .fetch_one(&pool)
        .await
        .expect("each scanner chain has one interact_tag trigger");
        assert!(once, "scanner chain {chain} must be `once`");

        for archetype in non_asgard() {
            let mut ctx = click(&tag, archetype);
            set_quota(&mut ctx, 0);
            let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
            assert!(summarized(&resolved, chain).is_empty(), "{archetype:?}");
        }
        let mut before_quota = click(&tag, Some(ASGARD));
        set_offer_open(&mut before_quota);
        let resolved = fire(&engine, TriggerType::InteractTag, &before_quota);
        assert!(summarized(&resolved, chain).is_empty());
    }
}

/// Revert: drop 23348's bind, or its step gate. A relog mid-quota then loses
/// the turn-in, and the fallback opens it without a completion (F21).
/// Revert: drop 23351's `remove_item`. The old scanners then survive a relog.
#[tokio::test]
async fn relog_during_quota_restores_bind_and_clears_scanners() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = login_ctx(BETA, Some(ASGARD));
    set_quota(&mut ctx, 3);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
    assert_eq!(
        summarized(&resolved, RELOG_TURN_IN),
        vec!["add_dialog_set(2343, slot=2305)".to_string()],
    );
    assert_eq!(
        summarized(&resolved, RELOG_CLEAR),
        vec!["remove_item(2647, 5)".to_string()],
    );
    assert_no_deferred_actions(&resolved, RELOG_CLEAR);
    assert_eq!(
        beta_matched(&resolved),
        BTreeSet::from([RELOG_TURN_IN, RELOG_CLEAR]),
    );

    for archetype in non_asgard() {
        let mut ctx = login_ctx(BETA, archetype);
        set_quota(&mut ctx, 3);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
        assert!(beta_matched(&resolved).is_empty(), "{archetype:?}");
    }
    let mut finished = login_ctx(BETA, Some(ASGARD));
    set_finished(&mut finished);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &finished);
    assert!(beta_matched(&resolved).is_empty());
}

/// Revert: lower 23349's counter gate to `gte 4`, or drop 23350's `lt 5`. The
/// exact list and the matched set both catch it: 4 scanners must never turn in.
#[tokio::test]
async fn mimir_turn_in_needs_five_scanners() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut short = click(MIMIR, Some(ASGARD));
    set_quota(&mut short, 4);
    let resolved = fire(&engine, TriggerType::InteractTag, &short);
    assert_eq!(
        summarized(&resolved, NOT_YET),
        vec!["display_dialog(1954)".to_string()],
    );
    assert_eq!(beta_matched(&resolved), BTreeSet::from([NOT_YET]));

    for counter in [5, 6] {
        let mut full = click(MIMIR, Some(ASGARD));
        set_quota(&mut full, counter);
        let resolved = fire(&engine, TriggerType::InteractTag, &full);
        assert_eq!(
            summarized(&resolved, TURN_IN),
            vec![
                "display_dialog(3057)".to_string(),
                "remove_dialog_set(2343, slot=2305)".to_string(),
                "remove_item(2647, 5)".to_string(),
                "complete_mission(420)".to_string(),
            ],
        );
        assert_no_deferred_actions(&resolved, TURN_IN);
        assert_eq!(beta_matched(&resolved), BTreeSet::from([TURN_IN]));
    }

    for archetype in non_asgard() {
        let mut ctx = click(MIMIR, archetype);
        set_quota(&mut ctx, 5);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert!(summarized(&resolved, TURN_IN).is_empty(), "{archetype:?}");
    }
}

/// One Mimir dialog chain per state. Revert: a sibling that shows a dialog on
/// the same click would add a second chain to the matched set.
#[tokio::test]
async fn mimir_one_dialog_chain_per_state() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let cases: [(fn(&mut ExecutionContext), i64); 5] = [
        (set_s1762, 23317),
        (set_offer_open, OFFER),
        (|c| set_quota(c, 4), NOT_YET),
        (|c| set_quota(c, 5), TURN_IN),
        (set_finished, 0),
    ];
    for (state, expected) in cases {
        let mut ctx = click(MIMIR, Some(ASGARD));
        state(&mut ctx);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        let want = if expected == 0 {
            BTreeSet::new()
        } else {
            BTreeSet::from([expected])
        };
        assert_eq!(beta_matched(&resolved), want);
    }
}

/// Every positive event above, replayed for each non-Asgard race. No 420
/// chain may fire for any of them (wrong race, D-UB13).
#[tokio::test]
async fn wrong_race_fires_no_420_chain() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        let mut login = login_ctx(BETA, archetype);
        set_offer_open(&mut login);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &login);
        assert!(m420_matched(&resolved).is_empty(), "login {archetype:?}");

        let mut offer = click(MIMIR, archetype);
        set_offer_open(&mut offer);
        let resolved = fire(&engine, TriggerType::InteractTag, &offer);
        assert!(m420_matched(&resolved).is_empty(), "offer {archetype:?}");

        let mut accept = choice(TURN_IN_DIALOG, archetype);
        set_offer_open(&mut accept);
        let resolved = fire(&engine, TriggerType::DialogChoice, &accept);
        assert!(m420_matched(&resolved).is_empty(), "accept {archetype:?}");

        let mut scanner = click("BetaE1_AsgardScanner_1", archetype);
        set_quota(&mut scanner, 0);
        let resolved = fire(&engine, TriggerType::InteractTag, &scanner);
        assert!(m420_matched(&resolved).is_empty(), "scanner {archetype:?}");

        let mut turn_in = click(MIMIR, archetype);
        set_quota(&mut turn_in, 5);
        let resolved = fire(&engine, TriggerType::InteractTag, &turn_in);
        assert!(m420_matched(&resolved).is_empty(), "turn-in {archetype:?}");
    }
}
