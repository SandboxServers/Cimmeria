//! Beta Site E1 473 Inventory (UB-12): after 381, Felger offers 473 (dialog
//! 2018), the player checks the crates in Storage (S1723 to S1724: the region
//! and the crate fallback, then the Arc-42 and Arc-66 partition), Felger's mid
//! talk (2015) and Carter's turn-in (2014) complete it (chains 23070-23083).
//! Seeded in `db/resources/Content/Seed/beta_e1_m473_chains.sql`. Other races
//! and unstamped players must match none of them.

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, fire, login_ctx, summarized, whole_seed, with_mission, with_obj, with_step,
    ALL_ARCHETYPES, BETA, HUMANS,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const OFFER_BIND: i64 = 23070;
const OFFER_SHOW: i64 = 23071;
const ACCEPT: i64 = 23072;
const STORAGE: i64 = 23073;
const ARC42_FALLBACK: i64 = 23074;
const ARC66_FALLBACK: i64 = 23075;
const ARC42_MARK: i64 = 23076;
const ARC42_LAST: i64 = 23077;
const ARC66_MARK: i64 = 23078;
const ARC66_LAST: i64 = 23079;
const MID_RESTORE: i64 = 23080;
const MID_TALK: i64 = 23081;
const DONE_RESTORE: i64 = 23082;
const TURN_IN: i64 = 23083;

const MISSION: i32 = 473;
const PREREQ: i32 = 381;
const STORAGE_STEP: i32 = 1723;
const CRATE_STEP: i32 = 1724;
const MID_STEP: i32 = 1725;
const DONE_STEP: i32 = 1726;
const ARC42_OBJ: i32 = 2037;
const ARC66_OBJ: i32 = 2038;
const REGION: &str = "Beta_Site_Evo_1.Storage";
const ARC42: &str = "BetaE1_Crate_Arc42";
const ARC66: &str = "BetaE1_Crate_Arc66";
const FELGER: &str = "BetaE1_Felger";
const CARTER: &str = "BetaE1_Carter";

/// The 473 chains (23070-23083) that `resolved` matched, for the exclusivity checks.
fn m473_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| (OFFER_BIND..=TURN_IN).contains(id))
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

/// A context with `param` set to `value`, and the archetype stamped when given.
fn event(param: &str, value: serde_json::Value, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param(param.to_string(), value);
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    ctx
}

/// 381 at `prereq`, 473 at `status`, and 473's step `step` (when given) active.
fn state(
    mut ctx: ExecutionContext,
    prereq: &str,
    status: &str,
    step: Option<i32>,
) -> ExecutionContext {
    with_mission(&mut ctx, PREREQ, prereq);
    with_mission(&mut ctx, MISSION, status);
    if let Some(s) = step {
        with_step(&mut ctx, MISSION, s, "active");
    }
    ctx
}

/// A Human-side state where 381 is done and 473 is at `status`.
fn done_381(ctx: ExecutionContext, status: &str, step: Option<i32>) -> ExecutionContext {
    state(ctx, "completed", status, step)
}

/// The offer state: 381 done, 473 not started.
fn offer_open(ctx: ExecutionContext) -> ExecutionContext {
    done_381(ctx, "not_active", None)
}

/// Pass the context to `fire` and assert that `chain` matched with `list`.
fn assert_chain(
    engine: &cimmeria_content_engine::chain::ChainEngine,
    trigger: TriggerType,
    ctx: &ExecutionContext,
    chain: i64,
    list: &[&str],
) {
    let resolved = fire(engine, trigger, ctx);
    assert_eq!(m473_matched(&resolved), BTreeSet::from([chain]));
    assert_eq!(summarized(&resolved, chain), expect(list));
    assert_no_deferred_actions(&resolved, chain);
}

/// Revert: drop the mission-completed trigger row of 23070, or the race gate.
/// A Human who finishes 381 binds Felger's blurb (2440), on login and on the
/// finish, and the whole seed matches only 23070.
#[tokio::test]
async fn offer_bind_after_381_binds_felger_blurb_per_player() {
    let pool = require_db_or_skip!();
    // 23070 has two trigger rows, so only the whole seed holds both paths.
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let login = offer_open(login_ctx(BETA, Some(archetype)));
        assert_chain(
            &whole,
            TriggerType::PlayerLoaded,
            &login,
            OFFER_BIND,
            &["add_dialog_set(2440, slot=2301)"],
        );
        let finished = offer_open(event(
            "mission_id",
            serde_json::json!(PREREQ),
            Some(archetype),
        ));
        assert_chain(
            &whole,
            TriggerType::MissionCompleted,
            &finished,
            OFFER_BIND,
            &["add_dialog_set(2440, slot=2301)"],
        );
    }
}

