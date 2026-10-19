//! UO-13: Omega's departure states, exported for Beta's replay tests.
//!
//! The Producing contract (`omega-work-packets.md` § Contracts):
//!
//! | Route | Omega leaves | Beta's first event |
//! |---|---|---|
//! | Human (archetypes 1-4) | 1655 `completed` (its S4942 key may still read `active`); 1423 `active` at S4223; gate 6 granted | Beta 23020 on arrival in world 23 |
//! | Free Jaffa (7) | 1655 `completed`; 1500 `active` at S4369; gate 6 granted | Beta 23220 |
//!
//! Beta's receiver tests (UB-10, UB-20, UB-30) may import these once UO-13 is
//! on main. Change them only with Beta's coordinator: both suites assert the
//! same table row, and this file is how they cannot drift apart.
//!
//! The fixtures encode three transitions that Omega's Accept chains resolve
//! (63822 for Jackson, 63832 for Bra'tac): `complete_mission(1655)`,
//! `accept_mission(offer)` and `grant_stargate_address(6)`. The tests below
//! pin that link, so a seed change to an Accept cannot silently leave the
//! fixtures describing a state Omega no longer produces. The gate grant is
//! player state the context does not carry; [`DEPARTURE_GATE`] names it.

use cimmeria_content_engine::context::ExecutionContext;

use super::executed::fire_mission_abandoned;
use super::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_player_loaded, fire_teleport_in,
    labels, with_mission, with_step, ARCH_HUMAN, ARCH_JAFFA, DLG_BRATAC, DLG_JACKSON, DLG_MODI,
    M_1423, M_1500, M_1655, REGION_CMD_PAD, S_4942, WORLD_CMD,
};
use crate::cell::content::build_engine;
use crate::test_support::require_db_or_skip;

/// Beta's world id (`Beta_Site_Evo_1`, `worlds.sql`).
pub(in crate::cell::content::chain_replay_tests) const WORLD_BETA: i32 = 23;
/// The gate Omega grants on departure.
pub(in crate::cell::content::chain_replay_tests) const DEPARTURE_GATE: i32 = 6;
/// The first step of 1423, where a Human leaves Omega.
pub(in crate::cell::content::chain_replay_tests) const FIRST_STEP_1423: i32 = 4223;
/// The first step of 1500, where a Free Jaffa leaves Omega.
pub(in crate::cell::content::chain_replay_tests) const FIRST_STEP_1500: i32 = 4369;

const WORLD_BETA_NAME: &str = "Beta_Site_Evo_1";

/// 1655 done (its S4942 key still `active`, as the context holds it until
/// the next save), the offered mission `active` at its first step.
fn departed(archetype: i64, world: i32, offer: i32, first_step: i32) -> ExecutionContext {
    let ctx = with_mission(ctx_for(archetype, Some(world)), M_1655, "completed");
    let ctx = with_step(ctx, M_1655, S_4942, "active");
    let ctx = with_mission(ctx, offer, "active");
    with_step(ctx, offer, first_step, "active")
}

/// A Human who accepted Jackson's offer, standing in `world`.
///
/// # Panics
/// If `archetype` is outside 1-4: only the Human archetypes take this route.
pub(in crate::cell::content::chain_replay_tests) fn human_departure(
    archetype: i64,
    world: i32,
) -> ExecutionContext {
    assert!(
        (1..=4).contains(&archetype),
        "human_departure takes archetypes 1-4, got {archetype}"
    );
    departed(archetype, world, M_1423, FIRST_STEP_1423)
}

/// A Free Jaffa who accepted Bra'tac's offer, standing in `world`.
pub(in crate::cell::content::chain_replay_tests) fn jaffa_departure(
    world: i32,
) -> ExecutionContext {
    departed(ARCH_JAFFA, world, M_1500, FIRST_STEP_1500)
}

