//! Beta Site E1 Covert Observations (UB-35, mission 477): the Mimir blurb bind
//! (23435), the offer (23436), the Accept, the blurb drop and the four subject
//! binds (23437), the subject partition (23438-23453), the subject relog
//! restores (23454-23457), the completion bind (23458) and the Mimir turn-in
//! (23459). Seeded in `db/resources/Content/Seed/beta_e1_m477_chains.sql`.
//!
//! Omega UO-16 is HELD (HELD-B16), so no live Asgard reaches the subjects
//! through Beta yet. These tests replay archetype 5 (D-UB13).

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, engine_for, fire, login_ctx, summarized, whole_seed, with_mission, with_obj,
    with_step, ALL_ARCHETYPES, ASGARD, BETA,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const M477_CHAINS: std::ops::RangeInclusive<i64> = 23435..=23459;
const MIMIR: &str = "BetaE1_Mimir";
/// A world other than Beta, for the `player_loaded` world-key negatives.
const OTHER_WORLD: &str = "Castle_Cellblock";
const OFFER_DIALOG: i64 = 1997;
const COMPLETION_DIALOG: i64 = 1996;
const RECORDING_DEVICE: i64 = 2646;
const BLURB_DSM: i64 = 2457;
const COMPLETION_DSM: i64 = 4408;
const MIMIR_SLOT: i64 = 2305;
/// The four objectives of S1744 in objective order (Carter, Debeaux, Felger, Satterfield).
const OBJECTIVES: [i32; 4] = [2060, 2061, 2062, 2063];
/// The four subjects: tag, dialog slot, and the first of its four partition chains.
const SUBJECTS: [(&str, i64, i64); 4] = [
    ("BetaE1_Carter", 2300, 23438),
    ("BetaE1_Debeaux", 2304, 23442),
    ("BetaE1_Felger", 2301, 23446),
    ("BetaE1_Satterfield", 2302, 23450),
];
const SUBJECT_TAGS: [&str; 4] = [
    "BetaE1_Carter",
    "BetaE1_Debeaux",
    "BetaE1_Felger",
    "BetaE1_Satterfield",
];

/// The 477 chains that matched in `resolved`, ignoring every other packet's chains.
fn m477_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| M477_CHAINS.contains(id))
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

/// A `mission_completed` context on `mission_id`. `None` leaves the archetype unset.
fn completed(mission_id: i64, archetype: Option<i64>) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param("mission_id".to_string(), serde_json::json!(mission_id));
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    ctx
}

/// Every archetype that is not Asgard, plus an unstamped context (`None`).
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

/// Mission 443 completed, and 477 either at the offer (`step` `None`) or
/// active at `step` with the objectives `open[i]` (true = active, false =
/// completed) in objective order.
fn set_477(ctx: &mut ExecutionContext, step: Option<i32>, open: [bool; 4]) {
    with_mission(ctx, 443, "completed");
    match step {
        None => with_mission(ctx, 477, "not_active"),
        Some(s) => {
            with_mission(ctx, 477, "active");
            with_step(ctx, 477, s, "active");
        }
    }
    for (i, obj) in OBJECTIVES.iter().enumerate() {
        let status = if open[i] { "active" } else { "completed" };
        with_obj(ctx, 477, *obj, status);
    }
}

/// The partition chain that the subject `si` (index into `SUBJECTS`) takes
/// when the objectives are `open`, or `None` when the subject is recorded.
fn partition_chain(si: usize, open: [bool; 4]) -> Option<i64> {
    if !open[si] {
        return None;
    }
    let others: Vec<usize> = (0..4).filter(|j| *j != si).collect();
    let kind = others.iter().position(|j| open[*j]).unwrap_or(3);
    Some(SUBJECTS[si].2 + kind as i64)
}

/// The 16 open/done combinations of the four objectives.
fn all_states() -> Vec<[bool; 4]> {
    (0..16u32)
        .map(|mask| std::array::from_fn(|i| mask & (1 << i) != 0))
        .collect()
}

#[tokio::test]
async fn offer_bind_and_offer_show_are_exact() {
    let pool = require_db_or_skip!();
    // Whole seed: the single-chain engine does not carry the mission_completed trigger.
    let bind = whole_seed(&pool).await;
    let mut ctx = completed(443, Some(ASGARD));
    set_477(&mut ctx, None, [false; 4]);
    let resolved = fire(&bind, TriggerType::MissionCompleted, &ctx);
    assert_no_deferred_actions(&resolved, 23435);
    assert_eq!(
        summarized(&resolved, 23435),
        vec![format!("add_dialog_set({BLURB_DSM}, slot={MIMIR_SLOT})")]
    );

    let show = engine_for(&pool, 23436).await;
    let mut ctx = click(MIMIR, Some(ASGARD));
    set_477(&mut ctx, None, [false; 4]);
    let resolved = fire(&show, TriggerType::InteractTag, &ctx);
    assert_no_deferred_actions(&resolved, 23436);
    assert_eq!(
        summarized(&resolved, 23436),
        vec![format!("display_dialog({OFFER_DIALOG})")]
    );
}

