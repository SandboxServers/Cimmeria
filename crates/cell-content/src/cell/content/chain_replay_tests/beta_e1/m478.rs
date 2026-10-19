//! Beta Site E1 478 Instant IT (UB-13): after 473, Carter offers 478 (dialog
//! 3223), the player goes to the Admin Building (S1746: the region, and the
//! workstation fallback), clicks the workstation (S1747, dialog 3224), shuts
//! the worm down (dialog choice 3224, S1749) and Carter's turn-in (1995)
//! completes it (chains 23100-23108). Seeded in
//! `db/resources/Content/Seed/beta_e1_m478_chains.sql`. Other races and
//! unstamped players must match none of them.

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, fire, login_ctx, summarized, whole_seed, with_mission, with_step,
    ALL_ARCHETYPES, BETA, HUMANS,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const OFFER_BIND: i64 = 23100;
const OFFER_SHOW: i64 = 23101;
const ACCEPT: i64 = 23102;
const REGION: i64 = 23103;
const WORKSTATION_FALLBACK: i64 = 23104;
const WORKSTATION: i64 = 23105;
const WORM: i64 = 23106;
const DONE_RESTORE: i64 = 23107;
const TURN_IN: i64 = 23108;

const MISSION: i32 = 478;
const PREREQ: i32 = 473;
const ADMIN_STEP: i32 = 1746;
const WORM_STEP: i32 = 1747;
const DONE_STEP: i32 = 1749;
const ADMIN_REGION: &str = "Beta_Site_Evo_1.AdminBuilding";
const WORKSTATION_TAG: &str = "BetaE1_AdminWorkstation";
const CARTER: &str = "BetaE1_Carter";

/// The 478 chains (23100-23108) that `resolved` matched, for the exclusivity checks.
fn m478_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| (OFFER_BIND..=TURN_IN).contains(id))
        .collect()
}

/// Every chain in the seed (not only 478's) that `resolved` matched and that
/// displays a dialog, so a clash with a UB-10, UB-11, UB-12 or other sibling shows up.
fn dialog_chains(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|&id| {
            summarized(resolved, id)
                .iter()
                .any(|s| s.starts_with("display_dialog("))
        })
        .collect()
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

/// 473 completed, 478 at `status`, and 478's step `step` (when given) active.
fn after_473(mut ctx: ExecutionContext, status: &str, step: Option<i32>) -> ExecutionContext {
    with_mission(&mut ctx, PREREQ, "completed");
    with_mission(&mut ctx, MISSION, status);
    if let Some(s) = step {
        with_step(&mut ctx, MISSION, s, "active");
    }
    ctx
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
    assert_eq!(m478_matched(&resolved), BTreeSet::from([chain]));
    assert_eq!(
        summarized(&resolved, chain),
        list.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    );
    assert_no_deferred_actions(&resolved, chain);
}

/// Revert: drop a mission-completed trigger row of 23100 (login only) or the
/// race gate. A Human who finishes 473 binds Carter's blurb (2460), on login and
/// on the finish, and the whole seed matches only 23100.
#[tokio::test]
async fn offer_bind_after_473_binds_carter_blurb_per_player() {
    let pool = require_db_or_skip!();
    // 23100 has two trigger rows, so only the whole seed holds both paths.
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let login = after_473(login_ctx(BETA, Some(archetype)), "not_active", None);
        assert_chain(
            &whole,
            TriggerType::PlayerLoaded,
            &login,
            OFFER_BIND,
            &["add_dialog_set(2460, slot=2300)"],
        );
        let finished = after_473(
            event("mission_id", serde_json::json!(PREREQ), Some(archetype)),
            "not_active",
            None,
        );
        assert_chain(
            &whole,
            TriggerType::MissionCompleted,
            &finished,
            OFFER_BIND,
            &["add_dialog_set(2460, slot=2300)"],
        );
    }
}

/// Revert: drop 23101's `display_dialog` row (no offer) or 23102's
/// `accept_mission` row (the mission never starts) or its `remove_dialog_set`
/// (the blurb stays bound). Other races and unstamped players match nothing.
#[tokio::test]
async fn carter_offer_shows_and_accept_takes_478() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let click = after_473(
            event("entity_tag", serde_json::json!(CARTER), Some(archetype)),
            "not_active",
            None,
        );
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &click,
            OFFER_SHOW,
            &["display_dialog(3223)"],
        );
        let accept = after_473(
            event("dialog_id", serde_json::json!(3223), Some(archetype)),
            "not_active",
            None,
        );
        assert_chain(
            &whole,
            TriggerType::DialogChoice,
            &accept,
            ACCEPT,
            &["accept_mission(478)", "remove_dialog_set(2460, slot=2300)"],
        );
    }
    for archetype in non_human_or_unstamped() {
        let click = after_473(
            event("entity_tag", serde_json::json!(CARTER), archetype),
            "not_active",
            None,
        );
        let accept = after_473(
            event("dialog_id", serde_json::json!(3223), archetype),
            "not_active",
            None,
        );
        for (trigger, ctx) in [
            (TriggerType::InteractTag, click),
            (TriggerType::DialogChoice, accept),
        ] {
            let resolved = fire(&whole, trigger, &ctx);
            assert!(
                m478_matched(&resolved).is_empty(),
                "archetype {archetype:?} must match no 478 chain",
            );
        }
    }
}

