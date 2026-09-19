//! The `mission_abandoned` repaint chains (packet H54): 6308 (1324),
//! 6342 (1326) and 6120 (742).
//!
//! Abandoning a mission returns it to not-active with the player already
//! past every edge that set the scene up. Before H54 nothing fired into the
//! content engine on an abandon, so the offer gate reopened with no edge
//! left to repaint it and whatever dialog-set binding the mission had
//! installed was stranded on its NPC. These three chains consume the new
//! trigger.
//!
//! One file rather than a section in each mission's own guards, because the
//! risk is the *family*: three chains in two seed files sharing one trigger,
//! one ordering rule (unbind before rebind) and one world gate. A regression
//! in any of them is the same regression.
//!
//! Resolve-only (TESTING.md type 6). Every verb these chains use has a
//! shipped executor arm; what H54 adds is the trigger and the dispatcher,
//! and the dispatcher has its own executor-level guards in
//! `content/event_dispatch/mission_abandoned_tests.rs`.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::super::engine_loader::build_engine;
use super::mission_1324::{ctx_for, with_mission, CMD_CENTER, HARSET, JAFFA, NOT_JAFFA};
use crate::test_support::require_db_or_skip;

/// `archetype` id for Goa'uld — chain 6118/6120's gate.
const GOAULD: i64 = 6;

/// Chain-id window covering both seed files these chains live in
/// (`harset_goauld_chains.sql` 6101-6300, `harset_jaffa_chains.sql`
/// 6301-6500). Wide on purpose: an abandon chain accidentally authored in
/// the wrong lane's range must still show up in the diff.
const HARSET_CHAIN_RANGE: std::ops::RangeInclusive<i64> = 6101..=6500;

/// Replay the `mission_abandoned` dispatch.
///
/// `fire_mission_abandoned` (`content/event_dispatch/mission.rs`) runs
/// *after* `abandon_mission` has removed the instance and then populates
/// world, archetype and the whole mission context — so every caller here
/// passes a context in which the abandoned mission already reads
/// `not_active`, exactly as the runtime builds it. Setting `mission_id` is
/// load-bearing: the trigger is keyed on it, and a context without it makes
/// every negative below pass vacuously.
fn fire_mission_abandoned(
    engine: &ChainEngine,
    ctx: &ExecutionContext,
    mission_id: i32,
) -> ResolvedActions {
    let mut out = ExecutionContext::new();
    out.world_id = ctx.world_id;
    out.params = ctx.params.clone();
    out.set_param("mission_id".to_string(), serde_json::json!(mission_id));

    let event = TriggerEvent {
        trigger_type: TriggerType::MissionAbandoned,
        source_entity: None,
        target_entity: None,
        params: out.params.clone(),
    };
    engine.resolve_event(&event, &out)
}

/// `"<chain_id>:<verb>(<args>)"` labels for the Harset range. Anything not
/// named maps to `OTHER`, so an accidental extra action is a diff rather
/// than a silent pass.
fn labels(resolved: &ResolvedActions) -> Vec<String> {
    resolved
        .actions
        .iter()
        .filter(|(id, _)| HARSET_CHAIN_RANGE.contains(id))
        .map(|(id, action)| {
            let body = match action {
                Action::AddDialogSet {
                    dialog_set_id,
                    slot,
                    ..
                } => format!("add_dialog_set({dialog_set_id}@{slot})"),
                Action::RemoveDialogSet {
                    dialog_set_id,
                    slot,
                } => format!("remove_dialog_set({dialog_set_id}@{slot})"),
                other => format!("OTHER({other:?})"),
            };
            format!("{id}:{body}")
        })
        .collect()
}

/// A Loyalist Jaffa standing in the Command Center who has just abandoned
/// 1324. Nothing else about their state is set: after the removal the step
/// is gone, which is exactly why chain 6308 clears both possible binds
/// rather than choosing between them.
fn jaffa_who_abandoned(mission_id: i32) -> ExecutionContext {
    with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), mission_id, "not_active")
}

// ---------------------------------------------------------------
// Chain 6308 — mission 1324
// ---------------------------------------------------------------

