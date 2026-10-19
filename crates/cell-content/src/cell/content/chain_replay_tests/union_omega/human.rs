//! UO-07: Jackson's induction (63820-63822).
//!
//! Resolve-only. The first beam (63820) binds Jackson's delivery map 63800 on
//! S4940, because on the first beam both `teleport_in` and `player_loaded`
//! see S4940 (F9), so a bind keyed on S4942 at `player_loaded` would miss it.
//! A Command Center load at S4942 rebinds it (63821). The Accept on 4641
//! (63822) removes the bind before it completes 1655 (F5), and the mission
//! guard (`mission_status 1655 active`) on all three chains keeps a completed
//! 1655 from firing the Accept or the bind again.

use cimmeria_content_engine::context::ExecutionContext;

use super::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_player_loaded, fire_teleport_in,
    labels, with_mission, with_step, ARCH_ASGARD, ARCH_HUMAN, ARCH_JAFFA, CMD_NAME, DLG_JACKSON,
    M_1423, M_1655, REGION_CMD_PAD, S_4940, S_4942, WORLD_CMD, WORLD_HUB,
};
use crate::cell::content::build_engine;
use crate::test_support::require_db_or_skip;

/// In the Command Center at `step`, with 1423 not taken.
fn at(archetype: i64, world: i32, step: i32) -> ExecutionContext {
    let ctx = with_mission(ctx_for(archetype, Some(world)), M_1655, "active");
    let ctx = with_step(ctx, M_1655, step, "active");
    with_mission(ctx, M_1423, "not_active")
}

/// The rendered labels from one chain id, so a test reads one chain at a time.
fn only(chain: i64, all: Vec<String>) -> Vec<String> {
    let prefix = format!("{chain}:");
    all.into_iter().filter(|l| l.starts_with(&prefix)).collect()
}

/// The rendered labels of Jackson's three chains only, so another chain's
/// action (the beam's 63801 advance, for one) does not fail a check.
fn jackson(all: Vec<String>) -> Vec<String> {
    all.into_iter()
        .filter(|l| {
            ["63820:", "63821:", "63822:"]
                .iter()
                .any(|p| l.starts_with(p))
        })
        .collect()
}

/// **The first beam binds Jackson's delivery map, once, for Humans (F9).**
/// Chain 63820 removes and then adds map 63800 on slot 63800. Past S4940 the
/// beam resolves no 63820.
#[tokio::test]
async fn live_db_a_human_beam_binds_jackson_once() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for archetype in [1, 2, 3, 4] {
        let ctx = at(archetype, WORLD_CMD, S_4940);
        assert_eq!(
            only(
                63820,
                labels(&fire_teleport_in(&engine, &ctx, REGION_CMD_PAD))
            ),
            vec![
                "63820:remove_dialog_set(63800@63800)".to_string(),
                "63820:add_dialog_set(63800@63800)".to_string(),
            ],
            "archetype {archetype} binds Jackson on the first beam"
        );
        let past = at(archetype, WORLD_CMD, S_4942);
        assert!(
            only(
                63820,
                labels(&fire_teleport_in(&engine, &past, REGION_CMD_PAD))
            )
            .is_empty(),
            "archetype {archetype} past S4940 binds nothing on the beam"
        );
    }
}

/// **A relog or a second beam at S4942 rebinds Jackson (63821).** At S4940
/// the same Command Center load resolves no 63821 (F9).
#[tokio::test]
async fn live_db_a_relog_at_4942_rebinds_jackson() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let ctx = at(ARCH_HUMAN, WORLD_CMD, S_4942);
    assert_eq!(
        labels(&fire_player_loaded(&engine, &ctx, CMD_NAME)),
        vec![
            "63821:remove_dialog_set(63800@63800)".to_string(),
            "63821:add_dialog_set(63800@63800)".to_string(),
        ],
        "a Command Center load at S4942 rebinds Jackson"
    );
    let early = at(ARCH_HUMAN, WORLD_CMD, S_4940);
    assert!(
        only(
            63821,
            labels(&fire_player_loaded(&engine, &early, CMD_NAME))
        )
        .is_empty(),
        "a Command Center load at S4940 resolves no 63821"
    );
}

/// **Jackson's Accept completes 1655 and offers The SGU (63822).** Map 63800
/// comes off first (F5), then 1655 completes, 1423 is accepted and Beta's gate
/// (6) is granted. The Accept opens no dialog of its own.
#[tokio::test]
async fn live_db_jacksons_accept_completes_1655_and_offers_the_sgu() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let ctx = at(ARCH_HUMAN, WORLD_CMD, S_4942);
    let resolved = fire_dialog_choice(&engine, &ctx, DLG_JACKSON);
    assert_eq!(
        labels(&resolved),
        vec![
            "63822:remove_dialog_set(63800@63800)".to_string(),
            "63822:complete_mission(1655)".to_string(),
            "63822:accept_mission(1423)".to_string(),
            "63822:grant_stargate_address(6)".to_string(),
        ],
    );
    assert_eq!(display_dialog_count(&resolved), 0);
}

/// **No other faction gets Jackson's offer.** Free Jaffa (7), Asgard (5) and
/// a player with no archetype (-1) resolve none of 63820-63822 on the beam,
/// the load or the Accept.
#[tokio::test]
async fn live_db_no_other_faction_gets_jacksons_offer() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for archetype in [ARCH_JAFFA, ARCH_ASGARD, -1] {
        let at_4940 = at(archetype, WORLD_CMD, S_4940);
        let at_4942 = at(archetype, WORLD_CMD, S_4942);
        let mut seen = jackson(labels(&fire_teleport_in(&engine, &at_4940, REGION_CMD_PAD)));
        seen.extend(jackson(labels(&fire_player_loaded(
            &engine, &at_4942, CMD_NAME,
        ))));
        seen.extend(jackson(labels(&fire_dialog_choice(
            &engine,
            &at_4942,
            DLG_JACKSON,
        ))));
        assert!(
            seen.is_empty(),
            "archetype {archetype} resolved Jackson's chains: {seen:?}"
        );
    }
}

/// **The mission guard and the state gates hold (F5 and the 1655 guard).**
/// With 1423 already taken, with 1655 completed (its S4942 key still reads
/// `active`), or in the hub, no Jackson chain fires on any of the three
/// events. At S4940 the Accept completes nothing.
#[tokio::test]
async fn live_db_jackson_ignores_the_wrong_state() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let taken = with_mission(at(ARCH_HUMAN, WORLD_CMD, S_4942), M_1423, "active");
    let done = with_mission(at(ARCH_HUMAN, WORLD_CMD, S_4942), M_1655, "completed");
    let hub = at(ARCH_HUMAN, WORLD_HUB, S_4942);
    for (state, ctx) in [
        ("1423 active", taken),
        ("1655 completed", done),
        ("hub", hub),
    ] {
        let mut seen = jackson(labels(&fire_teleport_in(&engine, &ctx, REGION_CMD_PAD)));
        seen.extend(jackson(labels(&fire_player_loaded(
            &engine, &ctx, CMD_NAME,
        ))));
        seen.extend(jackson(labels(&fire_dialog_choice(
            &engine,
            &ctx,
            DLG_JACKSON,
        ))));
        assert!(
            seen.is_empty(),
            "{state} resolved Jackson's chains: {seen:?}"
        );
    }
    let early = at(ARCH_HUMAN, WORLD_CMD, S_4940);
    assert!(
        jackson(labels(&fire_dialog_choice(&engine, &early, DLG_JACKSON))).is_empty(),
        "the Accept at S4940 completes nothing"
    );
}
