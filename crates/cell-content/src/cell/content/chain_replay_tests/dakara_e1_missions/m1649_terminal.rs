//! DK-14b: mission 1649 "Moh'katan", step 4922 (dakara_e1_m1649_terminal_chains.sql, 8220-8249).
//! Shares the 4922 context with DK-14a's offer chains; asserts only its own range.

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_interact_tag, fire_player_loaded, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

const MISSION: i32 = 1649;
const STEP_TERMINAL: i32 = 4922;
const STEP_RADIO: i32 = 4921;
const OBJ_HACK: i32 = 5638;
const OBJ_FIND: i32 = 5639;
const TERMINAL: &str = "Dakara_E1_CommandTerminal";

/// Drop tags with the carrier slot each one binds (template 449, 458-461).
const DROPS: [(&str, i64); 5] = [
    ("Dakara_E1_Drop1", 449),
    ("Dakara_E1_Drop2", 458),
    ("Dakara_E1_Drop3", 459),
    ("Dakara_E1_Drop4", 460),
    ("Dakara_E1_Drop5", 461),
];

/// The context for mission 1649 at `step` in `world`, with 1648 and 1646
/// completed (the shared rule for every 4922 context).
fn at(world: Option<i32>, step: i32) -> ExecutionContext {
    let ctx = with_mission(ctx_for(FREE_JAFFA, world), 1648, "completed");
    let ctx = with_mission(ctx, 1646, "completed");
    let ctx = with_mission(ctx, MISSION, "active");
    with_step(ctx, MISSION, step, "active")
}

/// Step 4922 in Dakara_E1 with the given objectives of 1649 completed.
fn terminal_ctx(done: &[i32]) -> ExecutionContext {
    let mut ctx = at(Some(DAKARA), STEP_TERMINAL);
    for objective in done {
        ctx = with_objective(ctx, MISSION, *objective, "completed");
    }
    ctx
}

/// Labels of the chains of this packet in `resolved`.
fn terminal_labels(resolved: &ResolvedActions) -> Vec<String> {
    labels_in(resolved, M1649_TERMINAL_CHAINS)
}

/// The six `add_dialog_set` labels for the terminal and the five drops.
fn all_bind_labels(chain: i64) -> Vec<String> {
    let mut out = vec![format!("{chain}:add_dialog_set(6760@453)")];
    for (_, slot) in DROPS {
        out.push(format!("{chain}:add_dialog_set(6760@{slot})"));
    }
    out
}

#[tokio::test]
async fn live_db_entry_binds_terminal_and_drops_until_message_found() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &terminal_ctx(&[]), DAKARA_NAME);
    let mut expected = vec!["8220:add_dialog_set(6760@453)".to_string()];
    expected.extend(
        DROPS
            .iter()
            .map(|(_, slot)| format!("8221:add_dialog_set(6760@{slot})")),
    );
    assert_eq!(terminal_labels(&r), expected);
    assert_no_deferred_actions(&r, 8220);
    // Once the message is found the drops stay unbound: only 8220 is left.
    let done = terminal_ctx(&[OBJ_FIND]);
    let r = fire_player_loaded(&engine, &done, DAKARA_NAME);
    assert_eq!(terminal_labels(&r), vec!["8220:add_dialog_set(6760@453)"]);
}

#[tokio::test]
async fn live_db_radio_step_load_binds_all_six_and_skips_4922_entry() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at(Some(DAKARA), STEP_RADIO), DAKARA_NAME);
    assert_eq!(terminal_labels(&r), all_bind_labels(8222));
    assert_no_deferred_actions(&r, 8222);
}

#[tokio::test]
async fn live_db_terminal_states_are_exclusive() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let hack = [
        "8223:display_dialog(5834)",
        "8223:add_item(6782x1)",
        "8223:complete_objective(1649/5638)",
    ];
    let decode = [
        "8224:display_dialog(5837)",
        "8224:remove_item(6780x1)",
        "8224:add_item(6783x1)",
        "8224:advance_step(1649->4920)",
        "8224:remove_dialog_set(6760@453)",
    ];
    // The whole label list per state: the union over 8220-8249 is exact, so
    // an extra or missing verb on the winning chain, or a second chain, fails.
    let cases: [(&[i32], &[&str]); 4] = [
        (&[], &hack),
        (&[OBJ_FIND], &hack),
        (&[OBJ_HACK], &["8225:display_dialog(5834)"]),
        (&[OBJ_HACK, OBJ_FIND], &decode),
    ];
    for (done, expected) in cases {
        let r = fire_interact_tag(&engine, &terminal_ctx(done), TERMINAL);
        assert_eq!(terminal_labels(&r), expected, "done {done:?}");
        assert_eq!(display_dialog_count(&r), 1, "done {done:?}");
    }
}

