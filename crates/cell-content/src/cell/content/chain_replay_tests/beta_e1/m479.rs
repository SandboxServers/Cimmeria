//! Beta Site E1 Process Support (UB-36, mission 479): the Mimir blurb bind
//! (23460), the offer (23461), the Accept (23462), the Analytical Machine and
//! Consciousness Simulator steps (23463-23465), the return bind (23466) and the
//! Mimir turn-in (23467). Seeded in `db/resources/Content/Seed/beta_e1_m479_chains.sql`.
//!
//! Omega UO-16 is HELD (HELD-B16), so no live Asgard reaches these chains
//! through Beta yet. These tests replay archetype 5 (D-UB13).

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, engine_for, fire, login_ctx, summarized, whole_seed, with_counter,
    with_mission, with_step, ALL_ARCHETYPES, ASGARD, BETA,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::ResolvedActions;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const M479_CHAINS: std::ops::RangeInclusive<i64> = 23460..=23467;
const MIMIR: &str = "BetaE1_Mimir";
const MACHINE: &str = "BetaE1_AnalyticalMachine";
const SIMULATOR: &str = "BetaE1_ConsciousnessSimulator";
const OFFER_DIALOG: i64 = 3228;
const DONE_DIALOG: i64 = 3227;
const BLURB_DSM: i64 = 2462;
const DONE_DSM: i64 = 3527;
const MIMIR_SLOT: i64 = 2305;
const SCANNER_COUNTER: &str = "beta_420_scanners";
/// The arc of Mimir's turn-ins, in order. Each mission is completed before the next is offered.
const ARC: [i32; 7] = [481, 420, 421, 1147, 443, 477, 479];
/// Index of 479 in `ARC`.
const M479: usize = 6;

/// Walk of the Mimir clicks from 481 S1762 to 479 done:
/// (`ARC` index, active step or `None` for the offer, scanner counter, the one
/// chain that is due). Each click matches exactly that chain, or none at a
/// mid-mission step.
const MIMIR_STATES: &[(usize, Option<i32>, i64, Option<i64>)] = &[
    (0, Some(1762), 0, Some(23317)),
    (1, None, 0, Some(23341)),
    (1, Some(1455), 0, Some(23350)),
    (1, Some(1455), 5, Some(23349)),
    (2, None, 0, Some(23361)),
    (2, Some(1464), 0, Some(23368)),
    (3, None, 0, Some(23391)),
    (3, Some(3459), 0, Some(23395)),
    (4, None, 0, Some(23406)),
    (4, Some(3455), 0, Some(23417)),
    (5, None, 0, Some(23436)),
    (5, Some(1744), 0, None),
    (5, Some(1745), 0, Some(23459)),
    (M479, None, 0, Some(23461)),
    (M479, Some(1750), 0, None),
    (M479, Some(1751), 0, None),
    (M479, Some(1752), 0, None),
    (M479, Some(3644), 0, Some(23467)),
    (7, None, 0, None),
];

/// Clicks on the Analytical Machine: (`ARC` index, active step, the one chain that is due).
const MACHINE_STATES: &[(usize, Option<i32>, Option<i64>)] = &[
    (3, Some(3458), Some(23393)),
    (3, Some(3459), None),
    (M479, Some(1750), Some(23463)),
    (M479, Some(1751), None),
    (M479, Some(1752), Some(23465)),
    (M479, Some(3644), None),
];

/// The 479 chains that matched in `resolved`, ignoring every other packet's chains.
fn m479_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| M479_CHAINS.contains(id))
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

/// Sets the arc: the missions before `ARC[cur]` are completed, `ARC[cur]` is
/// active at `step` (`None` = not yet accepted, its offer state) and the later
/// missions are not_active. `counter` is the 420 scanner count.
fn set_arc(ctx: &mut ExecutionContext, cur: usize, step: Option<i32>, counter: i64) {
    for (i, mission) in ARC.iter().enumerate() {
        let status = if i < cur {
            "completed"
        } else if i == cur && step.is_some() {
            "active"
        } else {
            "not_active"
        };
        with_mission(ctx, *mission, status);
    }
    if let (Some(s), Some(mission)) = (step, ARC.get(cur)) {
        with_step(ctx, *mission, s, "active");
    }
    with_counter(ctx, SCANNER_COUNTER, counter);
}

