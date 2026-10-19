//! Beta Site E1 mission 1502 The Nine Peaks Jaffa (UB-21): a Free Jaffa who has
//! 1500 completed is offered the medallion job by Ras'nor (chains 23240-23242).
//! The walk into the camp (23243) or a box click (23244) starts the box search,
//! two box buttons (23246, 23248) grant medallion 5175, a relog re-binds the
//! done dialog (23249) and Ras'nor's turn-in (23250) removes the medallion and
//! completes 1502. Seeded in `db/resources/Content/Seed/beta_e1_m1502_chains.sql`.
//! The gate is archetype 7 (Free Jaffa) only: Jaffa (8) is Praxis and every
//! other archetype, including unset, is wrong.

use std::collections::BTreeSet;
use std::ops::RangeInclusive;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, fire, login_ctx, summarized, whole_seed, with_mission, with_step,
    ALL_ARCHETYPES, BETA, FREE_JAFFA,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

/// The region that the go-to step S4376 waits for (`point_sets` 23060).
const NINE_PEAKS: &str = "Beta_Site_Evo_1.NinePeaks";
/// The mission 1502 block of the seed (`beta_e1_m1502_chains.sql`).
const M1502_CHAINS: RangeInclusive<i64> = 23240..=23279;

/// One positive case: the chain, the trigger and context that fire it, and the
/// exact actions it resolves to.
struct Case {
    chain: i64,
    trigger: TriggerType,
    ctx: ExecutionContext,
    expected: &'static [&'static str],
}

fn case(
    chain: i64,
    trigger: TriggerType,
    ctx: ExecutionContext,
    expected: &'static [&'static str],
) -> Case {
    Case {
        chain,
        trigger,
        ctx,
        expected,
    }
}

/// The 1502 chains that `resolved` matched, for the exclusivity checks.
pub(super) fn m1502_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| M1502_CHAINS.contains(id))
        .collect()
}

/// Every archetype a Free Jaffa gate must refuse: unset, 0-6 and 8 (Jaffa).
fn not_free_jaffa() -> Vec<Option<i64>> {
    let mut archetypes = vec![None];
    archetypes.extend(
        ALL_ARCHETYPES
            .iter()
            .filter(|a| **a != FREE_JAFFA)
            .map(|a| Some(*a)),
    );
    archetypes
}

/// A context carrying one event parameter, plus the archetype when it is set.
pub(super) fn event_ctx(
    key: &str,
    value: serde_json::Value,
    archetype: Option<i64>,
) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param(key.to_string(), value);
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    ctx
}

/// Stage the offer gate: 1500 completed and 1502 `not_active`.
pub(super) fn stage_offer(mut ctx: ExecutionContext) -> ExecutionContext {
    with_mission(&mut ctx, 1500, "completed");
    with_mission(&mut ctx, 1502, "not_active");
    ctx
}

/// Stage 1500 completed and 1502 active, with `step` at `status`.
pub(super) fn stage_step(mut ctx: ExecutionContext, step: i32, status: &str) -> ExecutionContext {
    with_mission(&mut ctx, 1500, "completed");
    with_mission(&mut ctx, 1502, "active");
    with_step(&mut ctx, 1502, step, status);
    ctx
}

