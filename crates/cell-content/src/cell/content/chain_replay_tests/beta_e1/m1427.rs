//! Beta Site E1 1427 The Ruins (UB-16): once 1426 is completed, Carter offers 1427
//! (dialog 4662, the blurb bind 5464). The accept takes 1427 and drops the bind.
//! The player clicks the Barrak'Ka post-it (S4248, the RuinNote object, dialog
//! 4663) to S4249, which binds Carter's report 5466. Carter's report (dialog 4664)
//! advances to S4250, where the arc stops (D-UB5 A). Chains 23190-23195, seeded in
//! `db/resources/Content/Seed/beta_e1_m1427_chains.sql`. Other races and unstamped
//! players must match none of them.

use std::collections::BTreeSet;

use super::super::assert_no_deferred_actions;
use super::{
    chains_matched, fire, login_ctx, summarized, whole_seed, with_mission, with_step,
    ALL_ARCHETYPES, BETA, HUMANS,
};
use crate::test_support::require_db_or_skip;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::TriggerType;

const LOGIN_OFFER: i64 = 23190;
const CARTER_OFFER: i64 = 23191;
const ACCEPT: i64 = 23192;
const NOTE_CLICK: i64 = 23193;
const RESTORE_REPORT: i64 = 23194;
const REPORT: i64 = 23195;
/// The 1426 turn-in (S4247), which shares tag `BetaE1_Carter` with the 1427 chains.
const TURN_IN_1426: i64 = 23171;

const M381: i32 = 381;
const M1423: i32 = 1423;
const M1425: i32 = 1425;
const M1426: i32 = 1426;
const M1427: i32 = 1427;
const CARTER: &str = "BetaE1_Carter";
const FELGER: &str = "BetaE1_Felger";
const NOTE: &str = "BetaE1_RuinNote";
const OFFER_DIALOG: i64 = 4662;