/// Fires the single chain `chain` on a click of `tag` at (`cur`, `step`) as
/// Asgard, and returns its summary.
async fn single_click(
    pool: &sqlx::PgPool,
    chain: i32,
    tag: &str,
    cur: usize,
    step: Option<i32>,
) -> Vec<String> {
    let engine = engine_for(pool, chain).await;
    let mut ctx = click(tag, Some(ASGARD));
    set_arc(&mut ctx, cur, step, 0);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_no_deferred_actions(&resolved, i64::from(chain));
    summarized(&resolved, i64::from(chain))
}

#[tokio::test]
async fn offer_bind_and_offer_show_are_exact() {
    let pool = require_db_or_skip!();
    // Whole seed: the single-chain engine does not carry the mission_completed trigger.
    let bind = whole_seed(&pool).await;
    let mut ctx = completed(477, Some(ASGARD));
    set_arc(&mut ctx, M479, None, 0);
    let resolved = fire(&bind, TriggerType::MissionCompleted, &ctx);
    assert_no_deferred_actions(&resolved, 23460);
    assert_eq!(
        summarized(&resolved, 23460),
        vec![format!("add_dialog_set({BLURB_DSM}, slot={MIMIR_SLOT})")]
    );

    let actions = single_click(&pool, 23461, MIMIR, M479, None).await;
    assert_eq!(actions, vec![format!("display_dialog({OFFER_DIALOG})")]);
}

#[tokio::test]
async fn accept_starts_479_and_drops_the_blurb() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23462).await;
    let mut ctx = choice(OFFER_DIALOG, Some(ASGARD));
    set_arc(&mut ctx, M479, None, 0);
    let resolved = fire(&engine, TriggerType::DialogChoice, &ctx);
    assert_no_deferred_actions(&resolved, 23462);
    assert_eq!(
        summarized(&resolved, 23462),
        vec![
            "accept_mission(479)".to_string(),
            format!("remove_dialog_set({BLURB_DSM}, slot={MIMIR_SLOT})"),
        ]
    );
}

/// Revert: drop `archetype eq 5` from 23461 and a Human is shown the offer.
/// Once 479 is active (the offer state is gone), the bind, the offer and the
/// Accept all stay silent.
#[tokio::test]
async fn offer_chains_need_asgard_and_the_offer_state() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for archetype in non_asgard() {
        let mut ctx = completed(477, archetype);
        set_arc(&mut ctx, M479, None, 0);
        let got = m479_matched(&fire(&engine, TriggerType::MissionCompleted, &ctx));
        assert!(
            got.is_empty(),
            "23460 must not match {archetype:?}: {got:?}"
        );

        let mut ctx = click(MIMIR, archetype);
        set_arc(&mut ctx, M479, None, 0);
        let got = m479_matched(&fire(&engine, TriggerType::InteractTag, &ctx));
        assert!(
            got.is_empty(),
            "23461 must not match {archetype:?}: {got:?}"
        );

        let mut ctx = choice(OFFER_DIALOG, archetype);
        set_arc(&mut ctx, M479, None, 0);
        let got = m479_matched(&fire(&engine, TriggerType::DialogChoice, &ctx));
        assert!(
            got.is_empty(),
            "23462 must not match {archetype:?}: {got:?}"
        );
    }
    for step in [1750, 1751, 1752, 3644] {
        let mut ctx = completed(477, Some(ASGARD));
        set_arc(&mut ctx, M479, Some(step), 0);
        let got = m479_matched(&fire(&engine, TriggerType::MissionCompleted, &ctx));
        assert!(got.is_empty(), "23460 at S{step}: {got:?}");
        let mut ctx = click(MIMIR, Some(ASGARD));
        set_arc(&mut ctx, M479, Some(step), 0);
        let got = m479_matched(&fire(&engine, TriggerType::InteractTag, &ctx));
        assert!(!got.contains(&23461), "23461 needs 479 not_active: {got:?}");
        let mut ctx = choice(OFFER_DIALOG, Some(ASGARD));
        set_arc(&mut ctx, M479, Some(step), 0);
        let got = m479_matched(&fire(&engine, TriggerType::DialogChoice, &ctx));
        assert!(got.is_empty(), "23462 at S{step}: {got:?}");
    }
}