/// Revert: a 477 offer chain without its archetype gate matches a Human.
#[tokio::test]
async fn offer_chains_need_asgard_and_the_offer_state() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        let mut ctx = completed(443, archetype);
        set_477(&mut ctx, None, [false; 4]);
        let got = m477_matched(&fire(&engine, TriggerType::MissionCompleted, &ctx));
        assert!(
            got.is_empty(),
            "23435 must not match {archetype:?}: {got:?}"
        );
        let mut ctx = click(MIMIR, archetype);
        set_477(&mut ctx, None, [false; 4]);
        let got = m477_matched(&fire(&engine, TriggerType::InteractTag, &ctx));
        assert!(
            got.is_empty(),
            "23436 must not match {archetype:?}: {got:?}"
        );
    }
    // Mission 477 already accepted: the offer is no longer shown.
    let mut ctx = click(MIMIR, Some(ASGARD));
    set_477(&mut ctx, Some(1744), [true; 4]);
    let got = m477_matched(&fire(&engine, TriggerType::InteractTag, &ctx));
    assert!(!got.contains(&23436), "23436 needs 477 not_active: {got:?}");
}

#[tokio::test]
async fn accept_starts_477_and_binds_the_subjects() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23437).await;
    let mut ctx = choice(OFFER_DIALOG, Some(ASGARD));
    set_477(&mut ctx, None, [false; 4]);
    let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
    assert_no_deferred_actions(&resolved, 23437);
    assert_eq!(
        summarized(&resolved, 23437),
        vec![
            "accept_mission(477)".to_string(),
            "remove_dialog_set(2457, slot=2305)".to_string(),
            format!("add_item({RECORDING_DEVICE}, 1)"),
            "add_dialog_set(2457, slot=2300)".to_string(),
            "add_dialog_set(2457, slot=2304)".to_string(),
            "add_dialog_set(2457, slot=2301)".to_string(),
            "add_dialog_set(2457, slot=2302)".to_string(),
        ]
    );
}

/// Revert: drop 23435's `player_loaded` trigger row and a relog in the offer
/// state (443 done, 477 not started) loses the Mimir blurb bind; drop its
/// `Beta_Site_Evo_1` event key and another world binds it.
#[tokio::test]
async fn offer_blurb_binds_on_login_in_beta_only() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let mut ctx = login_ctx(BETA, Some(ASGARD));
    set_477(&mut ctx, None, [false; 4]);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
    assert_no_deferred_actions(&resolved, 23435);
    assert_eq!(
        summarized(&resolved, 23435),
        vec![format!("add_dialog_set({BLURB_DSM}, slot={MIMIR_SLOT})")]
    );
    assert_eq!(m477_matched(&resolved), BTreeSet::from([23435]));

    let mut ctx = login_ctx(OTHER_WORLD, Some(ASGARD));
    set_477(&mut ctx, None, [false; 4]);
    let got = m477_matched(&fire(&engine, TriggerType::PlayerLoaded, &ctx));
    assert!(got.is_empty(), "23435 must stay in Beta: {got:?}");
}

/// The four relog restores follow the open objectives: in every one of the 16
/// objective states a login at S1744 matches exactly the restore of each open
/// subject (23454-23457), and none in another world.
///
/// Revert: drop `O<n> active` from one restore and a recorded subject is
/// re-bound; drop the world key and another world binds the subjects.
#[tokio::test]
async fn relog_restores_follow_the_open_objectives_in_beta_only() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for open in all_states() {
        let mut ctx = login_ctx(BETA, Some(ASGARD));
        set_477(&mut ctx, Some(1744), open);
        let want: BTreeSet<i64> = (0..4)
            .filter(|i| open[*i])
            .map(|i| 23454 + i as i64)
            .collect();
        let got = m477_matched(&fire(&engine, TriggerType::PlayerLoaded, &ctx));
        assert_eq!(got, want, "{open:?}");

        for step in [1744, 1745] {
            let mut ctx = login_ctx(OTHER_WORLD, Some(ASGARD));
            set_477(&mut ctx, Some(step), open);
            let got = m477_matched(&fire(&engine, TriggerType::PlayerLoaded, &ctx));
            assert!(got.is_empty(), "{OTHER_WORLD} S{step} {open:?}: {got:?}");
        }
    }
}

