//! Beta Site E1 1425 SGU Database (UB-14): after 478, Carter offers 1425
//! (dialog 4654, the blurb bind 5457), the player accepts it, clicks the hub
//! computer (S4242, dialog 4655, button 456), then the science computer (S4243,
//! dialog 5497, button 457), and the mission stops at S4244 (Jackson, Omega's
//! side, D-UB5 A). Chains 23130-23136, seeded in
//! `db/resources/Content/Seed/beta_e1_m1425_chains.sql`. Other races and
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

const OFFER_BIND: i64 = 23130;
const OFFER_SHOW: i64 = 23131;
const ACCEPT: i64 = 23132;
const HUB_CLICK: i64 = 23133;
const HUB_BUTTON: i64 = 23134;
const SCIENCE_CLICK: i64 = 23135;
const SCIENCE_BUTTON: i64 = 23136;

const MISSION: i32 = 1425;
const PREREQ: i32 = 478;
const HUB_STEP: i32 = 4242;
const SCIENCE_STEP: i32 = 4243;
const DELIVER_STEP: i32 = 4244;
const CARTER: &str = "BetaE1_Carter";
const HUB: &str = "BetaE1_HubComputer";
const SCIENCE: &str = "BetaE1_ScienceComputer";

/// The 1425 chains (23130-23136) that `resolved` matched, for the exclusivity checks.
fn m1425_matched(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| (OFFER_BIND..=SCIENCE_BUTTON).contains(id))
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

/// 478 completed, 1425 at `status`, and 1425's step `step` (when given) active.
fn after_478(mut ctx: ExecutionContext, status: &str, step: Option<i32>) -> ExecutionContext {
    with_mission(&mut ctx, PREREQ, "completed");
    with_mission(&mut ctx, MISSION, status);
    if let Some(s) = step {
        with_step(&mut ctx, MISSION, s, "active");
    }
    ctx
}

/// Pass the context to `fire` and assert that `chain` matched alone with `list`.
fn assert_chain(
    engine: &cimmeria_content_engine::chain::ChainEngine,
    trigger: TriggerType,
    ctx: &ExecutionContext,
    chain: i64,
    list: &[&str],
) {
    let resolved = fire(engine, trigger, ctx);
    assert_eq!(m1425_matched(&resolved), BTreeSet::from([chain]));
    assert_eq!(
        summarized(&resolved, chain),
        list.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    );
    assert_no_deferred_actions(&resolved, chain);
}

/// Revert: drop a `mission_completed` trigger row of 23130 (the bind then waits
/// for the next login) or the `add_dialog_set` row (no blurb). A Human who
/// finishes 478 binds Carter's blurb (5457), on login and on the finish, and
/// the whole seed matches only 23130.
#[tokio::test]
async fn offer_bind_after_478_binds_carter_blurb_per_player() {
    let pool = require_db_or_skip!();
    // 23130 has two trigger rows, so only the whole seed holds both paths.
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let login = after_478(login_ctx(BETA, Some(archetype)), "not_active", None);
        assert_chain(
            &whole,
            TriggerType::PlayerLoaded,
            &login,
            OFFER_BIND,
            &["add_dialog_set(5457, slot=2300)"],
        );
        let finished = after_478(
            event("mission_id", serde_json::json!(PREREQ), Some(archetype)),
            "not_active",
            None,
        );
        assert_chain(
            &whole,
            TriggerType::MissionCompleted,
            &finished,
            OFFER_BIND,
            &["add_dialog_set(5457, slot=2300)"],
        );
    }
    for archetype in non_human_or_unstamped() {
        let login = after_478(login_ctx(BETA, archetype), "not_active", None);
        let resolved = fire(&whole, TriggerType::PlayerLoaded, &login);
        assert!(
            m1425_matched(&resolved).is_empty(),
            "archetype {archetype:?} must match no 1425 chain on login",
        );
    }
}