/// Positive: abandoning 1324 in the Command Center clears Ba'al's council
/// bind and Moh'katan's turn-in bind, then repaints Moh'katan's offer.
///
/// The ORDER is the guard, not decoration. `interactions/dispatch/interact.rs`
/// takes the first bound entry on a template that carries a dialog, so dsm
/// 120001 has to leave slot 54 before 5149 arrives; swap the two rows in the
/// seed and Moh'katan keeps replaying the step-3954 turn-in dialog for a
/// player who no longer holds the mission.
#[tokio::test]
async fn abandoning_1324_clears_the_live_binds_and_repaints_the_offer() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_mission_abandoned(&engine, &jaffa_who_abandoned(1324), 1324);

    assert_eq!(
        labels(&resolved),
        vec![
            "6308:remove_dialog_set(5151@42)".to_string(),
            "6308:remove_dialog_set(120001@54)".to_string(),
            "6308:add_dialog_set(5149@54)".to_string(),
        ],
        "resolve: {:?}",
        resolved.actions,
    );
}

/// Negative (wrong world): the binds are per-player entries keyed on a
/// template in the player's current space, so a repaint fired from Harset
/// would write against templates that are not there. Abandoning outside the
/// Command Center needs nothing — `available_interactions` is rebuilt empty
/// on world entry and chain 6301 repaints on the way back in.
#[tokio::test]
async fn abandoning_1324_outside_the_command_center_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(JAFFA, Some(HARSET)), 1324, "not_active");
    assert!(
        labels(&fire_mission_abandoned(&engine, &ctx, 1324)).is_empty(),
        "chain 6308 must carry `world eq 68`"
    );
}

/// Negative (wrong archetype): only Loyalist Jaffa are offered 1324, so the
/// repaint must carry chain 6301's archetype gate too. Without it a Human
/// who somehow abandoned 1324 would get an offer icon they can never use.
#[tokio::test]
async fn abandoning_1324_as_a_non_jaffa_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(NOT_JAFFA, Some(CMD_CENTER)), 1324, "not_active");
    assert!(
        labels(&fire_mission_abandoned(&engine, &ctx, 1324)).is_empty(),
        "chain 6308 must carry `archetype eq 8`"
    );
}

/// Negative (key discrimination): the trigger is keyed on the mission id, so
/// abandoning some *other* mission must not repaint 1324's offer. This is
/// the assertion that would fail if a seed row lost its `event_key`, and the
/// one that proves the positive above is not matching on the trigger type
/// alone.
#[tokio::test]
async fn abandoning_a_different_mission_does_not_repaint_1324() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    // Everything 6308 wants is true except the event key.
    let resolved = fire_mission_abandoned(&engine, &jaffa_who_abandoned(1324), 9999);
    assert!(
        !labels(&resolved).iter().any(|l| l.starts_with("6308:")),
        "chain 6308 fired for mission 9999: {:?}",
        resolved.actions,
    );
}

// ---------------------------------------------------------------
// Chain 6342 — mission 1326
// ---------------------------------------------------------------

/// A Jaffa eligible for 1326 who has just abandoned it: 1324 and 1325 are
/// the prerequisites chain 6331 gates on, and 6342 carries them verbatim.
fn jaffa_who_abandoned_1326() -> ExecutionContext {
    let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1326, "not_active");
    let ctx = with_mission(ctx, 1325, "completed");
    with_mission(ctx, 1324, "completed")
}

/// Positive: abandoning 1326 clears Moh'katan's turn-in bind and repaints
/// his offer. Same unbind-before-rebind order as 6308, on one slot.
#[tokio::test]
async fn abandoning_1326_clears_the_turn_in_bind_and_repaints_the_offer() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_mission_abandoned(&engine, &jaffa_who_abandoned_1326(), 1326);

    assert_eq!(
        labels(&resolved),
        vec![
            "6342:remove_dialog_set(5161@54)".to_string(),
            "6342:add_dialog_set(5159@54)".to_string(),
        ],
        "resolve: {:?}",
        resolved.actions,
    );
}

/// Negative (prerequisite parity): 1326 is only offered to a Jaffa who has
/// finished 1324 and 1325. Dropping either gate from the abandon chain would
/// repaint an offer the player cannot accept — chain 6333's own gate would
/// then refuse the click and the icon would be permanently dead.
#[tokio::test]
async fn abandoning_1326_without_the_prerequisites_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    // 1325 not finished: everything else about the context is eligible.
    let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 1326, "not_active");
    let ctx = with_mission(ctx, 1325, "active");
    let ctx = with_mission(ctx, 1324, "completed");

    assert!(
        labels(&fire_mission_abandoned(&engine, &ctx, 1326)).is_empty(),
        "chain 6342 must carry chain 6331's 1324/1325 prerequisites"
    );
}

// ---------------------------------------------------------------
// Chain 6120 — mission 742
// ---------------------------------------------------------------