/// Revert: drop `archetype eq 5` from 23437 and a Human accepts 477.
#[tokio::test]
async fn accept_is_asgard_only_and_needs_443_done() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        let mut ctx = choice(OFFER_DIALOG, archetype);
        set_477(&mut ctx, None, [false; 4]);
        let got = m477_matched(&fire(&engine, TriggerType::DialogChoice, &ctx));
        assert!(
            got.is_empty(),
            "23437 must not match {archetype:?}: {got:?}"
        );
    }
    let mut ctx = choice(OFFER_DIALOG, Some(ASGARD));
    set_477(&mut ctx, None, [false; 4]);
    with_mission(&mut ctx, 443, "active");
    let got = m477_matched(&fire(&engine, TriggerType::DialogChoice, &ctx));
    assert!(got.is_empty(), "23437 needs 443 completed: {got:?}");
}

/// Every partition chain: for each subject and all 16 objective states, the
/// whole seed matches exactly the one chain the state selects (or none when
/// the subject is recorded), and a positive match unbinds that subject.
/// Revert: drop `O<a> active` from Carter's mark-a and the all-open state
/// matches two chains.
#[tokio::test]
async fn partition_is_disjoint_and_total() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for (si, (tag, slot, _)) in SUBJECTS.iter().enumerate() {
        for open in all_states() {
            let mut ctx = click(tag, Some(ASGARD));
            set_477(&mut ctx, Some(1744), open);
            let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
            let want: BTreeSet<i64> = partition_chain(si, open).into_iter().collect();
            assert_eq!(m477_matched(&resolved), want, "{tag} {open:?}");
            if let Some(chain) = partition_chain(si, open) {
                assert!(
                    summarized(&resolved, chain)
                        .contains(&format!("remove_dialog_set(2457, slot={slot})")),
                    "{chain} must unbind {tag}"
                );
            }
        }
    }
}

/// Revert: add `complete_objective` to 23441 and the last subject completes
/// mission 477 on its own, at S1744 (F19).
#[tokio::test]
async fn last_chain_never_completes_an_objective() {
    let pool = require_db_or_skip!();
    for (si, (tag, _, first)) in SUBJECTS.iter().enumerate() {
        let last = first + 3;
        let engine = engine_for(&pool, last as i32).await;
        let mut ctx = click(tag, Some(ASGARD));
        let mut open = [false; 4];
        open[si] = true;
        set_477(&mut ctx, Some(1744), open);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        assert_no_deferred_actions(&resolved, last);
        let actions = summarized(&resolved, last);
        assert!(
            actions.contains(&"advance_step(477, 1745)".to_string()),
            "{last}: {actions:?}"
        );
        assert!(
            !actions.iter().any(|a| a.starts_with("complete_objective")),
            "{last} must not complete an objective: {actions:?}"
        );
    }
}

#[tokio::test]
async fn subject_relog_restores_and_return_bind_are_exact() {
    let pool = require_db_or_skip!();
    for (si, (_, slot, _)) in SUBJECTS.iter().enumerate() {
        let chain = 23454 + si as i32;
        let engine = engine_for(&pool, chain).await;
        let mut ctx = login_ctx(BETA, Some(ASGARD));
        let mut open = [false; 4];
        open[si] = true;
        set_477(&mut ctx, Some(1744), open);
        let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
        assert_no_deferred_actions(&resolved, chain as i64);
        assert_eq!(
            summarized(&resolved, chain as i64),
            vec![format!("add_dialog_set(2457, slot={slot})")]
        );
    }

    let engine = engine_for(&pool, 23458).await;
    let mut ctx = login_ctx(BETA, Some(ASGARD));
    set_477(&mut ctx, Some(1745), [false; 4]);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
    assert_no_deferred_actions(&resolved, 23458);
    assert_eq!(
        summarized(&resolved, 23458),
        vec![format!(
            "add_dialog_set({COMPLETION_DSM}, slot={MIMIR_SLOT})"
        )]
    );
}

#[tokio::test]
async fn mimir_turn_in_completes_477() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23459).await;
    let mut ctx = click(MIMIR, Some(ASGARD));
    set_477(&mut ctx, Some(1745), [false; 4]);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_no_deferred_actions(&resolved, 23459);
    assert_eq!(
        summarized(&resolved, 23459),
        vec![
            format!("display_dialog({COMPLETION_DIALOG})"),
            format!("remove_dialog_set({COMPLETION_DSM}, slot={MIMIR_SLOT})"),
            format!("remove_item({RECORDING_DEVICE}, 1)"),
            "complete_mission(477)".to_string(),
        ]
    );
}

