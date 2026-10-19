//! Beta Site E1 381 Closing Shop (UB-11): after 1423, Carter offers 381 and
//! Felger takes his checklist (2636), sends the player for five xenobiology
//! cages (3635, counter `beta_381_cages`), and the turn-in completes 381
//! (chains 23040-23053). Seeded in
//! `db/resources/Content/Seed/beta_e1_m381_chains.sql`. Other races and
//! unstamped players must match none of them.

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, engine_for, fire, login_ctx, summarized, whole_seed, with_counter,
    with_mission, with_step, ALL_ARCHETYPES, BETA, HUMANS,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const OFFER_BIND: i64 = 23040;
const OFFER_SHOW: i64 = 23041;
const ACCEPT: i64 = 23042;
const FELGER_RESTORE: i64 = 23043;
const CHECKLIST: i64 = 23044;
/// Cage n (1 to 5) is chain `CAGE_FIRST + n - 1`.
const CAGE_FIRST: i64 = 23045;
const TURN_IN_RESTORE: i64 = 23050;
const TURN_IN: i64 = 23051;
const NOT_YET: i64 = 23052;
const CAGE_RESYNC: i64 = 23053;

const MISSION: i32 = 381;
const PREREQ: i32 = 1423;
const CHECKLIST_STEP: i32 = 1408;
const CAGE_STEP: i32 = 1409;
const CAGE_COUNTER: &str = "beta_381_cages";
const CARTER: &str = "BetaE1_Carter";
const FELGER: &str = "BetaE1_Felger";

/// The 381 chains that `resolved` matched, for the exclusivity checks.
fn m381_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| (OFFER_BIND..=CAGE_RESYNC).contains(id))
        .collect()
}

/// The expected rendered list, from string literals.
fn expect(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// Every archetype outside the Human gate, plus an unstamped player (`None`).
fn non_human_or_unstamped() -> Vec<Option<i64>> {
    ALL_ARCHETYPES
        .iter()
        .copied()
        .filter(|a| !HUMANS.contains(a))
        .map(Some)
        .chain(std::iter::once(None))
        .collect()
}

/// Stamp `archetype` on `ctx` when the player has one.
fn stamp(mut ctx: ExecutionContext, archetype: Option<i64>) -> ExecutionContext {
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    ctx
}

/// An `interact_tag` click on `tag`.
fn tap(tag: &str, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("entity_tag".to_string(), serde_json::json!(tag));
    stamp(ctx, archetype)
}

/// A `dialog_choice` on `dialog_id`.
fn pick(dialog_id: i64, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("dialog_id".to_string(), serde_json::json!(dialog_id));
    stamp(ctx, archetype)
}

/// A `mission_completed` of `mission_id`.
fn finished(mission_id: i64, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(mission_id));
    stamp(ctx, archetype)
}

/// Set 1423 to `prereq`, 381 to `status`, and 381's step when one is given.
fn state(
    mut ctx: ExecutionContext,
    prereq: &str,
    status: &str,
    step: Option<(i32, &str)>,
) -> ExecutionContext {
    with_mission(&mut ctx, PREREQ, prereq);
    with_mission(&mut ctx, MISSION, status);
    if let Some((id, s)) = step {
        with_step(&mut ctx, MISSION, id, s);
    }
    ctx
}

/// The 381 state for a Human who has finished 1423 and not started 381.
fn offer_open(ctx: ExecutionContext) -> ExecutionContext {
    state(ctx, "completed", "not_active", None)
}