/// Revert: drop a gate row on 23070, 23071 or 23072 (race, 381 completed, or
/// 473 not_active). Other races, unstamped players, a 381 still active and a
/// 473 that has started must all match nothing.
#[tokio::test]
async fn offer_gates_reject_other_races_unstamped_and_unfinished_381() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in non_human_or_unstamped() {
        let login = offer_open(login_ctx(BETA, archetype));
        let finished = offer_open(event("mission_id", serde_json::json!(PREREQ), archetype));
        let click = offer_open(event("entity_tag", serde_json::json!(FELGER), archetype));
        let accept = offer_open(event("dialog_id", serde_json::json!(2018), archetype));
        for (trigger, ctx) in [
            (TriggerType::PlayerLoaded, login),
            (TriggerType::MissionCompleted, finished),
            (TriggerType::InteractTag, click),
            (TriggerType::DialogChoice, accept),
        ] {
            let resolved = fire(&whole, trigger, &ctx);
            assert!(
                m473_matched(&resolved).is_empty(),
                "archetype {archetype:?} must match no 473 chain",
            );
        }
    }
    for archetype in HUMANS {
        let unfinished = state(
            login_ctx(BETA, Some(archetype)),
            "active",
            "not_active",
            None,
        );
        let resolved = fire(&whole, TriggerType::PlayerLoaded, &unfinished);
        assert!(m473_matched(&resolved).is_empty(), "381 still active");
        let started = done_381(
            event("entity_tag", serde_json::json!(FELGER), Some(archetype)),
            "active",
            Some(STORAGE_STEP),
        );
        let resolved = fire(&whole, TriggerType::InteractTag, &started);
        assert!(m473_matched(&resolved).is_empty(), "473 already started");
    }
}

/// Revert: drop the `mission_status 473 not_active` row on 23071. The offer then
/// shows again after the player has taken the mission.
#[tokio::test]
async fn felger_click_shows_the_offer_once_381_is_done() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let click = offer_open(event(
            "entity_tag",
            serde_json::json!(FELGER),
            Some(archetype),
        ));
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &click,
            OFFER_SHOW,
            &["display_dialog(2018)"],
        );
    }
}

/// Revert: drop 23072's `remove_dialog_set` (the blurb stays bound) or its
/// `accept_mission` row (the mission never starts). Other races match nothing.
#[tokio::test]
async fn accept_on_dialog_2018_takes_the_mission() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let accept = offer_open(event("dialog_id", serde_json::json!(2018), Some(archetype)));
        assert_chain(
            &whole,
            TriggerType::DialogChoice,
            &accept,
            ACCEPT,
            &["accept_mission(473)", "remove_dialog_set(2440, slot=2301)"],
        );
    }
    for archetype in non_human_or_unstamped() {
        let accept = offer_open(event("dialog_id", serde_json::json!(2018), archetype));
        let resolved = fire(&whole, TriggerType::DialogChoice, &accept);
        assert!(
            m473_matched(&resolved).is_empty(),
            "archetype {archetype:?}"
        );
    }
}

/// Revert: drop the step gate on 23073 (the region fires at S1724 too), or the
/// crate fallback rows 23074 and 23075 (a player already in Storage who clicks
/// a crate would not advance).
#[tokio::test]
async fn storage_entry_and_crate_fallback_both_advance_to_1724() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let entered = done_381(
            event("region_key", serde_json::json!(REGION), Some(archetype)),
            "active",
            Some(STORAGE_STEP),
        );
        assert_chain(
            &whole,
            TriggerType::RegionEnter,
            &entered,
            STORAGE,
            &["advance_step(473, 1724)"],
        );
        for (tag, chain) in [(ARC42, ARC42_FALLBACK), (ARC66, ARC66_FALLBACK)] {
            let clicked = done_381(
                event("entity_tag", serde_json::json!(tag), Some(archetype)),
                "active",
                Some(STORAGE_STEP),
            );
            assert_chain(
                &whole,
                TriggerType::InteractTag,
                &clicked,
                chain,
                &["advance_step(473, 1724)"],
            );
        }
        // Past S1723, neither the region nor the crates advance again.
        let past = done_381(
            event("region_key", serde_json::json!(REGION), Some(archetype)),
            "active",
            Some(CRATE_STEP),
        );
        let resolved = fire(&whole, TriggerType::RegionEnter, &past);
        assert!(
            m473_matched(&resolved).is_empty(),
            "S1724 is not storage entry"
        );
    }
    for archetype in non_human_or_unstamped() {
        let entered = done_381(
            event("region_key", serde_json::json!(REGION), archetype),
            "active",
            Some(STORAGE_STEP),
        );
        let resolved = fire(&whole, TriggerType::RegionEnter, &entered);
        assert!(
            m473_matched(&resolved).is_empty(),
            "archetype {archetype:?}"
        );
    }
}