/// Revert: drop 23131's `display_dialog` row (no offer), 23132's `accept_mission`
/// row (the mission never starts) or its `remove_dialog_set` (the blurb stays
/// bound). Other races and unstamped players match nothing.
#[tokio::test]
async fn carter_offer_shows_and_accept_takes_1425() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let click = after_478(
            event("entity_tag", serde_json::json!(CARTER), Some(archetype)),
            "not_active",
            None,
        );
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &click,
            OFFER_SHOW,
            &["display_dialog(4654)"],
        );
        let accept = after_478(
            event("dialog_id", serde_json::json!(4654), Some(archetype)),
            "not_active",
            None,
        );
        assert_chain(
            &whole,
            TriggerType::DialogChoice,
            &accept,
            ACCEPT,
            &["accept_mission(1425)", "remove_dialog_set(5457, slot=2300)"],
        );
    }
    for archetype in non_human_or_unstamped() {
        let click = after_478(
            event("entity_tag", serde_json::json!(CARTER), archetype),
            "not_active",
            None,
        );
        let accept = after_478(
            event("dialog_id", serde_json::json!(4654), archetype),
            "not_active",
            None,
        );
        for (trigger, ctx) in [
            (TriggerType::InteractTag, click),
            (TriggerType::DialogChoice, accept),
        ] {
            let resolved = fire(&whole, trigger, &ctx);
            assert!(
                m1425_matched(&resolved).is_empty(),
                "archetype {archetype:?} must match no 1425 chain",
            );
        }
    }
}

/// Revert: drop 23133's or 23135's `interact_tag` row (the object click shows
/// nothing), or the step gates: 23133 must not match at S4243 and 23135 must
/// not match at S4242, or the two computers would show out of order.
#[tokio::test]
async fn hub_and_science_computers_are_step_gated() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let hub_click = after_478(
            event("entity_tag", serde_json::json!(HUB), Some(archetype)),
            "active",
            Some(HUB_STEP),
        );
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &hub_click,
            HUB_CLICK,
            &["display_dialog(4655)"],
        );
        let hub_at_science = after_478(
            event("entity_tag", serde_json::json!(HUB), Some(archetype)),
            "active",
            Some(SCIENCE_STEP),
        );
        let resolved = fire(&whole, TriggerType::InteractTag, &hub_at_science);
        assert!(m1425_matched(&resolved).is_empty());
        let science_at_hub = after_478(
            event("entity_tag", serde_json::json!(SCIENCE), Some(archetype)),
            "active",
            Some(HUB_STEP),
        );
        let resolved = fire(&whole, TriggerType::InteractTag, &science_at_hub);
        assert!(m1425_matched(&resolved).is_empty());
        let science_at_science = after_478(
            event("entity_tag", serde_json::json!(SCIENCE), Some(archetype)),
            "active",
            Some(SCIENCE_STEP),
        );
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &science_at_science,
            SCIENCE_CLICK,
            &["display_dialog(5497)"],
        );
    }
    for archetype in non_human_or_unstamped() {
        for tag in [HUB, SCIENCE] {
            let ctx = after_478(
                event("entity_tag", serde_json::json!(tag), archetype),
                "active",
                Some(HUB_STEP),
            );
            let resolved = fire(&whole, TriggerType::InteractTag, &ctx);
            assert!(
                m1425_matched(&resolved).is_empty(),
                "archetype {archetype:?} must match no 1425 chain on {tag}",
            );
        }
    }
}

/// Revert: drop 23134's or 23136's `dialog_choice` row (the button press does
/// nothing), or the `advance_step` row (the step never moves), or the step
/// gate (the button at the wrong step advances it again). Button 456 is
/// `dialog_choice 4655` and button 457 is `dialog_choice 5497` (Q6).
#[tokio::test]
async fn hub_and_science_buttons_advance_one_step() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let hub = after_478(
            event("dialog_id", serde_json::json!(4655), Some(archetype)),
            "active",
            Some(HUB_STEP),
        );
        assert_chain(
            &whole,
            TriggerType::DialogChoice,
            &hub,
            HUB_BUTTON,
            &["advance_step(1425, 4243)"],
        );
        let science = after_478(
            event("dialog_id", serde_json::json!(5497), Some(archetype)),
            "active",
            Some(SCIENCE_STEP),
        );
        assert_chain(
            &whole,
            TriggerType::DialogChoice,
            &science,
            SCIENCE_BUTTON,
            &["advance_step(1425, 4244)"],
        );
        let hub_at_science = after_478(
            event("dialog_id", serde_json::json!(4655), Some(archetype)),
            "active",
            Some(SCIENCE_STEP),
        );
        let resolved = fire(&whole, TriggerType::DialogChoice, &hub_at_science);
        assert!(m1425_matched(&resolved).is_empty());
    }
    for archetype in non_human_or_unstamped() {
        for dialog in [4655, 5497] {
            let ctx = after_478(
                event("dialog_id", serde_json::json!(dialog), archetype),
                "active",
                Some(SCIENCE_STEP),
            );
            let resolved = fire(&whole, TriggerType::DialogChoice, &ctx);
            assert!(
                m1425_matched(&resolved).is_empty(),
                "archetype {archetype:?} must match no 1425 chain on {dialog}",
            );
        }
    }
}

