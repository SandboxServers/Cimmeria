//! DK-10c: mission 1570 steps 4902 and 4901 (dakara_e1_m1570_search_chains.sql, 8060-8074).

use super::super::assert_no_deferred_actions;
use super::super::mission_1324::{
    ctx_for, display_dialog_count, fire_interact_tag, fire_player_loaded, with_mission, with_step,
};
use super::*;
use crate::cell::content::engine_loader::build_engine;
use crate::test_support::require_db_or_skip;

/// One SG-18 remains: its interact tag, its first-pickup and last-pickup
/// chains, its dog-tag dialog, its dialog set map and template, and the
/// counter that marks it visited.
struct Site {
    tag: &'static str,
    first: i64,
    last: i64,
    dialog: i32,
    dsm: i32,
    slot: i32,
    marker: &'static str,
}

const SITES: [Site; 3] = [
    Site {
        tag: "Dakara_E1_SG18_Louis",
        first: 8065,
        last: 8066,
        dialog: 6224,
        dsm: 7153,
        slot: 450,
        marker: "sg18_louis",
    },
    Site {
        tag: "Dakara_E1_SG18_Nguyn",
        first: 8067,
        last: 8068,
        dialog: 6225,
        dsm: 7154,
        slot: 451,
        marker: "sg18_nguyn",
    },
    Site {
        tag: "Dakara_E1_SG18_Waters",
        first: 8069,
        last: 8070,
        dialog: 6226,
        dsm: 7155,
        slot: 452,
        marker: "sg18_waters",
    },
];

/// Mission 1570 active at `step`, in Dakara_E1, with no counters set.
fn at_step(step: i32) -> ExecutionContext {
    with_step(
        with_mission(ctx_for(FREE_JAFFA, Some(DAKARA)), 1570, "active"),
        1570,
        step,
        "active",
    )
}

/// Step 4902 with the search objectives in `completed` set to completed.
fn searching(completed: &[i32]) -> ExecutionContext {
    completed.iter().fold(at_step(4902), |ctx, &o| {
        with_objective(ctx, 1570, o, "completed")
    })
}

/// Whether chain `chain` is among the resolved actions.
fn fired(resolved: &ResolvedActions, chain: i64) -> bool {
    resolved.actions.iter().any(|(id, _)| *id == chain)
}

/// The labels of the 8061 / 8063 shape: "5814, advance to 4901, bind the three remains".
fn last_search_labels(chain: i64) -> Vec<String> {
    vec![
        format!("{chain}:display_dialog(5814)"),
        format!("{chain}:advance_step(1570->4901)"),
        format!("{chain}:add_dialog_set(7153@450)"),
        format!("{chain}:add_dialog_set(7154@451)"),
        format!("{chain}:add_dialog_set(7155@452)"),
    ]
}

/// The labels of a first-or-second pickup chain for `site`.
fn first_labels(site: &Site) -> Vec<String> {
    let c = site.first;
    vec![
        format!("{c}:display_dialog({})", site.dialog),
        format!("{c}:add_item(6778x1)"),
        format!("{c}:complete_objective(1570/5930)"),
        format!("{c}:increment_counter(sg18_remains+1)"),
        format!("{c}:increment_counter({}+1)", site.marker),
        format!("{c}:remove_dialog_set({}@{})", site.dsm, site.slot),
    ]
}

/// The labels of a third-pickup chain for `site`: leave 4901.
fn last_labels(site: &Site) -> Vec<String> {
    let c = site.last;
    vec![
        format!("{c}:display_dialog({})", site.dialog),
        format!("{c}:add_item(6778x1)"),
        format!("{c}:advance_step(1570->4905)"),
        format!("{c}:reset_counter(sg18_remains)"),
        format!("{c}:reset_counter(sg18_louis)"),
        format!("{c}:reset_counter(sg18_nguyn)"),
        format!("{c}:reset_counter(sg18_waters)"),
        format!("{c}:remove_dialog_set({}@{})", site.dsm, site.slot),
    ]
}

#[tokio::test]
async fn live_db_louis_first_then_last_region() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let first = fire_enter_region(&engine, &searching(&[]), "Dakara_E1.SG18Louis");
    assert_eq!(
        labels_in(&first, SEARCH_CHAINS),
        vec!["8060:complete_objective(1570/5632)"]
    );
    assert_no_deferred_actions(&first, 8060);
    let done = fire_enter_region(&engine, &searching(&[5632]), "Dakara_E1.SG18Louis");
    assert!(labels_in(&done, SEARCH_CHAINS).is_empty());
    let last = fire_enter_region(&engine, &searching(&[5630]), "Dakara_E1.SG18Louis");
    assert_eq!(labels_in(&last, SEARCH_CHAINS), last_search_labels(8061));
    assert_eq!(display_dialog_count(&last), 1);
    assert_no_deferred_actions(&last, 8061);
}

#[tokio::test]
async fn live_db_nguyn_first_then_last_region() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let first = fire_enter_region(&engine, &searching(&[]), "Dakara_E1.SG18Nguyn");
    assert_eq!(
        labels_in(&first, SEARCH_CHAINS),
        vec!["8062:complete_objective(1570/5630)"]
    );
    assert_no_deferred_actions(&first, 8062);
    let done = fire_enter_region(&engine, &searching(&[5630]), "Dakara_E1.SG18Nguyn");
    assert!(labels_in(&done, SEARCH_CHAINS).is_empty());
    let last = fire_enter_region(&engine, &searching(&[5632]), "Dakara_E1.SG18Nguyn");
    assert_eq!(labels_in(&last, SEARCH_CHAINS), last_search_labels(8063));
    assert_eq!(display_dialog_count(&last), 1);
    assert_no_deferred_actions(&last, 8063);
}