/// Each chain's positive case for a player of `archetype`. The state is the one
/// that chain's gates ask for, so the Free Jaffa list is the expected behaviour.
fn cases(archetype: Option<i64>) -> Vec<Case> {
    vec![
        case(
            23240,
            TriggerType::PlayerLoaded,
            stage_offer(login_ctx(BETA, archetype)),
            &["add_dialog_set(5885, slot=2306)"],
        ),
        case(
            23240,
            TriggerType::MissionCompleted,
            stage_offer(event_ctx("mission_id", serde_json::json!(1500), archetype)),
            &["add_dialog_set(5885, slot=2306)"],
        ),
        case(
            23241,
            TriggerType::InteractTag,
            stage_offer(event_ctx(
                "entity_tag",
                serde_json::json!("BetaE1_RasNor"),
                archetype,
            )),
            &["display_dialog(5041)"],
        ),
        case(
            23242,
            TriggerType::DialogChoice,
            stage_offer(event_ctx("dialog_id", serde_json::json!(5041), archetype)),
            &["accept_mission(1502)", "remove_dialog_set(5885, slot=2306)"],
        ),
        case(
            23243,
            TriggerType::RegionEnter,
            stage_step(
                event_ctx("region_key", serde_json::json!(NINE_PEAKS), archetype),
                4376,
                "active",
            ),
            &["advance_step(1502, 4377)"],
        ),
        case(
            23244,
            TriggerType::InteractTag,
            stage_step(
                event_ctx(
                    "entity_tag",
                    serde_json::json!("BetaE1_JaffaBox"),
                    archetype,
                ),
                4376,
                "active",
            ),
            &["advance_step(1502, 4377)", "display_dialog(5042)"],
        ),
        case(
            23245,
            TriggerType::InteractTag,
            stage_step(
                event_ctx(
                    "entity_tag",
                    serde_json::json!("BetaE1_JaffaBox"),
                    archetype,
                ),
                4377,
                "active",
            ),
            &["display_dialog(5042)"],
        ),
        case(
            23246,
            TriggerType::DialogChoice,
            stage_step(
                event_ctx("dialog_id", serde_json::json!(5042), archetype),
                4377,
                "active",
            ),
            &["advance_step(1502, 4378)", "display_dialog(5043)"],
        ),
        case(
            23247,
            TriggerType::InteractTag,
            stage_step(
                event_ctx(
                    "entity_tag",
                    serde_json::json!("BetaE1_JaffaBox"),
                    archetype,
                ),
                4378,
                "active",
            ),
            &["display_dialog(5043)"],
        ),
        case(
            23248,
            TriggerType::DialogChoice,
            stage_step(
                event_ctx("dialog_id", serde_json::json!(5043), archetype),
                4378,
                "active",
            ),
            &[
                "add_item(5175, 1)",
                "advance_step(1502, 4481)",
                "add_dialog_set(5887, slot=2306)",
            ],
        ),
        case(
            23249,
            TriggerType::PlayerLoaded,
            stage_step(login_ctx(BETA, archetype), 4481, "active"),
            &["add_dialog_set(5887, slot=2306)"],
        ),
        case(
            23250,
            TriggerType::InteractTag,
            stage_step(
                event_ctx("entity_tag", serde_json::json!("BetaE1_RasNor"), archetype),
                4481,
                "active",
            ),
            &[
                "display_dialog(5044)",
                "remove_dialog_set(5887, slot=2306)",
                "remove_item(5175, 1)",
                "complete_mission(1502)",
            ],
        ),
    ]
}

/// Revert: change any one action of a chain, drop a row, or drop its
/// `step_status` or `mission_status` gate, so the resolved list differs. Each
/// chain must resolve exactly its packet list. The whole seed is used because
/// a two-trigger chain (23240) loads one engine entry per trigger row, and
/// `engine_for` registers only the first.
#[tokio::test]
async fn live_db_each_chain_resolves_its_exact_actions() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for case in cases(Some(FREE_JAFFA)) {
        let resolved = fire(&whole, case.trigger, &case.ctx);
        let expected: Vec<String> = case.expected.iter().map(|s| s.to_string()).collect();
        assert_eq!(
            summarized(&resolved, case.chain),
            expected,
            "chain {} must resolve its packet list",
            case.chain,
        );
        assert_no_deferred_actions(&resolved, case.chain);
    }
}

/// Revert: a sibling chain on the same trigger key that also gates on a Free
/// Jaffa (a duplicate row, or a dropped `step_status` row). Each positive case
/// must match exactly its own chain in the whole seed.
#[tokio::test]
async fn live_db_free_jaffa_case_matches_exactly_its_own_chain() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for case in cases(Some(FREE_JAFFA)) {
        let matched = m1502_matched(&fire(&whole, case.trigger, &case.ctx));
        assert_eq!(
            matched,
            BTreeSet::from([case.chain]),
            "chain {}'s case must match only itself",
            case.chain,
        );
    }
}

