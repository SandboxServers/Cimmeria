//! UO-09: the re-offer after an abandon (63840-63845).
//!
//! Resolve-only. An abandon removes the instance of 1423 or 1500, and the
//! mission then reads `not_active` on `mission_abandoned` (F22), so the
//! re-offer keys on that event as well as on a Command Center load. Each
//! Accept (63842, 63845) takes the mission back, and the mission guard
//! (`mission_status 1655 completed` on all six chains) keeps the re-offer
//! from matching while 1655 is still active, where UO-07's and UO-08's
//! delivery chains own the dialogs.

use cimmeria_content_engine::chain::{ChainEngine, ResolvedActions};
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use super::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_player_loaded, labels, with_mission,
    with_step, ARCH_ASGARD, ARCH_HUMAN, ARCH_JAFFA, CMD_NAME, DLG_BRATAC, DLG_JACKSON, M_1423,
    M_1500, M_1655, S_4942, WORLD_CMD,
};
use crate::cell::content::build_engine;
use crate::test_support::require_db_or_skip;

/// Beta's world id: the re-offer's world gate (an abandon there binds nothing).
const WORLD_BETA: i32 = 23;

/// Past Omega: 1655 completed, with its last step key still `active` (as
/// the context holds it until the next save), and `offer` not held.
fn done(archetype: i64, world: i32, offer: i32) -> ExecutionContext {
    let ctx = with_mission(ctx_for(archetype, Some(world)), M_1655, "completed");
    let ctx = with_step(ctx, M_1655, S_4942, "active");
    with_mission(ctx, offer, "not_active")
}

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

/// The rendered labels of the Human re-offer (63840-63842) only.
fn human_reoffer(all: Vec<String>) -> Vec<String> {
    all.into_iter()
        .filter(|l| {
            ["63840:", "63841:", "63842:"]
                .iter()
                .any(|p| l.starts_with(p))
        })
        .collect()
}

/// The rendered labels of the Free Jaffa re-offer (63843-63845) only.
fn jaffa_reoffer(all: Vec<String>) -> Vec<String> {
    all.into_iter()
        .filter(|l| {
            ["63843:", "63844:", "63845:"]
                .iter()
                .any(|p| l.starts_with(p))
        })
        .collect()
}

/// **A Command Center load after an abandon re-binds the offer (63840, 63843).**
/// Jackson's load binds map 63802 for 1423 and Bra'tac's binds map 63803 for
/// 1500, each as a remove then an add on its slot.
#[tokio::test]
async fn live_db_a_load_after_an_abandon_binds_the_reoffer() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let human = done(ARCH_HUMAN, WORLD_CMD, M_1423);
    assert_eq!(
        labels(&fire_player_loaded(&engine, &human, CMD_NAME)),
        vec![
            "63840:remove_dialog_set(63802@63800)".to_string(),
            "63840:add_dialog_set(63802@63800)".to_string(),
        ],
        "a Human Command Center load with 1423 not held binds Jackson's re-offer"
    );
    let jaffa = done(ARCH_JAFFA, WORLD_CMD, M_1500);
    assert_eq!(
        labels(&fire_player_loaded(&engine, &jaffa, CMD_NAME)),
        vec![
            "63843:remove_dialog_set(63803@63801)".to_string(),
            "63843:add_dialog_set(63803@63801)".to_string(),
        ],
        "a Free Jaffa Command Center load with 1500 not held binds Bra'tac's re-offer"
    );
}

/// **An abandon in the Command Center binds the offer at once (63841, 63844).**
/// The same abandon in Beta (world 23) binds nothing: the world gate holds it
/// until the next Command Center load. A Human abandoning 1500 binds nothing,
/// because the event key names the mission.
#[tokio::test]
async fn live_db_an_abandon_in_the_command_center_binds_at_once() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let human = done(ARCH_HUMAN, WORLD_CMD, M_1423);
    assert_eq!(
        labels(&fire_mission_abandoned(&engine, &human, M_1423)),
        vec![
            "63841:remove_dialog_set(63802@63800)".to_string(),
            "63841:add_dialog_set(63802@63800)".to_string(),
        ],
        "a Human abandon of 1423 in the Command Center binds Jackson's re-offer"
    );
    let jaffa = done(ARCH_JAFFA, WORLD_CMD, M_1500);
    assert_eq!(
        labels(&fire_mission_abandoned(&engine, &jaffa, M_1500)),
        vec![
            "63844:remove_dialog_set(63803@63801)".to_string(),
            "63844:add_dialog_set(63803@63801)".to_string(),
        ],
        "a Free Jaffa abandon of 1500 in the Command Center binds Bra'tac's re-offer"
    );
    let beta_human = done(ARCH_HUMAN, WORLD_BETA, M_1423);
    assert!(
        labels(&fire_mission_abandoned(&engine, &beta_human, M_1423)).is_empty(),
        "an abandon of 1423 in Beta binds nothing"
    );
    let beta_jaffa = done(ARCH_JAFFA, WORLD_BETA, M_1500);
    assert!(
        labels(&fire_mission_abandoned(&engine, &beta_jaffa, M_1500)).is_empty(),
        "an abandon of 1500 in Beta binds nothing"
    );
    assert!(
        labels(&fire_mission_abandoned(&engine, &human, M_1500)).is_empty(),
        "a Human abandon of 1500 binds nothing (the event key names the mission)"
    );
}