/// Revert: drop 23103's step gate (the region fires at S1747 and S1749 too), or
/// the workstation fallback row 23104 (a player already inside the building who
/// clicks the workstation would not advance).
#[tokio::test]
async fn region_and_workstation_fallback_both_advance_to_1747() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let entered = after_473(
            event(
                "region_key",
                serde_json::json!(ADMIN_REGION),
                Some(archetype),
            ),
            "active",
            Some(ADMIN_STEP),
        );
        assert_chain(
            &whole,
            TriggerType::RegionEnter,
            &entered,
            REGION,
            &["advance_step(478, 1747)"],
        );
        let clicked = after_473(
            event(
                "entity_tag",
                serde_json::json!(WORKSTATION_TAG),
                Some(archetype),
            ),
            "active",
            Some(ADMIN_STEP),
        );
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &clicked,
            WORKSTATION_FALLBACK,
            &["advance_step(478, 1747)", "display_dialog(3224)"],
        );
        // Already at S1747: the region does not fire, and the region chain is idle.
        let inside_at_worm = after_473(
            event(
                "region_key",
                serde_json::json!(ADMIN_REGION),
                Some(archetype),
            ),
            "active",
            Some(WORM_STEP),
        );
        let resolved = fire(&whole, TriggerType::RegionEnter, &inside_at_worm);
        assert!(m478_matched(&resolved).is_empty(), "region at S1747");
    }
    for archetype in non_human_or_unstamped() {
        let entered = after_473(
            event("region_key", serde_json::json!(ADMIN_REGION), archetype),
            "active",
            Some(ADMIN_STEP),
        );
        let clicked = after_473(
            event("entity_tag", serde_json::json!(WORKSTATION_TAG), archetype),
            "active",
            Some(ADMIN_STEP),
        );
        for (trigger, ctx) in [
            (TriggerType::RegionEnter, entered),
            (TriggerType::InteractTag, clicked),
        ] {
            let resolved = fire(&whole, trigger, &ctx);
            assert!(
                m478_matched(&resolved).is_empty(),
                "archetype {archetype:?}"
            );
        }
    }
}

/// Revert: drop the step gate on 23104 or 23105 (a workstation click then matches
/// two chains at one step). The three 478 states are disjoint: S1746 matches
/// only 23104, S1747 only 23105, and S1749 none (Carter is the turn-in).
#[tokio::test]
async fn workstation_states_are_disjoint() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    // (478 step, the one 478 chain that may match a workstation click).
    let cases: [(i32, Option<i64>); 3] = [
        (ADMIN_STEP, Some(WORKSTATION_FALLBACK)),
        (WORM_STEP, Some(WORKSTATION)),
        (DONE_STEP, None),
    ];
    for archetype in HUMANS {
        for (step, expected) in cases {
            let click = after_473(
                event(
                    "entity_tag",
                    serde_json::json!(WORKSTATION_TAG),
                    Some(archetype),
                ),
                "active",
                Some(step),
            );
            let resolved = fire(&whole, TriggerType::InteractTag, &click);
            let expected: BTreeSet<i64> = expected.into_iter().collect();
            assert_eq!(m478_matched(&resolved), expected, "step {step}");
        }
    }
}

/// Revert: drop 23106's `advance_step` (the worm stays at S1747) or its
/// `add_dialog_set` (Carter's turn-in is never bound). Pressing the worm button
/// at S1746 or with 478 not started matches nothing; other races match nothing.
#[tokio::test]
async fn worm_button_advances_to_carter() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let pressed = after_473(
            event("dialog_id", serde_json::json!(3224), Some(archetype)),
            "active",
            Some(WORM_STEP),
        );
        assert_chain(
            &whole,
            TriggerType::DialogChoice,
            &pressed,
            WORM,
            &["advance_step(478, 1749)", "add_dialog_set(3884, slot=2300)"],
        );
        let early = after_473(
            event("dialog_id", serde_json::json!(3224), Some(archetype)),
            "active",
            Some(ADMIN_STEP),
        );
        let resolved = fire(&whole, TriggerType::DialogChoice, &early);
        assert!(m478_matched(&resolved).is_empty(), "worm at S1746");
    }
    for archetype in non_human_or_unstamped() {
        let pressed = after_473(
            event("dialog_id", serde_json::json!(3224), archetype),
            "active",
            Some(WORM_STEP),
        );
        let resolved = fire(&whole, TriggerType::DialogChoice, &pressed);
        assert!(
            m478_matched(&resolved).is_empty(),
            "archetype {archetype:?}"
        );
    }
}