/// Whole-seed exclusivity for the Mimir, accept and completion-bind chains: each
/// event in each state matches exactly the expected 477 chain, and none for a
/// non-Asgard login at S1745.
///
/// Revert: drop `archetype eq 5` from 23458 and a non-Asgard login at S1745
/// binds the completion dialog; add a sibling on the Mimir tag, dialog 1997 or
/// the 23458 login and the exact set of one of these states changes.
#[tokio::test]
async fn return_leg_matches_exactly_one_chain_per_state() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let none = BTreeSet::new();

    let mut offer = click(MIMIR, Some(ASGARD));
    set_477(&mut offer, None, [false; 4]);
    let got = m477_matched(&fire(&engine, TriggerType::InteractTag, &offer));
    assert_eq!(got, BTreeSet::from([23436]), "Mimir at the offer");

    let mut accept = choice(OFFER_DIALOG, Some(ASGARD));
    set_477(&mut accept, None, [false; 4]);
    let got = m477_matched(&fire(&engine, TriggerType::DialogChoice, &accept));
    assert_eq!(got, BTreeSet::from([23437]), "Accept");

    let mut mid = click(MIMIR, Some(ASGARD));
    set_477(&mut mid, Some(1744), [true; 4]);
    let got = m477_matched(&fire(&engine, TriggerType::InteractTag, &mid));
    assert_eq!(got, none, "Mimir at S1744");

    let mut turn_in = click(MIMIR, Some(ASGARD));
    set_477(&mut turn_in, Some(1745), [false; 4]);
    let got = m477_matched(&fire(&engine, TriggerType::InteractTag, &turn_in));
    assert_eq!(got, BTreeSet::from([23459]), "Mimir at S1745");

    let mut finished = click(MIMIR, Some(ASGARD));
    set_477(&mut finished, Some(1745), [false; 4]);
    with_mission(&mut finished, 477, "completed");
    with_step(&mut finished, 477, 1745, "completed");
    let got = m477_matched(&fire(&engine, TriggerType::InteractTag, &finished));
    assert_eq!(got, none, "Mimir after 477");

    for tag in SUBJECT_TAGS {
        let mut ctx = click(tag, Some(ASGARD));
        set_477(&mut ctx, Some(1745), [false; 4]);
        let got = m477_matched(&fire(&engine, TriggerType::InteractTag, &ctx));
        assert_eq!(got, none, "{tag} at S1745");
    }

    let mut relog = login_ctx(BETA, Some(ASGARD));
    set_477(&mut relog, Some(1745), [false; 4]);
    let got = m477_matched(&fire(&engine, TriggerType::PlayerLoaded, &relog));
    assert_eq!(got, BTreeSet::from([23458]), "Asgard login at S1745");

    let mut done = login_ctx(BETA, Some(ASGARD));
    set_477(&mut done, Some(1745), [false; 4]);
    with_mission(&mut done, 477, "completed");
    with_step(&mut done, 477, 1745, "completed");
    let got = m477_matched(&fire(&engine, TriggerType::PlayerLoaded, &done));
    assert_eq!(got, none, "login after 477");

    for archetype in non_asgard() {
        let mut ctx = login_ctx(BETA, archetype);
        set_477(&mut ctx, Some(1745), [false; 4]);
        let got = m477_matched(&fire(&engine, TriggerType::PlayerLoaded, &ctx));
        assert_eq!(got, none, "login S1745 {archetype:?}");
    }
}

/// Revert: drop `archetype eq 5` from any subject chain and a Human (or an
/// unstamped click) records a subject.
#[tokio::test]
async fn subjects_and_return_are_asgard_only() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        for tag in SUBJECT_TAGS.iter().chain(std::iter::once(&MIMIR)) {
            for step in [1744, 1745] {
                let mut ctx = click(tag, archetype);
                set_477(&mut ctx, Some(step), [true; 4]);
                let got = m477_matched(&fire(&engine, TriggerType::InteractTag, &ctx));
                assert!(got.is_empty(), "{tag} {archetype:?} {step}: {got:?}");
            }
        }
        let mut ctx = login_ctx(BETA, archetype);
        set_477(&mut ctx, Some(1744), [true; 4]);
        let got = m477_matched(&fire(&engine, TriggerType::PlayerLoaded, &ctx));
        assert!(got.is_empty(), "login {archetype:?}: {got:?}");
    }
}