/// **The re-offer's Accept takes the mission back (63842, 63845).** Jackson's
/// Accept removes map 63802 and accepts 1423. It completes nothing, grants no
/// gate and resolves no 63822 (its 1655-active guard), and opens no dialog of
/// its own. Bra'tac's Accept does the same for 1500 on map 63803.
#[tokio::test]
async fn live_db_the_reoffer_accept_takes_the_mission_back() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let human = done(ARCH_HUMAN, WORLD_CMD, M_1423);
    let resolved = fire_dialog_choice(&engine, &human, DLG_JACKSON);
    assert_eq!(
        labels(&resolved),
        vec![
            "63842:remove_dialog_set(63802@63800)".to_string(),
            "63842:accept_mission(1423)".to_string(),
        ],
        "Jackson's re-offer Accept takes 1423 back and nothing else"
    );
    assert_eq!(display_dialog_count(&resolved), 0);
    let jaffa = done(ARCH_JAFFA, WORLD_CMD, M_1500);
    let resolved = fire_dialog_choice(&engine, &jaffa, DLG_BRATAC);
    assert_eq!(
        labels(&resolved),
        vec![
            "63845:remove_dialog_set(63803@63801)".to_string(),
            "63845:accept_mission(1500)".to_string(),
        ],
        "Bra'tac's re-offer Accept takes 1500 back and nothing else"
    );
    assert_eq!(display_dialog_count(&resolved), 0);
}

/// **No re-offer while the mission is held, done or 1655 is still active.**
/// With 1423 active, with 1423 completed, or with 1655 active (the delivery
/// chains' state), no 63840-63842 fires on a load, an abandon or the Accept.
#[tokio::test]
async fn live_db_no_reoffer_while_the_mission_is_held_or_done() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let held = with_mission(done(ARCH_HUMAN, WORLD_CMD, M_1423), M_1423, "active");
    let finished = with_mission(done(ARCH_HUMAN, WORLD_CMD, M_1423), M_1423, "completed");
    let delivering = with_mission(
        with_mission(ctx_for(ARCH_HUMAN, Some(WORLD_CMD)), M_1655, "active"),
        M_1423,
        "not_active",
    );
    for (state, ctx) in [
        ("1423 active", held),
        ("1423 completed", finished),
        ("1655 active", delivering),
    ] {
        let mut seen = human_reoffer(labels(&fire_player_loaded(&engine, &ctx, CMD_NAME)));
        seen.extend(human_reoffer(labels(&fire_mission_abandoned(
            &engine, &ctx, M_1423,
        ))));
        seen.extend(human_reoffer(labels(&fire_dialog_choice(
            &engine,
            &ctx,
            DLG_JACKSON,
        ))));
        assert!(
            seen.is_empty(),
            "{state} resolved Jackson's re-offer: {seen:?}"
        );
    }
}

/// **No re-offer crosses factions.** Jackson's three chains ignore Free Jaffa
/// (7), Asgard (5), Goa'uld (8) and a player with no archetype (-1). Bra'tac's
/// three ignore the Human archetypes (1, 4) and the same other factions (5, 8,
/// -1).
#[tokio::test]
async fn live_db_no_reoffer_crosses_factions() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for archetype in [ARCH_JAFFA, ARCH_ASGARD, 8, -1] {
        let ctx = done(archetype, WORLD_CMD, M_1423);
        let mut seen = human_reoffer(labels(&fire_player_loaded(&engine, &ctx, CMD_NAME)));
        seen.extend(human_reoffer(labels(&fire_mission_abandoned(
            &engine, &ctx, M_1423,
        ))));
        seen.extend(human_reoffer(labels(&fire_dialog_choice(
            &engine,
            &ctx,
            DLG_JACKSON,
        ))));
        assert!(
            seen.is_empty(),
            "archetype {archetype} resolved Jackson's re-offer: {seen:?}"
        );
    }
    for archetype in [1, 4, ARCH_ASGARD, 8, -1] {
        let ctx = done(archetype, WORLD_CMD, M_1500);
        let mut seen = jaffa_reoffer(labels(&fire_player_loaded(&engine, &ctx, CMD_NAME)));
        seen.extend(jaffa_reoffer(labels(&fire_mission_abandoned(
            &engine, &ctx, M_1500,
        ))));
        seen.extend(jaffa_reoffer(labels(&fire_dialog_choice(
            &engine, &ctx, DLG_BRATAC,
        ))));
        assert!(
            seen.is_empty(),
            "archetype {archetype} resolved Bra'tac's re-offer: {seen:?}"
        );
    }
}

/// **Outside the Command Center nothing re-offers.** The world gate sits on all
/// six chains, not only the abandon pair: a Beta (world 23) load or Accept
/// resolves no Jackson or Bra'tac re-offer chain.
#[tokio::test]
async fn live_db_the_reoffer_world_gate_holds_on_load_and_accept() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let human = done(ARCH_HUMAN, WORLD_BETA, M_1423);
    let mut seen = human_reoffer(labels(&fire_player_loaded(&engine, &human, CMD_NAME)));
    seen.extend(human_reoffer(labels(&fire_dialog_choice(
        &engine,
        &human,
        DLG_JACKSON,
    ))));
    let jaffa = done(ARCH_JAFFA, WORLD_BETA, M_1500);
    seen.extend(jaffa_reoffer(labels(&fire_player_loaded(
        &engine, &jaffa, CMD_NAME,
    ))));
    seen.extend(jaffa_reoffer(labels(&fire_dialog_choice(
        &engine, &jaffa, DLG_BRATAC,
    ))));
    assert!(
        seen.is_empty(),
        "a re-offer chain resolved outside the Command Center: {seen:?}"
    );
}