/// Revert: drop the `objective_status 2038 neq completed` row from 23076. The
/// open/done case then matches 23076 and 23077 together. Revert the other way
/// (a 2037 `done` gate on 23079) and the done/open case matches nothing.
#[tokio::test]
async fn crate_partition_is_disjoint_and_never_completes_the_last_objective() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    // (crate tag, O2037 status, O2038 status, the one chain that may match).
    let cases: [(&str, &str, &str, Option<i64>); 8] = [
        (ARC42, "active", "active", Some(ARC42_MARK)),
        (ARC42, "active", "completed", Some(ARC42_LAST)),
        (ARC42, "completed", "active", None),
        (ARC42, "completed", "completed", None),
        (ARC66, "active", "active", Some(ARC66_MARK)),
        (ARC66, "completed", "active", Some(ARC66_LAST)),
        (ARC66, "active", "completed", None),
        (ARC66, "completed", "completed", None),
    ];
    for (tag, arc42, arc66, expected) in cases {
        let mut click = done_381(
            event("entity_tag", serde_json::json!(tag), Some(HUMANS[0])),
            "active",
            Some(CRATE_STEP),
        );
        with_obj(&mut click, MISSION, ARC42_OBJ, arc42);
        with_obj(&mut click, MISSION, ARC66_OBJ, arc66);
        let resolved = fire(&whole, TriggerType::InteractTag, &click);
        let expected: BTreeSet<i64> = expected.into_iter().collect();
        assert_eq!(
            m473_matched(&resolved),
            expected,
            "{tag} with O2037 {arc42} and O2038 {arc66}",
        );
        for chain in &expected {
            assert_no_deferred_actions(&resolved, *chain);
            // Only the mark chains complete an objective; the last ones advance.
            let acts = summarized(&resolved, *chain);
            let completes = acts.iter().any(|a| a.starts_with("complete_objective"));
            assert_eq!(completes, [ARC42_MARK, ARC66_MARK].contains(chain));
            assert!(!acts.iter().any(|a| a.starts_with("complete_mission")));
        }
    }
}

/// Revert: drop the step gate on 23081 or 23083 (a Felger or Carter click then
/// matches two chains at one step).
#[tokio::test]
async fn felger_and_carter_one_dialog_chain_per_state() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    // (tag, 473 status, step, the one 473 chain that may match).
    let cases: [(&str, &str, Option<i32>, Option<i64>); 8] = [
        (FELGER, "not_active", None, Some(OFFER_SHOW)),
        (FELGER, "active", Some(STORAGE_STEP), None),
        (FELGER, "active", Some(CRATE_STEP), None),
        (FELGER, "active", Some(MID_STEP), Some(MID_TALK)),
        (FELGER, "active", Some(DONE_STEP), None),
        (CARTER, "not_active", None, None),
        (CARTER, "active", Some(MID_STEP), None),
        (CARTER, "active", Some(DONE_STEP), Some(TURN_IN)),
    ];
    for (tag, status, step, expected) in cases {
        let click = done_381(
            event("entity_tag", serde_json::json!(tag), Some(HUMANS[0])),
            status,
            step,
        );
        let resolved = fire(&whole, TriggerType::InteractTag, &click);
        let expected: BTreeSet<i64> = expected.into_iter().collect();
        assert_eq!(
            m473_matched(&resolved),
            expected,
            "{tag} at {status} {step:?}"
        );
    }
}

/// Revert: drop 23080 (a relog on S1725 leaves Felger without the mid bind),
/// or the Carter bind on 23081 (Carter never offers the turn-in after the mid
/// talk). The turn-in on 23083 must complete 473 and nothing else.
#[tokio::test]
async fn felger_mid_talk_and_relog_restore_carter_turn_in() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let relog_mid = done_381(login_ctx(BETA, Some(archetype)), "active", Some(MID_STEP));
        assert_chain(
            &whole,
            TriggerType::PlayerLoaded,
            &relog_mid,
            MID_RESTORE,
            &["add_dialog_set(2439, slot=2301)"],
        );
        let talk = done_381(
            event("entity_tag", serde_json::json!(FELGER), Some(archetype)),
            "active",
            Some(MID_STEP),
        );
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &talk,
            MID_TALK,
            &[
                "display_dialog(2015)",
                "remove_dialog_set(2439, slot=2301)",
                "advance_step(473, 1726)",
                "add_dialog_set(2438, slot=2300)",
            ],
        );
        let relog_done = done_381(login_ctx(BETA, Some(archetype)), "active", Some(DONE_STEP));
        assert_chain(
            &whole,
            TriggerType::PlayerLoaded,
            &relog_done,
            DONE_RESTORE,
            &["add_dialog_set(2438, slot=2300)"],
        );
        let turn_in = done_381(
            event("entity_tag", serde_json::json!(CARTER), Some(archetype)),
            "active",
            Some(DONE_STEP),
        );
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &turn_in,
            TURN_IN,
            &[
                "display_dialog(2014)",
                "remove_dialog_set(2438, slot=2300)",
                "complete_mission(473)",
            ],
        );
    }
    // A login at the crate step binds nothing, and no non-Human matches.
    let crates = done_381(login_ctx(BETA, Some(HUMANS[0])), "active", Some(CRATE_STEP));
    assert!(m473_matched(&fire(&whole, TriggerType::PlayerLoaded, &crates)).is_empty());
    for archetype in non_human_or_unstamped() {
        let relog = done_381(login_ctx(BETA, archetype), "active", Some(MID_STEP));
        let resolved = fire(&whole, TriggerType::PlayerLoaded, &relog);
        assert!(
            m473_matched(&resolved).is_empty(),
            "archetype {archetype:?}"
        );
    }
}