/// A mission's status and, when given, its active step.
type State = (&'static str, Option<i32>);

/// The missions the Human arc reads, one state each. Mission 381 (Carter's offer
/// 23041) is completed before 1425, so the walk holds it done from index 2.
#[derive(Clone, Copy)]
struct Arc {
    m381: State,
    m1423: State,
    m1425: State,
    m1426: State,
    m1427: State,
}

const fn arc(m1423: State, m1425: State, m1426: State, m1427: State) -> Arc {
    Arc {
        m381: DONE,
        m1423,
        m1425,
        m1426,
        m1427,
    }
}
/// The same state with mission 381 not yet offered (Carter then shows its offer).
const fn before_381(arc: Arc) -> Arc {
    Arc { m381: NOT, ..arc }
}
const fn act(step: i32) -> State {
    ("active", Some(step))
}
const NOT: State = ("not_active", None);
const DONE: State = ("completed", None);

/// The Human arc, one state per entry, from 1423 S4224 to 1427 S4250. Index 8 is
/// 1426 S4247, index 9 is the 1427 offer, index 11 is S4249 and index 12 is S4250.
/// 1424 is read by no chain in the walk.
const WALK: [Arc; 13] = [
    before_381(arc(act(4224), NOT, NOT, NOT)),
    before_381(arc(DONE, NOT, NOT, NOT)),
    arc(DONE, act(4242), NOT, NOT),
    arc(DONE, act(4243), NOT, NOT),
    arc(DONE, act(4244), NOT, NOT),
    arc(DONE, DONE, NOT, NOT),
    arc(DONE, DONE, act(4245), NOT),
    arc(DONE, DONE, act(4246), NOT),
    arc(DONE, DONE, act(4247), NOT),
    arc(DONE, DONE, DONE, NOT),
    arc(DONE, DONE, DONE, act(4248)),
    arc(DONE, DONE, DONE, act(4249)),
    arc(DONE, DONE, DONE, act(4250)),
];

/// Set every mission in `arc`, the way `populate_mission_context` does.
fn with_arc(mut ctx: ExecutionContext, arc: Arc) -> ExecutionContext {
    for (id, (status, step)) in [
        (M381, arc.m381),
        (M1423, arc.m1423),
        (M1425, arc.m1425),
        (M1426, arc.m1426),
        (M1427, arc.m1427),
    ] {
        with_mission(&mut ctx, id, status);
        if let Some(step) = step {
            with_step(&mut ctx, id, step, "active");
        }
    }
    ctx
}

/// A context with `param` set to `value`, the archetype stamped when given, and
/// the missions in state `arc`.
fn event(
    param: &str,
    value: serde_json::Value,
    archetype: Option<i64>,
    arc: Arc,
) -> ExecutionContext {
    let mut ctx = ExecutionContext::new();
    ctx.set_param(param.to_string(), value);
    if let Some(a) = archetype {
        ctx.set_param("archetype".to_string(), serde_json::json!(a));
    }
    with_arc(ctx, arc)
}

/// A click on `tag` by a player of `archetype` in state `arc`.
fn tap(tag: &str, archetype: Option<i64>, arc: Arc) -> ExecutionContext {
    event("entity_tag", serde_json::json!(tag), archetype, arc)
}

/// A `player_loaded` into `world` in state `arc`.
fn login(world: &str, archetype: Option<i64>, arc: Arc) -> ExecutionContext {
    with_arc(login_ctx(world, archetype), arc)
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

/// The 1427 chains (23190-23219) that `resolved` matched.
fn matched_1427(resolved: &ResolvedActions) -> BTreeSet<i64> {
    chains_matched(resolved)
        .into_iter()
        .filter(|id| (LOGIN_OFFER..=23219).contains(id))
        .collect()
}

/// Every chain in the seed that `resolved` matched and that displays a dialog.
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

/// Pass the context to `fire` and assert that `chain` matched alone among the
/// 1427 chains, with exactly `list`.
fn assert_chain(
    engine: &ChainEngine,
    trigger: TriggerType,
    ctx: &ExecutionContext,
    chain: i64,
    list: &[&str],
) {
    let resolved = fire(engine, trigger, ctx);
    assert_eq!(matched_1427(&resolved), BTreeSet::from([chain]));
    assert_eq!(
        summarized(&resolved, chain),
        list.iter().map(|s| s.to_string()).collect::<Vec<_>>()
    );
    assert_no_deferred_actions(&resolved, chain);
}

/// Assert that no 1427 chain matches `ctx`.
fn assert_silent(engine: &ChainEngine, trigger: TriggerType, ctx: &ExecutionContext) {
    let resolved = fire(engine, trigger, ctx);
    let matched = matched_1427(&resolved);
    assert!(
        matched.is_empty(),
        "no 1427 chain may match, got {matched:?}"
    );
}

/// Revert: drop the `mission_completed` trigger row of 23190 (the bind then waits
/// for the next login) or the `add_dialog_set` row (no blurb). A Human with 1426
/// completed and 1427 not started binds Carter's blurb 5464 on login and on the
/// finish of 1426. Other races, unstamped players, another world, and the 1427
/// states that already hold the bind (S4248 on) match nothing.
#[tokio::test]
async fn offer_bind_after_1426_on_login_and_completion() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let bind = ["add_dialog_set(5464, slot=2300)"];
    let offer = WALK[9];
    for archetype in HUMANS {
        let a = Some(archetype);
        assert_chain(
            &whole,
            TriggerType::PlayerLoaded,
            &login(BETA, a, offer),
            LOGIN_OFFER,
            &bind,
        );
        // 23190 has two trigger rows, so the whole seed holds both paths.
        let finished = event("mission_id", serde_json::json!(M1426), a, offer);
        assert_chain(
            &whole,
            TriggerType::MissionCompleted,
            &finished,
            LOGIN_OFFER,
            &bind,
        );
        assert_silent(&whole, TriggerType::PlayerLoaded, &login(BETA, a, WALK[10]));
    }
    for archetype in non_human_or_unstamped() {
        assert_silent(
            &whole,
            TriggerType::PlayerLoaded,
            &login(BETA, archetype, offer),
        );
        assert_silent(
            &whole,
            TriggerType::MissionCompleted,
            &event("mission_id", serde_json::json!(M1426), archetype, offer),
        );
        // The wrong world must match nothing either.
        assert_silent(
            &whole,
            TriggerType::PlayerLoaded,
            &login("Some_Other_World", archetype, offer),
        );
    }
}

/// Revert: drop the `mission_status 1427 not_active` row of 23191 (the offer then
/// shows after the accept) or the `remove_dialog_set` row of 23192 (the blurb
/// stays bound). Carter shows 4662 once 1426 is done, and the pick accepts 1427
/// and removes the blurb 5464. Other races and unstamped players match nothing,
/// and nothing offers 1427 before 1426 or after the accept.
#[tokio::test]
async fn carter_offer_shows_and_accept_takes_1427() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let accept = ["accept_mission(1427)", "remove_dialog_set(5464, slot=2300)"];
    for archetype in HUMANS {
        let a = Some(archetype);
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &tap(CARTER, a, WALK[9]),
            CARTER_OFFER,
            &["display_dialog(4662)"],
        );
        assert_chain(
            &whole,
            TriggerType::DialogChoice,
            &event("dialog_id", serde_json::json!(OFFER_DIALOG), a, WALK[9]),
            ACCEPT,
            &accept,
        );
        // Before 1426 is done, and once 1427 is accepted, the offer is silent.
        assert_silent(&whole, TriggerType::InteractTag, &tap(CARTER, a, WALK[5]));
        assert_silent(&whole, TriggerType::InteractTag, &tap(CARTER, a, WALK[10]));
        assert_silent(
            &whole,
            TriggerType::DialogChoice,
            &event("dialog_id", serde_json::json!(OFFER_DIALOG), a, WALK[10]),
        );
    }
    for archetype in non_human_or_unstamped() {
        assert_silent(
            &whole,
            TriggerType::InteractTag,
            &tap(CARTER, archetype, WALK[9]),
        );
        assert_silent(
            &whole,
            TriggerType::DialogChoice,
            &event(
                "dialog_id",
                serde_json::json!(OFFER_DIALOG),
                archetype,
                WALK[9],
            ),
        );
    }
}