/// Revert: delete 23040's rows, drop one of its gates, or change its bind. A
/// Human who finished 1423 (by login or by the completion) must get Carter's
/// blurb bind, and only that.
#[tokio::test]
async fn offer_bind_after_1423_binds_the_blurb_per_player() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, OFFER_BIND as i32).await;
    // 23040 has two trigger rows, so the loader keeps one chain per trigger:
    // the finished-1423 path is only in the whole seed.
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let login = offer_open(login_ctx(BETA, Some(archetype)));
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &login);
        assert_eq!(
            summarized(&resolved, OFFER_BIND),
            expect(&["add_dialog_set(2305, slot=2300)"]),
            "Human archetype {archetype} on login must bind the 381 blurb",
        );
        assert_no_deferred_actions(&resolved, OFFER_BIND);

        let done = offer_open(finished(PREREQ as i64, Some(archetype)));
        let resolved = fire(&whole, TriggerType::MissionCompleted, &done);
        assert_eq!(
            summarized(&resolved, OFFER_BIND),
            expect(&["add_dialog_set(2305, slot=2300)"]),
            "Human archetype {archetype} finishing 1423 must bind the 381 blurb",
        );
        assert_no_deferred_actions(&resolved, OFFER_BIND);
    }

    // Whole seed: a login after 1423 matches the offer bind and nothing else.
    let login = offer_open(login_ctx(BETA, Some(1)));
    assert_eq!(
        m381_matched(&fire(&whole, TriggerType::PlayerLoaded, &login)),
        BTreeSet::from([OFFER_BIND]),
    );
}

/// Revert: drop a gate row on 23040 (race, 1423 completed, or 381 not_active).
/// Other races, unstamped players, a 1423 that is only active, and a 381 that
/// has started must all match nothing.
#[tokio::test]
async fn offer_bind_gates_reject_other_races_unstamped_and_started_381() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, OFFER_BIND as i32).await;
    let whole = whole_seed(&pool).await;
    for archetype in non_human_or_unstamped() {
        let login = offer_open(login_ctx(BETA, archetype));
        assert!(
            summarized(
                &fire(&engine, TriggerType::PlayerLoaded, &login),
                OFFER_BIND
            )
            .is_empty(),
            "archetype {archetype:?} on login must not get the 381 blurb",
        );
        let done = offer_open(finished(PREREQ as i64, archetype));
        assert!(
            summarized(
                &fire(&whole, TriggerType::MissionCompleted, &done),
                OFFER_BIND
            )
            .is_empty(),
            "archetype {archetype:?} finishing 1423 must not get the 381 blurb",
        );
    }
    for archetype in HUMANS {
        let active_1423 = state(
            login_ctx(BETA, Some(archetype)),
            "active",
            "not_active",
            None,
        );
        assert!(
            summarized(
                &fire(&engine, TriggerType::PlayerLoaded, &active_1423),
                OFFER_BIND
            )
            .is_empty(),
            "1423 still active must not bind the 381 blurb",
        );
        let started = state(
            login_ctx(BETA, Some(archetype)),
            "completed",
            "active",
            None,
        );
        assert!(
            summarized(
                &fire(&engine, TriggerType::PlayerLoaded, &started),
                OFFER_BIND
            )
            .is_empty(),
            "381 already active must not bind the 381 blurb",
        );
    }
}

/// Revert: drop the offer's show row or the 1423 gate on 23041. The offer
/// dialog 1882 is shown from Carter, and only once 1423 is complete and 381 has
/// not started. Whole seed: one click, one chain.
#[tokio::test]
async fn carter_click_shows_the_offer_once_1423_is_done() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, OFFER_SHOW as i32).await;
    for archetype in HUMANS {
        let click = offer_open(tap(CARTER, Some(archetype)));
        let resolved = fire(&engine, TriggerType::InteractTag, &click);
        assert_eq!(
            summarized(&resolved, OFFER_SHOW),
            expect(&["display_dialog(1882)"]),
            "Human archetype {archetype} clicking Carter must show the offer",
        );
        assert_no_deferred_actions(&resolved, OFFER_SHOW);
    }
    for archetype in non_human_or_unstamped() {
        let click = offer_open(tap(CARTER, archetype));
        assert!(
            summarized(&fire(&engine, TriggerType::InteractTag, &click), OFFER_SHOW).is_empty(),
            "archetype {archetype:?} clicking Carter must not get the offer",
        );
    }
    let whole = whole_seed(&pool).await;
    let click = offer_open(tap(CARTER, Some(2)));
    assert_eq!(
        m381_matched(&fire(&whole, TriggerType::InteractTag, &click)),
        BTreeSet::from([OFFER_SHOW]),
    );
    // Once 381 has started, the same click offers nothing (the offer is gone).
    let started = state(tap(CARTER, Some(2)), "completed", "active", None);
    assert!(m381_matched(&fire(&whole, TriggerType::InteractTag, &started)).is_empty());
}

