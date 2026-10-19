//! DK-12c: mission 1646, step 4908 five drops (dakara_e1_m1646_drop_chains.sql, 8140-8159).

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_interact_tag, fire_player_loaded, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

const MISSION: i32 = 1646;
const STEP_DROPS: i32 = 4908;
const STEP_LEAVE: i32 = 4909;
/// The counter the count chains read and write (work-packets.md, C4).
const COUNTER: &str = "drops_4908";

/// (tag, objective, template slot, dialog set map) for drops 1 to 5.
const DROPS: [(&str, i32, i32, i32); 5] = [
    ("Dakara_E1_Drop1", 5635, 449, 6739),
    ("Dakara_E1_Drop2", 5939, 458, 6740),
    ("Dakara_E1_Drop3", 5940, 459, 6741),
    ("Dakara_E1_Drop4", 5941, 460, 6742),
    ("Dakara_E1_Drop5", 5942, 461, 6743),
];

/// Chain id of drop `k` (0-based) while fewer than four drops are counted.
fn count_chain(k: usize) -> i64 {
    8140 + 2 * k as i64
}

/// Chain id of drop `k` when it is the fifth drop and leaves 4908.
fn last_chain(k: usize) -> i64 {
    8141 + 2 * k as i64
}

/// Chain id of the restore bind for drop `k`.
fn restore_chain(k: usize) -> i64 {
    8150 + k as i64
}

/// Chain id of the count rebuild for drop `k`.
fn rebuild_chain(k: usize) -> i64 {
    8155 + k as i64
}

/// The context for mission 1646 at step `step`, with 1570 and 1645 completed
/// and the player in `world`.
fn at(world: Option<i32>, step: i32) -> ExecutionContext {
    let ctx = with_mission(ctx_for(FREE_JAFFA, world), 1570, "completed");
    let ctx = with_mission(ctx, 1645, "completed");
    let ctx = with_mission(ctx, MISSION, "active");
    with_step(ctx, MISSION, step, "active")
}

/// Step 4908 in Dakara_E1, with the given drop objectives completed and the
/// counter at `counter`.
fn drop_ctx(done: &[i32], counter: i64) -> ExecutionContext {
    let mut ctx = at(Some(DAKARA), STEP_DROPS);
    for objective in done {
        ctx = with_objective(ctx, MISSION, *objective, "completed");
    }
    with_counter(ctx, COUNTER, counter)
}

/// Labels of the count chain for drop `k`.
fn count_labels(k: usize) -> Vec<String> {
    let c = count_chain(k);
    let (_, objective, slot, dsm) = DROPS[k];
    vec![
        format!("{c}:display_dialog(5821)"),
        format!("{c}:complete_objective(1646/{objective})"),
        format!("{c}:increment_counter(drops_4908+1)"),
        format!("{c}:remove_dialog_set({dsm}@{slot})"),
    ]
}

/// Labels of the fifth-drop chain for drop `k`.
fn last_labels(k: usize) -> Vec<String> {
    let c = last_chain(k);
    let (_, _, slot, dsm) = DROPS[k];
    vec![
        format!("{c}:display_dialog(5821)"),
        format!("{c}:advance_step(1646->4909)"),
        format!("{c}:reset_counter(drops_4908)"),
        format!("{c}:remove_dialog_set({dsm}@{slot})"),
    ]
}

/// The restore label for drop `k`.
fn restore_label(k: usize) -> String {
    let (_, _, slot, dsm) = DROPS[k];
    format!("{}:add_dialog_set({dsm}@{slot})", restore_chain(k))
}

/// The rebuild label for drop `k`.
fn rebuild_label(k: usize) -> String {
    format!("{}:increment_counter(drops_4908+1)", rebuild_chain(k))
}

/// Labels of the drop chains in `resolved`, sorted (the order across chains
/// is not part of the contract).
fn sorted_drop_labels(resolved: &ResolvedActions) -> Vec<String> {
    let mut got = labels_in(resolved, M1646_DROP_CHAINS);
    got.sort();
    got
}

/// Labels of the single chain `chain` in `resolved`.
fn chain_labels(resolved: &ResolvedActions, chain: i64) -> Vec<String> {
    labels_in(resolved, chain..=chain)
}

fn sorted(mut labels: Vec<String>) -> Vec<String> {
    labels.sort();
    labels
}