/// A Goa'uld who has finished 1200 and just abandoned 742, in the Command
/// Center — chain 6118's condition set, which 6120 carries verbatim.
fn goauld_who_abandoned_742() -> ExecutionContext {
    let ctx = with_mission(ctx_for(GOAULD, Some(CMD_CENTER)), 742, "not_active");
    with_mission(ctx, 1200, "completed")
}

/// Positive: abandoning 742 clears Anat's in-progress topic and repaints her
/// offer topic.
#[tokio::test]
async fn abandoning_742_clears_the_in_progress_topic_and_repaints_the_offer() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let resolved = fire_mission_abandoned(&engine, &goauld_who_abandoned_742(), 742);

    assert_eq!(
        labels(&resolved),
        vec![
            "6120:remove_dialog_set(3130@43)".to_string(),
            "6120:add_dialog_set(3127@43)".to_string(),
        ],
        "resolve: {:?}",
        resolved.actions,
    );
}

/// Negative (prerequisite): 742 is only offered once 1200 is done. Anat
/// carries binds from both missions on one template, so a repaint fired
/// before 1200 completes would put the 742 offer on an NPC the 1200 chains
/// still own.
#[tokio::test]
async fn abandoning_742_before_1200_is_done_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(GOAULD, Some(CMD_CENTER)), 742, "not_active");
    let ctx = with_mission(ctx, 1200, "active");

    assert!(
        labels(&fire_mission_abandoned(&engine, &ctx, 742)).is_empty(),
        "chain 6120 must carry `mission_status 1200 eq completed`"
    );
}

/// Negative (wrong archetype): 742 is a Goa'uld mission; the Jaffa gate on
/// the twin chains must not let a Jaffa pick up Anat's topic.
#[tokio::test]
async fn abandoning_742_as_a_non_goauld_resolves_nothing() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let ctx = with_mission(ctx_for(JAFFA, Some(CMD_CENTER)), 742, "not_active");
    let ctx = with_mission(ctx, 1200, "completed");

    assert!(
        labels(&fire_mission_abandoned(&engine, &ctx, 742)).is_empty(),
        "chain 6120 must carry `archetype eq 6`"
    );
}

// ---------------------------------------------------------------
// Structural
// ---------------------------------------------------------------

/// Every `mission_abandoned` chain in the Harset range unbinds before it
/// binds, and never binds a slot it has not just cleared.
///
/// This is the one rule that cannot be read off a single chain: the failure
/// mode is silent (the NPC keeps a stale topic and the click replays an old
/// dialog), and it is easy to reintroduce by appending a `remove_dialog_set`
/// to the end of an action list. Driven over all three chains at once so a
/// fourth abandon chain authored later is covered without editing this file.
#[tokio::test]
async fn every_abandon_chain_unbinds_before_it_rebinds() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    let cases: [(i32, ExecutionContext); 3] = [
        (1324, jaffa_who_abandoned(1324)),
        (1326, jaffa_who_abandoned_1326()),
        (742, goauld_who_abandoned_742()),
    ];

    for (mission_id, ctx) in cases {
        let resolved = fire_mission_abandoned(&engine, &ctx, mission_id);
        let acts: Vec<&Action> = resolved
            .actions
            .iter()
            .filter(|(id, _)| HARSET_CHAIN_RANGE.contains(id))
            .map(|(_, a)| a)
            .collect();
        assert!(
            !acts.is_empty(),
            "mission {mission_id} has no abandon chain — the fixture drifted"
        );

        let first_bind = acts
            .iter()
            .position(|a| matches!(a, Action::AddDialogSet { .. }));
        let last_unbind = acts
            .iter()
            .rposition(|a| matches!(a, Action::RemoveDialogSet { .. }));
        if let (Some(bind), Some(unbind)) = (first_bind, last_unbind) {
            assert!(
                unbind < bind,
                "mission {mission_id}: a remove_dialog_set runs after an \
                 add_dialog_set, so interact.rs can hand the player the stale \
                 dialog. Actions: {acts:?}"
            );
        }

        // Nothing but binds and unbinds: an abandon repaint that granted an
        // item or advanced something would be acting on a mission the player
        // just dropped.
        for a in &acts {
            assert!(
                matches!(
                    a,
                    Action::AddDialogSet { .. } | Action::RemoveDialogSet { .. }
                ),
                "mission {mission_id}: unexpected verb on an abandon chain: {a:?}"
            );
        }
    }
}