/// Revert: give 23463 the step gate of 23464 (S1751) and the S1750 click matches nothing;
/// add `complete_objective` to any of the four and the step completes early (F19).
#[tokio::test]
async fn machine_and_simulator_steps_are_exact() {
    let pool = require_db_or_skip!();
    assert_eq!(
        single_click(&pool, 23463, MACHINE, M479, Some(1750)).await,
        vec!["advance_step(479, 1751)".to_string()]
    );
    assert_eq!(
        single_click(&pool, 23464, SIMULATOR, M479, Some(1751)).await,
        vec!["advance_step(479, 1752)".to_string()]
    );
    let actions = single_click(&pool, 23465, MACHINE, M479, Some(1752)).await;
    assert_eq!(
        actions,
        vec![
            "advance_step(479, 3644)".to_string(),
            format!("add_dialog_set({DONE_DSM}, slot={MIMIR_SLOT})"),
        ]
    );
    assert!(
        !actions.iter().any(|a| a.starts_with("complete_objective")),
        "23465 must not complete an objective: {actions:?}"
    );
}

/// Revert: give the Machine chains the same step (or drop 23465's step gate)
/// and the state matches two chains, or the S1750 state matches 23465.
#[tokio::test]
async fn analytical_machine_one_chain_per_step() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for (cur, step, want) in MACHINE_STATES {
        let mut ctx = click(MACHINE, Some(ASGARD));
        set_arc(&mut ctx, *cur, *step, 0);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        let want: BTreeSet<i64> = want.iter().copied().collect();
        assert_eq!(chains_matched(&resolved), want, "{MACHINE} {cur} {step:?}");
    }
}

#[tokio::test]
async fn return_relog_restores_the_done_bind() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23466).await;
    let mut ctx = login_ctx(BETA, Some(ASGARD));
    set_arc(&mut ctx, M479, Some(3644), 0);
    let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
    assert_no_deferred_actions(&resolved, 23466);
    assert_eq!(
        summarized(&resolved, 23466),
        vec![format!("add_dialog_set({DONE_DSM}, slot={MIMIR_SLOT})")]
    );
}

#[tokio::test]
async fn mimir_turn_in_completes_479() {
    let pool = require_db_or_skip!();
    let engine = engine_for(&pool, 23467).await;
    let mut ctx = click(MIMIR, Some(ASGARD));
    set_arc(&mut ctx, M479, Some(3644), 0);
    let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
    assert_no_deferred_actions(&resolved, 23467);
    assert_eq!(
        summarized(&resolved, 23467),
        vec![
            format!("display_dialog({DONE_DIALOG})"),
            format!("remove_dialog_set({DONE_DSM}, slot={MIMIR_SLOT})"),
            "complete_mission(479)".to_string(),
        ]
    );
}

/// Walks the Mimir clicks from 481 S1762 to 479 done. Each state matches
/// exactly the one Mimir chain that is due (so at most one shows a dialog), or
/// none at a mid-mission step.
/// Revert: move 23467's step gate to S1745 and the S3644 state matches nothing.
#[tokio::test]
async fn mimir_full_asgard_arc_exclusivity() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    for (cur, step, counter, want) in MIMIR_STATES {
        let mut ctx = click(MIMIR, Some(ASGARD));
        set_arc(&mut ctx, *cur, *step, *counter);
        let resolved = fire(&engine, TriggerType::InteractTag, &ctx);
        let want: BTreeSet<i64> = want.iter().copied().collect();
        assert_eq!(
            chains_matched(&resolved),
            want,
            "Mimir at ARC[{cur}] {step:?} counter {counter}"
        );
    }
}

/// Revert: drop `archetype eq 5` from any 479 chain and a Human (or an
/// unstamped context) gets the offer, the Accept, a machine or simulator step,
/// the return bind, or the turn-in.
#[tokio::test]
async fn chains_are_asgard_only() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let arc_clicks = [
        (MIMIR, Some(3644)),
        (MACHINE, Some(1750)),
        (MACHINE, Some(1752)),
        (SIMULATOR, Some(1751)),
    ];
    for archetype in non_asgard() {
        for (tag, step) in arc_clicks {
            let mut ctx = click(tag, archetype);
            set_arc(&mut ctx, M479, step, 0);
            let got = m479_matched(&fire(&engine, TriggerType::InteractTag, &ctx));
            assert!(got.is_empty(), "{tag} {archetype:?} {step:?}: {got:?}");
        }
        let mut ctx = login_ctx(BETA, archetype);
        set_arc(&mut ctx, M479, Some(3644), 0);
        let got = m479_matched(&fire(&engine, TriggerType::PlayerLoaded, &ctx));
        assert!(got.is_empty(), "login {archetype:?}: {got:?}");
    }
}