/// Revert: a Carter offer that does not wait for 1423 (drop the 1423 gate from
/// 23040 or 23041). With 1423 still active, neither offer chain matches, on a
/// login or on a click.
#[tokio::test]
async fn carter_offer_waits_for_1423() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let login = state(
            login_ctx(BETA, Some(archetype)),
            "active",
            "not_active",
            None,
        );
        assert!(
            m381_matched(&fire(&whole, TriggerType::PlayerLoaded, &login)).is_empty(),
            "1423 active: a login must not offer 381 (archetype {archetype})",
        );
        let click = state(tap(CARTER, Some(archetype)), "active", "not_active", None);
        assert!(
            m381_matched(&fire(&whole, TriggerType::InteractTag, &click)).is_empty(),
            "1423 active: clicking Carter must not offer 381 (archetype {archetype})",
        );
    }
}

/// Revert: drop 23042's rows, or its `pick:1882` row. Accepting the offer
/// accepts 381, unbinds the blurb, gives the checklist and binds Felger.
/// Gate negatives: other races, unstamped players, and 381 already started.
#[tokio::test]
async fn accept_on_dialog_1882_takes_the_checklist() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, ACCEPT as i32).await;
    for archetype in HUMANS {
        let accept = offer_open(pick(1882, Some(archetype)));
        let resolved = fire(&engine, TriggerType::DialogChoice, &accept);
        assert_eq!(
            summarized(&resolved, ACCEPT),
            expect(&[
                "accept_mission(381)",
                "remove_dialog_set(2305, slot=2300)",
                "add_item(2636, 1)",
                "add_dialog_set(2304, slot=2301)",
            ]),
            "Human archetype {archetype} accepting 1882 must take the checklist",
        );
        assert_no_deferred_actions(&resolved, ACCEPT);
    }
    for archetype in non_human_or_unstamped() {
        let accept = offer_open(pick(1882, archetype));
        assert!(
            summarized(&fire(&engine, TriggerType::DialogChoice, &accept), ACCEPT).is_empty(),
            "archetype {archetype:?} accepting 1882 must not take the checklist",
        );
    }
    let started = state(pick(1882, Some(1)), "completed", "active", None);
    assert!(summarized(&fire(&engine, TriggerType::DialogChoice, &started), ACCEPT).is_empty());

    // Seal C1: dialog 1886 is never displayed, so no chain may accept on it.
    let whole = whole_seed(&pool).await;
    let blurb = offer_open(pick(1886, Some(1)));
    assert!(
        m381_matched(&fire(&whole, TriggerType::DialogChoice, &blurb)).is_empty(),
        "pick:1886 must match no 381 chain (seal C1)",
    );
    let accept = offer_open(pick(1882, Some(1)));
    assert_eq!(
        m381_matched(&fire(&whole, TriggerType::DialogChoice, &accept)),
        BTreeSet::from([ACCEPT]),
    );
}

/// Revert: drop 23043's bind or its S1408 gate. A Human who relogs on S1408
/// gets Felger's checklist talk back; nobody else does.
#[tokio::test]
async fn relog_on_checklist_step_rebinds_felger() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, FELGER_RESTORE as i32).await;
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let login = state(
            login_ctx(BETA, Some(archetype)),
            "completed",
            "active",
            Some((CHECKLIST_STEP, "active")),
        );
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &login);
        assert_eq!(
            summarized(&resolved, FELGER_RESTORE),
            expect(&["add_dialog_set(2304, slot=2301)"]),
            "Human archetype {archetype} relogging on S1408 must rebind Felger",
        );
        assert_no_deferred_actions(&resolved, FELGER_RESTORE);
        assert_eq!(
            m381_matched(&fire(&whole, TriggerType::PlayerLoaded, &login)),
            BTreeSet::from([FELGER_RESTORE]),
        );
    }
    for archetype in non_human_or_unstamped() {
        let login = state(
            login_ctx(BETA, archetype),
            "completed",
            "active",
            Some((CHECKLIST_STEP, "active")),
        );
        assert!(summarized(
            &fire(&engine, TriggerType::PlayerLoaded, &login),
            FELGER_RESTORE
        )
        .is_empty());
    }
    let cages = state(
        login_ctx(BETA, Some(1)),
        "completed",
        "active",
        Some((CAGE_STEP, "active")),
    );
    assert!(summarized(
        &fire(&engine, TriggerType::PlayerLoaded, &cages),
        FELGER_RESTORE
    )
    .is_empty());
}