#[tokio::test]
async fn live_db_waters_is_optional_and_checked_once() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let pairs: [&[i32]; 4] = [&[], &[5630], &[5632], &[5630, 5632]];
    for pair in pairs {
        let open = fire_enter_region(&engine, &searching(pair), "Dakara_E1.SG18Waters");
        assert_eq!(
            labels_in(&open, SEARCH_CHAINS),
            vec!["8064:complete_objective(1570/5631)"]
        );
        let mut completed = pair.to_vec();
        completed.push(5631);
        let done = fire_enter_region(&engine, &searching(&completed), "Dakara_E1.SG18Waters");
        assert!(labels_in(&done, SEARCH_CHAINS).is_empty());
    }
}

#[tokio::test]
async fn live_db_search_regions_need_world_61_and_4902() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let story_rm = with_step(
        with_mission(ctx_for(FREE_JAFFA, Some(STORY_RM)), 1570, "active"),
        1570,
        4902,
        "active",
    );
    let contexts = [story_rm, at_step(4904), at_step(4901)];
    let regions = [
        "Dakara_E1.SG18Louis",
        "Dakara_E1.SG18Nguyn",
        "Dakara_E1.SG18Waters",
    ];
    for ctx in &contexts {
        for region in regions {
            let r = fire_enter_region(&engine, ctx, region);
            assert!(labels_in(&r, SEARCH_CHAINS).is_empty(), "{region}");
        }
    }
}

#[tokio::test]
async fn live_db_relog_at_4902_keeps_the_last_region() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // A relog keeps completed objectives and drops every counter: this
    // context has no counters at all.
    let r = fire_enter_region(&engine, &searching(&[5630]), "Dakara_E1.SG18Louis");
    assert_eq!(labels_in(&r, SEARCH_CHAINS), last_search_labels(8061));
}

#[tokio::test]
async fn live_db_first_pickup_per_site() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for site in &SITES {
        for remains in [0, 1] {
            let ctx = with_counter(
                with_counter(at_step(4901), "sg18_remains", remains),
                site.marker,
                0,
            );
            let r = fire_interact_tag(&engine, &ctx, site.tag);
            assert_eq!(labels_in(&r, SEARCH_CHAINS), first_labels(site));
            assert_eq!(display_dialog_count(&r), 1);
            assert_no_deferred_actions(&r, site.first);
        }
        // Not with two remains already collected, nor with this remains visited.
        let full = with_counter(
            with_counter(at_step(4901), "sg18_remains", 2),
            site.marker,
            0,
        );
        let visited = with_counter(
            with_counter(at_step(4901), "sg18_remains", 0),
            site.marker,
            1,
        );
        for ctx in [&full, &visited] {
            assert!(!fired(
                &fire_interact_tag(&engine, ctx, site.tag),
                site.first
            ));
        }
    }
}

#[tokio::test]
async fn live_db_third_pickup_leaves_4901() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for site in &SITES {
        let ctx = with_counter(
            with_counter(at_step(4901), "sg18_remains", 2),
            site.marker,
            0,
        );
        let r = fire_interact_tag(&engine, &ctx, site.tag);
        assert_eq!(labels_in(&r, SEARCH_CHAINS), last_labels(site));
        assert_eq!(display_dialog_count(&r), 1);
        assert_no_deferred_actions(&r, site.last);
        // One remains collected is not the third pickup.
        let one = with_counter(
            with_counter(at_step(4901), "sg18_remains", 1),
            site.marker,
            0,
        );
        assert!(!fired(
            &fire_interact_tag(&engine, &one, site.tag),
            site.last
        ));
        // Revert proof for the marker: with this remains already visited, the
        // third pickup resolves nothing in the search range.
        let visited = with_counter(
            with_counter(at_step(4901), "sg18_remains", 2),
            site.marker,
            1,
        );
        let r = fire_interact_tag(&engine, &visited, site.tag);
        assert!(labels_in(&r, SEARCH_CHAINS).is_empty());
    }
}

#[tokio::test]
async fn live_db_remains_restore_on_load_at_4901_only() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let r = fire_player_loaded(&engine, &at_step(4901), DAKARA_NAME);
    assert_eq!(
        labels_in(&r, SEARCH_CHAINS),
        vec![
            "8071:add_dialog_set(7153@450)",
            "8071:add_dialog_set(7154@451)",
            "8071:add_dialog_set(7155@452)",
        ]
    );
    assert_no_deferred_actions(&r, 8071);
    for step in [4905, 4902] {
        let other = fire_player_loaded(&engine, &at_step(step), DAKARA_NAME);
        assert!(labels_in(&other, SEARCH_CHAINS).is_empty());
    }
}

#[tokio::test]
async fn live_db_relog_at_4901_rearms_all_three_remains() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    // A relog gives every counter 0: the binds come back, and the first
    // click on any remains is the first pickup, not the advancing one.
    let restore = fire_player_loaded(&engine, &at_step(4901), DAKARA_NAME);
    assert_eq!(labels_in(&restore, SEARCH_CHAINS).len(), 3);
    for site in &SITES {
        let r = fire_interact_tag(&engine, &at_step(4901), site.tag);
        assert_eq!(labels_in(&r, SEARCH_CHAINS), first_labels(site));
    }
}