/// Revert: drop the archetype row of any 1502 chain, or widen it (to a Human,
/// Asgard, Goa'uld, Jaffa or unset player). Every 1502 positive case with any
/// archetype but 7 must match nothing in 23240-23279.
#[tokio::test]
async fn live_db_ras_nor_chains_are_free_jaffa_only() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in not_free_jaffa() {
        for case in cases(archetype) {
            let matched = m1502_matched(&fire(&whole, case.trigger, &case.ctx));
            assert!(
                matched.is_empty(),
                "archetype {archetype:?} on chain {}'s case matched {matched:?}",
                case.chain,
            );
        }
    }
}

/// Revert: drop the `mission_status` gate of 23240-23242 (the offer would
/// show before 1500 is done, or again after 1502 is accepted), or the Ras'nor
/// click gate of 23241. Only the offer state fires the offer chains.
#[tokio::test]
async fn live_db_offer_waits_for_1500_and_is_not_repeated() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;

    let mut login_1500_active = login_ctx(BETA, Some(FREE_JAFFA));
    with_mission(&mut login_1500_active, 1500, "active");
    with_mission(&mut login_1500_active, 1502, "not_active");
    assert!(
        m1502_matched(&fire(&whole, TriggerType::PlayerLoaded, &login_1500_active)).is_empty(),
        "a Free Jaffa with 1500 still active must not be offered 1502",
    );

    let mut click_1500_active = event_ctx(
        "entity_tag",
        serde_json::json!("BetaE1_RasNor"),
        Some(FREE_JAFFA),
    );
    with_mission(&mut click_1500_active, 1500, "active");
    with_mission(&mut click_1500_active, 1502, "not_active");
    assert!(
        m1502_matched(&fire(&whole, TriggerType::InteractTag, &click_1500_active)).is_empty(),
        "Ras'nor's offer show must wait for 1500",
    );

    let mut accept_1500_active = event_ctx("dialog_id", serde_json::json!(5041), Some(FREE_JAFFA));
    with_mission(&mut accept_1500_active, 1500, "active");
    with_mission(&mut accept_1500_active, 1502, "not_active");
    assert!(
        m1502_matched(&fire(
            &whole,
            TriggerType::DialogChoice,
            &accept_1500_active
        ))
        .is_empty(),
        "accepting 5041 must wait for 1500",
    );

    let accepted = stage_step(login_ctx(BETA, Some(FREE_JAFFA)), 4376, "active");
    assert!(
        m1502_matched(&fire(&whole, TriggerType::PlayerLoaded, &accepted)).is_empty(),
        "an accepted 1502 at S4376 must not re-bind the offer",
    );
}

/// Revert: drop 23243's region row or its `step_status` row, so the camp
/// trigger also fires at S4377. The region matches only while S4376 is active.
#[tokio::test]
async fn live_db_camp_region_advances_only_from_s4376() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let at = |step: i32| {
        stage_step(
            event_ctx(
                "region_key",
                serde_json::json!(NINE_PEAKS),
                Some(FREE_JAFFA),
            ),
            step,
            "active",
        )
    };
    assert_eq!(
        m1502_matched(&fire(&whole, TriggerType::RegionEnter, &at(4376))),
        BTreeSet::from([23243]),
        "entering the camp at S4376 must match exactly 23243",
    );
    for step in [4377, 4378, 4481] {
        assert!(
            m1502_matched(&fire(&whole, TriggerType::RegionEnter, &at(step))).is_empty(),
            "entering the camp at S{step} must match no 1502 chain",
        );
    }
}

/// Revert: drop one box chain's `step_status` row, or let two box chains share
/// a step. A box click matches exactly one chain at each box step, and none
/// once the medallion is in hand (S4481).
#[tokio::test]
async fn live_db_box_states_are_disjoint() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for (step, expected) in [
        (4376, Some(23244)),
        (4377, Some(23245)),
        (4378, Some(23247)),
        (4481, None),
    ] {
        let ctx = stage_step(
            event_ctx(
                "entity_tag",
                serde_json::json!("BetaE1_JaffaBox"),
                Some(FREE_JAFFA),
            ),
            step,
            "active",
        );
        let want: BTreeSet<i64> = expected.into_iter().collect();
        assert_eq!(
            m1502_matched(&fire(&whole, TriggerType::InteractTag, &ctx)),
            want,
            "a box click at S{step} must match {want:?}",
        );
    }
}