/// Revert: drop 23107's step gate (a relog at S1746 binds the turn-in early), or
/// the bind row of 23107 (a relog at S1749 has no turn-in). Carter's click at
/// S1749 matches 23108 alone and completes the mission; at S1746 and S1747 he
/// shows nothing new.
#[tokio::test]
async fn carter_restore_and_turn_in_complete_478() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let relog = after_473(login_ctx(BETA, Some(archetype)), "active", Some(DONE_STEP));
        assert_chain(
            &whole,
            TriggerType::PlayerLoaded,
            &relog,
            DONE_RESTORE,
            &["add_dialog_set(3884, slot=2300)"],
        );
        let turn_in = after_473(
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
                "display_dialog(1995)",
                "remove_dialog_set(3884, slot=2300)",
                "complete_mission(478)",
            ],
        );
        let early = after_473(login_ctx(BETA, Some(archetype)), "active", Some(ADMIN_STEP));
        let resolved = fire(&whole, TriggerType::PlayerLoaded, &early);
        assert!(m478_matched(&resolved).is_empty(), "relog at S1746");
    }
    for archetype in non_human_or_unstamped() {
        let relog = after_473(login_ctx(BETA, archetype), "active", Some(DONE_STEP));
        let turn_in = after_473(
            event("entity_tag", serde_json::json!(CARTER), archetype),
            "active",
            Some(DONE_STEP),
        );
        for (trigger, ctx) in [
            (TriggerType::PlayerLoaded, relog),
            (TriggerType::InteractTag, turn_in),
        ] {
            let resolved = fire(&whole, trigger, &ctx);
            assert!(
                m478_matched(&resolved).is_empty(),
                "archetype {archetype:?}"
            );
        }
    }
}

/// Revert: drop the step or status gate on 23101 or 23108 (Carter then matches
/// two 478 chains at one state). One Carter click state, one 478 chain at most,
/// and no other chain in the seed displays a dialog on the same click, with 381
/// and 473 done as they are in play (so UB-11's and UB-12's Carter chains are in
/// the count).
#[tokio::test]
async fn carter_one_478_chain_per_state() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    // (478 status, step, the one 478 chain that may match a Carter click).
    let cases: [(&str, Option<i32>, Option<i64>); 5] = [
        ("not_active", None, Some(OFFER_SHOW)),
        ("active", Some(ADMIN_STEP), None),
        ("active", Some(WORM_STEP), None),
        ("active", Some(DONE_STEP), Some(TURN_IN)),
        ("completed", None, None),
    ];
    for archetype in HUMANS {
        for (status, step, expected) in cases {
            let click = after_473(
                event("entity_tag", serde_json::json!(CARTER), Some(archetype)),
                status,
                step,
            );
            let resolved = fire(&whole, TriggerType::InteractTag, &click);
            let expected: BTreeSet<i64> = expected.into_iter().collect();
            assert_eq!(m478_matched(&resolved), expected, "478 {status} {step:?}");
            let mut as_played = after_473(
                event("entity_tag", serde_json::json!(CARTER), Some(archetype)),
                status,
                step,
            );
            with_mission(&mut as_played, 381, "completed");
            let resolved = fire(&whole, TriggerType::InteractTag, &as_played);
            assert_eq!(
                dialog_chains(&resolved),
                expected,
                "whole-seed dialog chains, 478 {status} {step:?}"
            );
        }
    }
}

/// Revert: drop the world filter on the `player_loaded` trigger of 23100 or
/// 23107 (the binds then also fire on a login into another world). A login
/// anywhere but Beta, in a state that would bind on Beta, matches no 478 chain,
/// even for a Human.
#[tokio::test]
async fn logins_in_another_world_match_no_478_chain() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for (world, expected) in [
        ("Some_Other_World", None),
        (BETA, Some((OFFER_BIND, DONE_RESTORE))),
    ] {
        for (status, step, chain) in [
            ("not_active", None, expected.map(|c| c.0)),
            ("active", Some(DONE_STEP), expected.map(|c| c.1)),
        ] {
            let login = after_473(login_ctx(world, Some(HUMANS[0])), status, step);
            let resolved = fire(&whole, TriggerType::PlayerLoaded, &login);
            let want: BTreeSet<i64> = chain.into_iter().collect();
            assert_eq!(m478_matched(&resolved), want, "{world} {status} {step:?}");
        }
    }
}

/// Revert: drop the `mission_status 473 completed` row of 23100 or 23101. Other
/// races, unstamped players, a 473 still active and a 478 already started match
/// nothing.
#[tokio::test]
async fn unfinished_473_and_started_478_match_nothing() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let mut unfinished = login_ctx(BETA, Some(archetype));
        with_mission(&mut unfinished, PREREQ, "active");
        with_mission(&mut unfinished, MISSION, "not_active");
        let resolved = fire(&whole, TriggerType::PlayerLoaded, &unfinished);
        assert!(m478_matched(&resolved).is_empty(), "473 still active");
        let started = after_473(
            event("entity_tag", serde_json::json!(CARTER), Some(archetype)),
            "active",
            Some(ADMIN_STEP),
        );
        let resolved = fire(&whole, TriggerType::InteractTag, &started);
        assert!(m478_matched(&resolved).is_empty(), "478 already started");
    }
}