/// Revert: drop the world filter on the `player_loaded` trigger of 23040, 23043,
/// 23050 or 23053. A Human whose state would bind or resync on Beta (offer open,
/// S1408, S1409) who logs into any other world must match no 381 chain.
#[tokio::test]
async fn logins_in_another_world_match_no_381_chain() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    // (381 step, the chains that match on Beta in that state).
    let states: [(Option<i32>, &[i64]); 3] = [
        (None, &[OFFER_BIND]),
        (Some(CHECKLIST_STEP), &[FELGER_RESTORE]),
        (Some(CAGE_STEP), &[TURN_IN_RESTORE, CAGE_RESYNC]),
    ];
    for (step, on_beta) in states {
        let build = |world: &str| match step {
            None => offer_open(login_ctx(world, Some(1))),
            Some(id) => state(
                login_ctx(world, Some(1)),
                "completed",
                "active",
                Some((id, "active")),
            ),
        };
        let resolved = fire(&whole, TriggerType::PlayerLoaded, &build(BETA));
        assert_eq!(
            m381_matched(&resolved),
            on_beta.iter().copied().collect::<BTreeSet<i64>>(),
            "Beta, step {step:?}",
        );
        let resolved = fire(
            &whole,
            TriggerType::PlayerLoaded,
            &build("Some_Other_World"),
        );
        assert!(
            m381_matched(&resolved).is_empty(),
            "another world, step {step:?}",
        );
    }
}

/// Revert: drop 23044's rows or its S1408 gate. Felger takes the checklist back,
/// shows the cage dialog 1884, and moves 381 to S1409 with the turn-in talk
/// bound.
#[tokio::test]
async fn felger_takes_the_checklist_and_sends_the_player_to_the_cages() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, CHECKLIST as i32).await;
    for archetype in HUMANS {
        let click = state(
            tap(FELGER, Some(archetype)),
            "completed",
            "active",
            Some((CHECKLIST_STEP, "active")),
        );
        let resolved = fire(&engine, TriggerType::InteractTag, &click);
        assert_eq!(
            summarized(&resolved, CHECKLIST),
            expect(&[
                "display_dialog(1884)",
                "remove_dialog_set(2304, slot=2301)",
                "remove_item(2636, 1)",
                "advance_step(381, 1409)",
                "add_dialog_set(2302, slot=2301)",
            ]),
            "Human archetype {archetype} clicking Felger on S1408 must hand over to S1409",
        );
        assert_no_deferred_actions(&resolved, CHECKLIST);
    }
    for archetype in non_human_or_unstamped() {
        let click = state(
            tap(FELGER, archetype),
            "completed",
            "active",
            Some((CHECKLIST_STEP, "active")),
        );
        assert!(summarized(&fire(&engine, TriggerType::InteractTag, &click), CHECKLIST).is_empty());
    }
    let cages = state(
        tap(FELGER, Some(1)),
        "completed",
        "active",
        Some((CAGE_STEP, "active")),
    );
    assert!(summarized(&fire(&engine, TriggerType::InteractTag, &cages), CHECKLIST).is_empty());
}