/// **The fixtures name the seeded first steps.** `load_mission_defs`
/// (`crates/cell-catalog/src/cell/spawner/missions.rs`) starts a mission at
/// the lowest `mission_steps.index`, so that is the step an `accept_mission`
/// leaves `active`.
#[tokio::test]
async fn live_db_the_first_steps_are_the_seeded_ones() {
    let pool = require_db_or_skip!();
    for (mission, expected) in [(M_1423, FIRST_STEP_1423), (M_1500, FIRST_STEP_1500)] {
        let first: i32 = sqlx::query_scalar(
            "SELECT step_id FROM resources.mission_steps WHERE mission_id = $1 \
             ORDER BY index ASC, step_id ASC LIMIT 1",
        )
        .bind(mission)
        .fetch_one(&pool)
        .await
        .expect("the mission must have a step");
        assert_eq!(first, expected, "first step of mission {mission}");
    }
}

/// The three transitions an Accept must resolve, in order, ignoring the
/// dialog-set bookkeeping around them.
fn transitions(resolved: &cimmeria_content_engine::chain::ResolvedActions) -> Vec<String> {
    labels(resolved)
        .into_iter()
        .filter(|l| !l.contains("dialog_set"))
        .collect()
}

/// **Jackson's Accept produces the Human departure.** From the UO-07 state
/// (1655 active at S4942, 1423 not held), the click on 4641 resolves
/// `complete_mission(1655)`, `accept_mission(1423)` and
/// `grant_stargate_address(6)`: the three transitions [`human_departure`]
/// encodes.
#[tokio::test]
async fn live_db_jacksons_accept_produces_the_human_departure() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let ctx = with_mission(ctx_for(ARCH_HUMAN, Some(WORLD_CMD)), M_1655, "active");
    let ctx = with_step(ctx, M_1655, S_4942, "active");
    let ctx = with_mission(ctx, M_1423, "not_active");
    assert_eq!(
        transitions(&fire_dialog_choice(&engine, &ctx, DLG_JACKSON)),
        vec![
            "63822:complete_mission(1655)".to_string(),
            "63822:accept_mission(1423)".to_string(),
            format!("63822:grant_stargate_address({DEPARTURE_GATE})"),
        ],
    );
}

/// **Bra'tac's Accept produces the Free Jaffa departure.** The same for
/// 5025 and 1500.
#[tokio::test]
async fn live_db_bratacs_accept_produces_the_jaffa_departure() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let ctx = with_mission(ctx_for(ARCH_JAFFA, Some(WORLD_CMD)), M_1655, "active");
    let ctx = with_step(ctx, M_1655, S_4942, "active");
    let ctx = with_mission(ctx, M_1500, "not_active");
    assert_eq!(
        transitions(&fire_dialog_choice(&engine, &ctx, DLG_BRATAC)),
        vec![
            "63832:complete_mission(1655)".to_string(),
            "63832:accept_mission(1500)".to_string(),
            format!("63832:grant_stargate_address({DEPARTURE_GATE})"),
        ],
    );
}

/// **A departed player resolves no Omega chain in Beta.** In world 23, for
/// each route: Beta's `player_loaded`, the ring pad beam, every dialog Omega
/// keys, and an abandon of a mission Omega does not offer (1655, which has
/// no Omega re-offer) read no Omega label and open no dialog.
#[tokio::test]
async fn live_db_a_departed_player_resolves_no_omega_chain_in_beta() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for (route, ctx) in [
        ("human", human_departure(1, WORLD_BETA)),
        ("jaffa", jaffa_departure(WORLD_BETA)),
    ] {
        let mut seen = labels(&fire_player_loaded(&engine, &ctx, WORLD_BETA_NAME));
        seen.extend(labels(&fire_teleport_in(&engine, &ctx, REGION_CMD_PAD)));
        seen.extend(labels(&fire_mission_abandoned(&engine, &ctx, M_1655)));
        for dialog in [DLG_JACKSON, DLG_BRATAC, DLG_MODI] {
            let resolved = fire_dialog_choice(&engine, &ctx, dialog);
            assert_eq!(
                display_dialog_count(&resolved),
                0,
                "{route}: dialog {dialog} must open nothing in Beta"
            );
            seen.extend(labels(&resolved));
        }
        assert!(seen.is_empty(), "{route} resolved Omega labels: {seen:?}");
    }
}