/// Revert: add `complete_mission 1425` to 23136 (or any `advance_step 1425`
/// past S4244, or `complete_objective` on 1425: F19). Under D-UB5 A the mission
/// stays open at S4244 in the journal, so no action of 23130-23159 may finish
/// it or move it past S4244.
#[tokio::test]
async fn mission_stays_open_at_s4244() {
    let pool = require_db_or_skip!();
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT chain_id::text, action_type::text, COALESCE(target_id::text, ''), \
         COALESCE(target_key::text, '') FROM resources.content_actions \
         WHERE chain_id BETWEEN 23130 AND 23159 ORDER BY chain_id, sort_order",
    )
    .fetch_all(&pool)
    .await
    .expect("read 1425 chain actions");
    assert!(!rows.is_empty(), "the 1425 chains must be seeded");
    for (chain, action, target, key) in rows {
        if target != "1425" {
            continue;
        }
        match action.as_str() {
            "complete_mission" | "complete_objective" => {
                panic!("chain {chain} must not {action} 1425");
            }
            "advance_step" => {
                let step: i32 = key.parse().expect("advance_step key is a step id");
                assert!(
                    step <= DELIVER_STEP,
                    "chain {chain} advances 1425 past S4244 to {step}",
                );
            }
            _ => {}
        }
    }
}

/// Revert: give Carter's `interact_tag` chains an extra dialog-showing sibling
/// for one state, or drop 23130's `mission_status` gate. For each Carter click
/// state, the whole seed matches at most one chain that displays a dialog, and
/// the 1425 chains match exactly the expected one (or none).
#[tokio::test]
async fn carter_one_dialog_chain_per_state() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    // (state, the 1425 chains that must match on Carter's click in that state).
    let states: [(&str, BTreeSet<i64>); 3] = [
        // 478 still on its admin workstation step: no 1425 offer yet.
        ("478 S1749", BTreeSet::new()),
        // 478 done, 1425 open: the offer.
        ("1425 open", BTreeSet::from([OFFER_SHOW])),
        // 1425 at S4244: Carter is silent for 1425.
        ("1425 S4244", BTreeSet::new()),
    ];
    for archetype in HUMANS {
        for (state, expected) in &states {
            let resolved = fire(
                &whole,
                TriggerType::InteractTag,
                &carter_click(archetype, state),
            );
            let showing = chains_matched(&resolved)
                .into_iter()
                .filter(|id| {
                    summarized(&resolved, *id)
                        .iter()
                        .any(|a| a.starts_with("display_dialog"))
                })
                .count();
            assert!(showing <= 1, "{state}: {showing} dialog chains matched");
            assert_eq!(
                &m1425_matched(&resolved),
                expected,
                "{state}: 1425 chains for archetype {archetype}",
            );
        }
    }
}

/// Carter's click for a Human in the named state (see the test above).
fn carter_click(archetype: i64, state: &str) -> ExecutionContext {
    let mut ctx = event("entity_tag", serde_json::json!(CARTER), Some(archetype));
    match state {
        "478 S1749" => {
            with_mission(&mut ctx, PREREQ, "active");
            with_step(&mut ctx, PREREQ, 1749, "active");
        }
        "1425 open" => with_mission(&mut ctx, PREREQ, "completed"),
        _ => {
            with_mission(&mut ctx, PREREQ, "completed");
            with_mission(&mut ctx, MISSION, "active");
            with_step(&mut ctx, MISSION, DELIVER_STEP, "active");
        }
    }
    ctx
}