/// Revert: drop the `advance_step 1427 4249` row of 23193 (the note then stays at
/// S4248) or the `add_dialog_set` row (no report bind for Carter). The note click
/// at S4248 shows dialog 4663, advances to S4249 and binds 5466. Other races and
/// unstamped players match nothing, and the note is silent at every other step.
#[tokio::test]
async fn note_click_advances_to_report_s4249() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let list = [
        "display_dialog(4663)",
        "advance_step(1427, 4249)",
        "add_dialog_set(5466, slot=2300)",
    ];
    for archetype in HUMANS {
        let a = Some(archetype);
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &tap(NOTE, a, WALK[10]),
            NOTE_CLICK,
            &list,
        );
        for arc in [WALK[9], WALK[11], WALK[12]] {
            assert_silent(&whole, TriggerType::InteractTag, &tap(NOTE, a, arc));
        }
    }
    for archetype in non_human_or_unstamped() {
        assert_silent(
            &whole,
            TriggerType::InteractTag,
            &tap(NOTE, archetype, WALK[10]),
        );
    }
}

/// Revert: drop the `step_status 1427 4249` row of 23194 (the bind then also lands
/// at S4248) or the `add_dialog_set` row (a relog loses the report bind). A Human
/// who logs in at S4249 gets Carter's report bind 5466 back, and the other states
/// and other races match nothing.
#[tokio::test]
async fn login_restores_report_bind_at_s4249() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let bind = ["add_dialog_set(5466, slot=2300)"];
    for archetype in HUMANS {
        let a = Some(archetype);
        assert_chain(
            &whole,
            TriggerType::PlayerLoaded,
            &login(BETA, a, WALK[11]),
            RESTORE_REPORT,
            &bind,
        );
        for arc in [WALK[10], WALK[12]] {
            assert_silent(&whole, TriggerType::PlayerLoaded, &login(BETA, a, arc));
        }
    }
    for archetype in non_human_or_unstamped() {
        assert_silent(
            &whole,
            TriggerType::PlayerLoaded,
            &login(BETA, archetype, WALK[11]),
        );
    }
    assert_silent(
        &whole,
        TriggerType::PlayerLoaded,
        &login("Some_Other_World", Some(1), WALK[11]),
    );
}