#[tokio::test]
async fn live_db_terminal_chains_need_dakara_and_step_4922() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let tags: Vec<&str> = std::iter::once(TERMINAL)
        .chain(DROPS.iter().map(|(tag, _)| *tag))
        .collect();
    // Load: no chain of this packet may fire (`no_load`). Tags: no chain of
    // this packet may fire on the terminal or any drop (`no_tag`).
    let no_load = |ctx: &ExecutionContext, why: &str| {
        let r = fire_player_loaded(&engine, ctx, DAKARA_NAME);
        assert!(
            terminal_labels(&r).is_empty(),
            "load, {why}: {:?}",
            terminal_labels(&r)
        );
    };
    let no_tag = |ctx: &ExecutionContext, why: &str| {
        for &tag in &tags {
            let r = fire_interact_tag(&engine, ctx, tag);
            assert!(
                terminal_labels(&r).is_empty(),
                "{tag}, {why}: {:?}",
                terminal_labels(&r)
            );
        }
    };
    // Wrong world, or no world: Condition::World fails closed.
    for world in [Some(STORY_RM), None] {
        let ctx = at(world, STEP_TERMINAL);
        no_load(&ctx, &format!("world {world:?}"));
        no_tag(&ctx, &format!("world {world:?}"));
    }
    // Wrong step: 4920 is before the step, and 4921 loads only 8222 (see
    // live_db_radio_step_load_binds_all_six_and_skips_4922_entry).
    let before = at(Some(DAKARA), 4920);
    no_load(&before, "step 4920");
    no_tag(&before, "step 4920");
    no_tag(&at(Some(DAKARA), STEP_RADIO), "step 4921");
    // Step 4922 already completed: the chains read the step's status.
    let left = with_step(terminal_ctx(&[]), MISSION, STEP_TERMINAL, "completed");
    no_load(&left, "4922 completed");
    no_tag(&left, "4922 completed");
}

#[tokio::test]
async fn live_db_hack_gives_orders_and_completes_objective() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &terminal_ctx(&[]), TERMINAL);
    assert_eq!(
        terminal_labels(&r),
        vec![
            "8223:display_dialog(5834)",
            "8223:add_item(6782x1)",
            "8223:complete_objective(1649/5638)",
        ]
    );
}

#[tokio::test]
async fn live_db_decode_takes_message_gives_decoded_and_leaves_4922() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &terminal_ctx(&[OBJ_HACK, OBJ_FIND]), TERMINAL);
    assert_eq!(
        terminal_labels(&r),
        vec![
            "8224:display_dialog(5837)",
            "8224:remove_item(6780x1)",
            "8224:add_item(6783x1)",
            "8224:advance_step(1649->4920)",
            "8224:remove_dialog_set(6760@453)",
        ]
    );
    assert_no_deferred_actions(&r, 8224);
}

#[tokio::test]
async fn live_db_drop5_holds_message_and_unbinds_all_drops_once() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_interact_tag(&engine, &terminal_ctx(&[OBJ_HACK]), "Dakara_E1_Drop5");
    let mut expected = vec![
        "8226:display_dialog(5835)".to_string(),
        "8226:add_item(6780x1)".to_string(),
        "8226:complete_objective(1649/5639)".to_string(),
    ];
    expected.extend(
        DROPS
            .iter()
            .map(|(_, slot)| format!("8226:remove_dialog_set(6760@{slot})")),
    );
    assert_eq!(terminal_labels(&r), expected);
    assert_eq!(display_dialog_count(&r), 1);
}

#[tokio::test]
async fn live_db_drops_one_to_four_say_taken_and_drops_close_once_found() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for (k, (tag, _)) in DROPS[..4].iter().enumerate() {
        let r = fire_interact_tag(&engine, &terminal_ctx(&[OBJ_HACK]), tag);
        let chain = 8227 + k as i64;
        assert_eq!(
            terminal_labels(&r),
            vec![format!("{chain}:display_dialog(5836)")],
            "drop {}",
            k + 1
        );
        assert_eq!(display_dialog_count(&r), 1, "drop {}", k + 1);
    }
    // With the message found, no drop chain and no drop bind resolves.
    let found = terminal_ctx(&[OBJ_FIND]);
    for (tag, _) in DROPS {
        let r = fire_interact_tag(&engine, &found, tag);
        assert!(terminal_labels(&r).is_empty(), "{tag} after 5639");
    }
    // Positive on the same context: only the entry bind 8220 is left.
    let r = fire_player_loaded(&engine, &found, DAKARA_NAME);
    assert_eq!(terminal_labels(&r), vec!["8220:add_dialog_set(6760@453)"]);
}