/// Revert: drop a cage's `once` flag, its add, or its counter. Each of the five
/// cages gives one cage to a Human on S1409 and counts once. Its trigger is
/// `once`, so a second click does nothing for that player.
#[tokio::test]
async fn cage_chains_are_once_per_player_and_count_one_each() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for n in 1..=5 {
        let chain = CAGE_FIRST + n - 1;
        let tag = format!("BetaE1_XenoCage_{n}");
        let engine = engine_for(&pool, chain as i32).await;

        let once: bool =
            sqlx::query_scalar("SELECT once FROM resources.content_triggers WHERE chain_id = $1")
                .bind(chain as i32)
                .fetch_one(&pool)
                .await
                .expect("cage chain has one trigger row");
        assert!(once, "cage chain {chain} must be once per player");

        let click = state(
            tap(&tag, Some(3)),
            "completed",
            "active",
            Some((CAGE_STEP, "active")),
        );
        let resolved = fire(&engine, TriggerType::InteractTag, &click);
        assert_eq!(
            summarized(&resolved, chain),
            expect(&["add_item(3635, 1)", "increment_counter(beta_381_cages)"]),
            "{tag} on S1409 must give one cage and count it",
        );
        assert_no_deferred_actions(&resolved, chain);
        assert_eq!(
            m381_matched(&fire(&whole, TriggerType::InteractTag, &click)),
            BTreeSet::from([chain]),
            "{tag} on S1409 must match only its own chain",
        );

        for archetype in non_human_or_unstamped() {
            let click = state(
                tap(&tag, archetype),
                "completed",
                "active",
                Some((CAGE_STEP, "active")),
            );
            assert!(summarized(&fire(&engine, TriggerType::InteractTag, &click), chain).is_empty());
        }
        let before = state(
            tap(&tag, Some(1)),
            "completed",
            "active",
            Some((CHECKLIST_STEP, "active")),
        );
        assert!(summarized(&fire(&engine, TriggerType::InteractTag, &before), chain).is_empty());
    }
}

/// Revert: drop 23050's bind or its S1409 gate. A relog on S1409 rebinds
/// Felger's turn-in talk, and the resync 23053 removes the five cages with the
/// counter.
#[tokio::test]
async fn relog_on_cage_step_rebinds_turn_in_and_resyncs_cages() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let turn_in_restore = engine_for(&pool, TURN_IN_RESTORE as i32).await;
    let resync = engine_for(&pool, CAGE_RESYNC as i32).await;
    for archetype in HUMANS {
        let login = state(
            login_ctx(BETA, Some(archetype)),
            "completed",
            "active",
            Some((CAGE_STEP, "active")),
        );
        let resolved = fire(&turn_in_restore, TriggerType::PlayerLoaded, &login);
        assert_eq!(
            summarized(&resolved, TURN_IN_RESTORE),
            expect(&["add_dialog_set(2302, slot=2301)"]),
            "Human archetype {archetype} relogging on S1409 must rebind the turn-in",
        );
        assert_no_deferred_actions(&resolved, TURN_IN_RESTORE);

        let resolved = fire(&resync, TriggerType::PlayerLoaded, &login);
        assert_eq!(
            summarized(&resolved, CAGE_RESYNC),
            expect(&["remove_item(3635, 5)"]),
            "Human archetype {archetype} relogging on S1409 must resync the cages",
        );
        assert_no_deferred_actions(&resolved, CAGE_RESYNC);

        assert_eq!(
            m381_matched(&fire(&whole, TriggerType::PlayerLoaded, &login)),
            BTreeSet::from([TURN_IN_RESTORE, CAGE_RESYNC]),
        );
    }
    for archetype in non_human_or_unstamped() {
        let login = state(
            login_ctx(BETA, archetype),
            "completed",
            "active",
            Some((CAGE_STEP, "active")),
        );
        assert!(m381_matched(&fire(&whole, TriggerType::PlayerLoaded, &login)).is_empty());
    }
}