#[tokio::test]
async fn live_db_drop_count_chain_fires_for_each_drop_until_four() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for (k, (tag, objective, _, _)) in DROPS.iter().enumerate() {
        for counter in [0, 3] {
            let r = fire_interact_tag(&engine, &drop_ctx(&[], counter), tag);
            assert_eq!(labels_in(&r, M1646_DROP_CHAINS), count_labels(k));
            assert_eq!(display_dialog_count(&r), 1, "drop {k}, counter {counter}");
        }
        let done = drop_ctx(&[*objective], 0);
        let r = fire_interact_tag(&engine, &done, tag);
        assert!(labels_in(&r, M1646_DROP_CHAINS).is_empty(), "drop {k} done");
    }
}

#[tokio::test]
async fn live_db_fifth_drop_leaves_4908_and_never_completes_objective() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for (k, (tag, objective, _, _)) in DROPS.iter().enumerate() {
        let r = fire_interact_tag(&engine, &drop_ctx(&[], 4), tag);
        assert_eq!(labels_in(&r, M1646_DROP_CHAINS), last_labels(k));
        assert_eq!(display_dialog_count(&r), 1, "drop {k}");
        assert_no_deferred_actions(&r, last_chain(k));

        // Three counted drops give the count chain, not the leave.
        let three = fire_interact_tag(&engine, &drop_ctx(&[], 3), tag);
        assert!(chain_labels(&three, last_chain(k)).is_empty());

        // A done objective blocks the leave as well.
        let done = fire_interact_tag(&engine, &drop_ctx(&[*objective], 4), tag);
        assert!(
            labels_in(&done, M1646_DROP_CHAINS).is_empty(),
            "drop {k} done"
        );
    }
}

#[tokio::test]
async fn live_db_first_entry_restores_all_five_binds_without_rebuild() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &drop_ctx(&[], 0), DAKARA_NAME);
    let want: Vec<String> = (0..DROPS.len()).map(restore_label).collect();
    assert_eq!(sorted_drop_labels(&r), sorted(want));
    for c in 8150..=8154 {
        assert_no_deferred_actions(&r, c);
    }
}

#[tokio::test]
async fn live_db_relog_restores_open_drops_and_rebuilds_count() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // Drops 1 and 2 were done before the relog; the count is 0 again.
    let relog = drop_ctx(&[5635, 5939], 0);
    let r = fire_player_loaded(&engine, &relog, DAKARA_NAME);
    let mut want = vec![restore_label(2), restore_label(3), restore_label(4)];
    want.push(rebuild_label(0));
    want.push(rebuild_label(1));
    assert_eq!(sorted_drop_labels(&r), sorted(want));

    // Then the open drops count from the rebuilt value.
    let drop3 = fire_interact_tag(&engine, &drop_ctx(&[5635, 5939], 2), "Dakara_E1_Drop3");
    assert_eq!(labels_in(&drop3, M1646_DROP_CHAINS), count_labels(2));
    let drop4 = fire_interact_tag(&engine, &drop_ctx(&[5635, 5939], 3), "Dakara_E1_Drop4");
    assert_eq!(labels_in(&drop4, M1646_DROP_CHAINS), count_labels(3));
    let drop5 = fire_interact_tag(&engine, &drop_ctx(&[5635, 5939], 4), "Dakara_E1_Drop5");
    assert_eq!(labels_in(&drop5, M1646_DROP_CHAINS), last_labels(4));
}

#[tokio::test]
async fn live_db_drop_chains_need_dakara_and_step_4908() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for (tag, _, _, _) in DROPS {
        for counter in [0, 4] {
            // In the tent (world 62) at 4908, and in Dakara_E1 at 4909.
            let in_tent = with_counter(at(Some(STORY_RM), STEP_DROPS), COUNTER, counter);
            let leaving = with_counter(at(Some(DAKARA), STEP_LEAVE), COUNTER, counter);
            let r = fire_interact_tag(&engine, &in_tent, tag);
            assert!(
                labels_in(&r, M1646_DROP_CHAINS).is_empty(),
                "{tag} in world 62"
            );
            let r = fire_interact_tag(&engine, &leaving, tag);
            assert!(labels_in(&r, M1646_DROP_CHAINS).is_empty(), "{tag} at 4909");
        }
    }
    let in_tent = at(Some(STORY_RM), STEP_DROPS);
    let r = fire_player_loaded(&engine, &in_tent, DAKARA_NAME);
    assert!(labels_in(&r, M1646_DROP_CHAINS).is_empty());
    let leaving = at(Some(DAKARA), STEP_LEAVE);
    let r = fire_player_loaded(&engine, &leaving, DAKARA_NAME);
    assert!(labels_in(&r, M1646_DROP_CHAINS).is_empty());
}