/// Revert: drop the `advance_step 1427 4250` row of 23195 (the arc then stays at
/// S4249) or the `remove_dialog_set` row (the report bind stays on Carter). Carter's
/// report at S4249 shows 4664, unbinds 5466 and advances to S4250. Other races and
/// the other steps match nothing.
#[tokio::test]
async fn carter_report_advances_to_s4250() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        let a = Some(archetype);
        assert_chain(
            &whole,
            TriggerType::InteractTag,
            &tap(CARTER, a, WALK[11]),
            REPORT,
            &[
                "display_dialog(4664)",
                "remove_dialog_set(5466, slot=2300)",
                "advance_step(1427, 4250)",
            ],
        );
        for arc in [WALK[10], WALK[12]] {
            assert_silent(&whole, TriggerType::InteractTag, &tap(CARTER, a, arc));
        }
    }
    for archetype in non_human_or_unstamped() {
        assert_silent(
            &whole,
            TriggerType::InteractTag,
            &tap(CARTER, archetype, WALK[11]),
        );
    }
}

/// Revert: add `complete_mission 1427` to 23195, or a dialog 4670 or dsm 5468 row
/// to any 1427 chain. The arc ends at S4250 with no completion and no Woolsey
/// dialog or bind, since Woolsey belongs to SGC (D-UB5 A). The count must be
/// positive first, so an unloaded seed cannot pass.
#[tokio::test]
async fn arc_stops_at_s4250() {
    let pool = require_db_or_skip!();
    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resources.content_actions \
         WHERE chain_id BETWEEN 23190 AND 23219",
    )
    .fetch_one(&pool)
    .await
    .expect("count the 1427 actions");
    assert!(total > 0, "the 1427 seed must be loaded");
    let reaching_past: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM resources.content_actions \
         WHERE chain_id BETWEEN 23190 AND 23219 AND ( \
           (action_type = 'complete_mission' AND target_id = 1427) \
           OR target_id IN (4670, 5468))",
    )
    .fetch_one(&pool)
    .await
    .expect("count the 1427 actions past S4250");
    assert_eq!(
        reaching_past, 0,
        "no 1427 action may complete or reach Woolsey"
    );
}

/// Revert: widen 23191 to drop `1427 not_active` (the offer then shows at S4248 to
/// S4250 as well), or widen 23195 to match S4248 (Carter then displays two dialogs
/// on one click). Over the 1426 turn-in and the 1427 chains, Carter displays at
/// most one dialog per state, and none at S4248, S4250.
#[tokio::test]
async fn carter_one_dialog_chain_per_state() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    let expected: [(usize, BTreeSet<i64>); 5] = [
        (8, BTreeSet::from([TURN_IN_1426])),
        (9, BTreeSet::from([CARTER_OFFER])),
        (10, BTreeSet::new()),
        (11, BTreeSet::from([REPORT])),
        (12, BTreeSet::new()),
    ];
    for archetype in HUMANS {
        for (index, want) in &expected {
            let resolved = fire(
                &whole,
                TriggerType::InteractTag,
                &tap(CARTER, Some(archetype), WALK[*index]),
            );
            let got: BTreeSet<i64> = dialog_chains(&resolved)
                .into_iter()
                .filter(|id| (TURN_IN_1426..=REPORT).contains(id))
                .collect();
            assert_eq!(
                &got, want,
                "Carter at walk state {index}, archetype {archetype}"
            );
        }
    }
}

/// Revert: as for `carter_one_dialog_chain_per_state`, a 1427 chain that also
/// displays on Carter or Felger, or a dropped `not_active` gate. Walking the whole
/// Human arc (1423 S4224 to 1427 S4250) one state at a time, Carter and Felger each
/// display at most one dialog on the whole seed in every state.
#[tokio::test]
async fn human_cast_full_arc_exclusivity() {
    let pool = require_db_or_skip!();
    let whole = whole_seed(&pool).await;
    for archetype in HUMANS {
        for (index, arc) in WALK.iter().enumerate() {
            for tag in [CARTER, FELGER] {
                let resolved = fire(
                    &whole,
                    TriggerType::InteractTag,
                    &tap(tag, Some(archetype), *arc),
                );
                let shown = dialog_chains(&resolved);
                assert!(
                    shown.len() <= 1,
                    "{tag} at walk state {index}, archetype {archetype}: {shown:?}"
                );
            }
        }
    }
}