/// Revert: drop 23248's `add_item` (the medallion is never granted), drop
/// 23250's `remove_item` (the player keeps it), or let 23248 or 23250 fire at
/// the wrong step. The grant is at S4378 only and the return at S4481 only.
#[tokio::test]
async fn live_db_medallion_granted_once_and_returned() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;

    let grant = stage_step(
        event_ctx("dialog_id", serde_json::json!(5043), Some(FREE_JAFFA)),
        4378,
        "active",
    );
    let resolved = fire(&whole, TriggerType::DialogChoice, &grant);
    assert_eq!(m1502_matched(&resolved), BTreeSet::from([23248]));
    assert_eq!(
        summarized(&resolved, 23248),
        vec![
            "add_item(5175, 1)".to_string(),
            "advance_step(1502, 4481)".to_string(),
            "add_dialog_set(5887, slot=2306)".to_string(),
        ],
        "the second box button must grant 5175 and move to S4481",
    );

    let regrant = stage_step(
        event_ctx("dialog_id", serde_json::json!(5043), Some(FREE_JAFFA)),
        4481,
        "active",
    );
    assert!(
        m1502_matched(&fire(&whole, TriggerType::DialogChoice, &regrant)).is_empty(),
        "the second box button must not grant 5175 again at S4481",
    );

    let early_return = stage_step(
        event_ctx(
            "entity_tag",
            serde_json::json!("BetaE1_RasNor"),
            Some(FREE_JAFFA),
        ),
        4378,
        "active",
    );
    assert!(
        m1502_matched(&fire(&whole, TriggerType::InteractTag, &early_return)).is_empty(),
        "Ras'nor must not take the medallion before S4481",
    );

    let ret = stage_step(
        event_ctx(
            "entity_tag",
            serde_json::json!("BetaE1_RasNor"),
            Some(FREE_JAFFA),
        ),
        4481,
        "active",
    );
    let resolved = fire(&whole, TriggerType::InteractTag, &ret);
    assert_eq!(m1502_matched(&resolved), BTreeSet::from([23250]));
    assert_eq!(
        summarized(&resolved, 23250),
        vec![
            "display_dialog(5044)".to_string(),
            "remove_dialog_set(5887, slot=2306)".to_string(),
            "remove_item(5175, 1)".to_string(),
            "complete_mission(1502)".to_string(),
        ],
        "Ras'nor's turn-in must take 5175 and complete 1502",
    );
}

/// Revert: drop 23249's `step_status` row (the done dialog is bound at every
/// relog) or its archetype gate. Only a Free Jaffa at S4481 re-binds 5887; a
/// Free Jaffa at an earlier step, or a finished 1502, gets nothing.
#[tokio::test]
async fn live_db_restore_rebinds_done_dialog_only_at_s4481() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for step in [4376, 4377, 4378] {
        let ctx = stage_step(login_ctx(BETA, Some(FREE_JAFFA)), step, "active");
        assert!(
            m1502_matched(&fire(&whole, TriggerType::PlayerLoaded, &ctx)).is_empty(),
            "a Free Jaffa login at S{step} must not bind 5887",
        );
    }
    let mut finished = login_ctx(BETA, Some(FREE_JAFFA));
    with_mission(&mut finished, 1500, "completed");
    with_mission(&mut finished, 1502, "completed");
    assert!(
        m1502_matched(&fire(&whole, TriggerType::PlayerLoaded, &finished)).is_empty(),
        "a completed 1502 must not re-bind 5887",
    );
    for archetype in not_free_jaffa() {
        let ctx = stage_step(login_ctx(BETA, archetype), 4481, "active");
        assert!(
            m1502_matched(&fire(&whole, TriggerType::PlayerLoaded, &ctx)).is_empty(),
            "archetype {archetype:?} at S4481 must not re-bind 5887",
        );
    }
}