/// Revert: drop 23051's counter gate, its `remove_item` or `complete_mission`,
/// or the not-yet sibling 23052. Five cages on Felger complete 381; four show
/// the cage dialog again instead. Other races match neither.
#[tokio::test]
async fn felger_turn_in_needs_five_cages() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let turn_in = engine_for(&pool, TURN_IN as i32).await;
    let not_yet = engine_for(&pool, NOT_YET as i32).await;
    for archetype in HUMANS {
        let mut five = state(
            tap(FELGER, Some(archetype)),
            "completed",
            "active",
            Some((CAGE_STEP, "active")),
        );
        with_counter(&mut five, CAGE_COUNTER, 5);
        let resolved = fire(&turn_in, TriggerType::InteractTag, &five);
        assert_eq!(
            summarized(&resolved, TURN_IN),
            expect(&[
                "display_dialog(1883)",
                "remove_dialog_set(2302, slot=2301)",
                "remove_item(3635, 5)",
                "complete_mission(381)",
            ]),
            "Human archetype {archetype} with five cages must complete 381",
        );
        assert_no_deferred_actions(&resolved, TURN_IN);
        assert_eq!(
            m381_matched(&fire(&whole, TriggerType::InteractTag, &five)),
            BTreeSet::from([TURN_IN]),
        );

        let mut four = state(
            tap(FELGER, Some(archetype)),
            "completed",
            "active",
            Some((CAGE_STEP, "active")),
        );
        with_counter(&mut four, CAGE_COUNTER, 4);
        let resolved = fire(&not_yet, TriggerType::InteractTag, &four);
        assert_eq!(
            summarized(&resolved, NOT_YET),
            expect(&["display_dialog(1884)"]),
            "Human archetype {archetype} with four cages must see the cage talk",
        );
        assert_no_deferred_actions(&resolved, NOT_YET);
        assert_eq!(
            m381_matched(&fire(&whole, TriggerType::InteractTag, &four)),
            BTreeSet::from([NOT_YET]),
        );
    }
    for archetype in non_human_or_unstamped() {
        let mut five = state(
            tap(FELGER, archetype),
            "completed",
            "active",
            Some((CAGE_STEP, "active")),
        );
        with_counter(&mut five, CAGE_COUNTER, 5);
        assert!(m381_matched(&fire(&whole, TriggerType::InteractTag, &five)).is_empty());
    }
}

/// The click on Felger, per state: 381 not started matches no Felger chain; on
/// S1408 it matches the checklist (23044); on S1409 with counter 0 or 4 it
/// matches the not-yet sibling; with 5 it matches the turn-in. One chain each.
#[tokio::test]
async fn felger_click_matches_one_dialog_chain_per_state() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let open = offer_open(tap(FELGER, Some(1)));
    assert!(
        m381_matched(&fire(&whole, TriggerType::InteractTag, &open)).is_empty(),
        "381 not started: Felger has no 381 talk yet",
    );
    let checklist = state(
        tap(FELGER, Some(1)),
        "completed",
        "active",
        Some((CHECKLIST_STEP, "active")),
    );
    assert_eq!(
        m381_matched(&fire(&whole, TriggerType::InteractTag, &checklist)),
        BTreeSet::from([CHECKLIST]),
    );
    for (count, expected) in [(0, NOT_YET), (5, TURN_IN)] {
        let mut ctx = state(
            tap(FELGER, Some(1)),
            "completed",
            "active",
            Some((CAGE_STEP, "active")),
        );
        with_counter(&mut ctx, CAGE_COUNTER, count);
        assert_eq!(
            m381_matched(&fire(&whole, TriggerType::InteractTag, &ctx)),
            BTreeSet::from([expected]),
            "S1409 with counter {count} must match exactly one Felger chain",
        );
    }
}

/// Revert: drop 23040's 1423 gate, or 23041's. With 1423 active, neither the
/// login offer nor the Carter click may match, so Carter offers 381 only after
/// the report. Covered per race in `carter_offer_waits_for_1423`.
#[tokio::test]
async fn carter_click_matches_one_offer_chain_per_state() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let offer = offer_open(tap(CARTER, Some(4)));
    assert_eq!(
        m381_matched(&fire(&whole, TriggerType::InteractTag, &offer)),
        BTreeSet::from([OFFER_SHOW]),
    );
    let started = state(
        tap(CARTER, Some(4)),
        "completed",
        "active",
        Some((CHECKLIST_STEP, "active")),
    );
    assert!(m381_matched(&fire(&whole, TriggerType::InteractTag, &started)).is_empty());
}
