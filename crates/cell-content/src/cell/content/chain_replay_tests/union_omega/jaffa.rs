//! UO-08: Bra'tac's induction (63830-63832).
//!
//! Resolve-only for the three chains. The gates are archetype 7 (Free Jaffa,
//! not archetype 8), the Command Center world, the 1655 step, and 1500 not
//! yet active. Each bind runs `remove_dialog_set` before `add_dialog_set`, and
//! the Accept completes 1655 (F21: no objective is completed earlier).

use cimmeria_content_engine::context::ExecutionContext;

use super::{
    ctx_for, display_dialog_count, fire_dialog_choice, fire_player_loaded, fire_teleport_in,
    labels, with_mission, with_step, ARCH_ASGARD, ARCH_HUMAN, ARCH_JAFFA, CMD_NAME, DLG_BRATAC,
    DLG_JACKSON, M_1423, M_1500, M_1655, REGION_CMD_PAD, S_4940, S_4942, WORLD_CMD, WORLD_HUB,
};
use crate::cell::content::build_engine;
use crate::test_support::require_db_or_skip;

/// Bra'tac's chains, 63830-63832. Other Omega chains are filtered out.
const BRATAC_CHAINS: std::ops::RangeInclusive<i64> = 63830..=63832;

/// A Free Jaffa in the Command Center at `step` on 1655, with 1500 not yet
/// active, so Bra'tac's offer is open.
fn at(archetype: i64, world: i32, step: i32) -> ExecutionContext {
    let ctx = with_mission(ctx_for(archetype, Some(world)), M_1655, "active");
    let ctx = with_step(ctx, M_1655, step, "active");
    with_mission(ctx, M_1500, "not_active")
}

/// The rendered labels from one chain id, so a test reads one chain at a time.
fn only(chain: i64, all: Vec<String>) -> Vec<String> {
    let prefix = format!("{chain}:");
    all.into_iter().filter(|l| l.starts_with(&prefix)).collect()
}

/// Keep only the labels from Bra'tac's three chains.
fn bratac(all: Vec<String>) -> Vec<String> {
    all.into_iter()
        .filter(|l| {
            l.split(':')
                .next()
                .and_then(|chain| chain.parse::<i64>().ok())
                .is_some_and(|chain| BRATAC_CHAINS.contains(&chain))
        })
        .collect()
}

/// **The Command Center beam at S4940 binds Bra'tac, once per fire.** Chain
/// 63830 removes and then adds his bind (map 63801, slot 63801). Past S4940
/// the beam resolves nothing Bra'tac owns.
#[tokio::test]
async fn live_db_a_jaffa_beam_binds_bratac_once() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let ctx = at(ARCH_JAFFA, WORLD_CMD, S_4940);
    assert_eq!(
        only(
            63830,
            labels(&fire_teleport_in(&engine, &ctx, REGION_CMD_PAD))
        ),
        vec![
            "63830:remove_dialog_set(63801@63801)".to_string(),
            "63830:add_dialog_set(63801@63801)".to_string(),
        ],
        "the Command Center beam binds Bra'tac once"
    );
    let past = at(ARCH_JAFFA, WORLD_CMD, S_4942);
    assert!(
        only(
            63830,
            labels(&fire_teleport_in(&engine, &past, REGION_CMD_PAD))
        )
        .is_empty(),
        "past S4940 the beam binds nothing"
    );
}

/// **A Command Center load at S4942 rebinds Bra'tac.** Chain 63831 is the
/// same remove-then-add pair, for a relog or a second beam. At S4940 it does
/// nothing.
#[tokio::test]
async fn live_db_a_relog_at_4942_rebinds_bratac() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let ctx = at(ARCH_JAFFA, WORLD_CMD, S_4942);
    assert_eq!(
        labels(&fire_player_loaded(&engine, &ctx, CMD_NAME)),
        vec![
            "63831:remove_dialog_set(63801@63801)".to_string(),
            "63831:add_dialog_set(63801@63801)".to_string(),
        ],
        "a Command Center load at S4942 rebinds Bra'tac"
    );
    let early = at(ARCH_JAFFA, WORLD_CMD, S_4940);
    assert!(
        only(
            63831,
            labels(&fire_player_loaded(&engine, &early, CMD_NAME))
        )
        .is_empty(),
        "a Command Center load at S4940 does not rebind"
    );
}

/// **Bra'tac's Accept completes 1655, offers The SGU and grants gate 6.** The
/// chain removes his bind first, then completes 1655 and accepts 1500 (The
/// SGU). Dialog 5025 opens no display dialog on the Accept.
#[tokio::test]
async fn live_db_bratacs_accept_completes_1655_and_offers_the_sgu() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    let ctx = at(ARCH_JAFFA, WORLD_CMD, S_4942);
    let resolved = fire_dialog_choice(&engine, &ctx, DLG_BRATAC);
    assert_eq!(
        labels(&resolved),
        vec![
            "63832:remove_dialog_set(63801@63801)".to_string(),
            "63832:complete_mission(1655)".to_string(),
            "63832:accept_mission(1500)".to_string(),
            "63832:grant_stargate_address(6)".to_string(),
        ],
        "Bra'tac's Accept runs the four actions in order"
    );
    assert_eq!(
        display_dialog_count(&resolved),
        0,
        "the Accept opens no display dialog"
    );
}

/// **No other faction gets Bra'tac's offer.** Archetype 8 (Jaffa) is the case
/// a `gte` gate would let through; the rest are the Human range, Asgard and
/// unknown. Each event is fired in the state where the offer is open.
#[tokio::test]
async fn live_db_no_other_faction_gets_bratacs_offer() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for archetype in [ARCH_HUMAN, 4, ARCH_ASGARD, 6, 8, -1] {
        let mut seen = bratac(labels(&fire_teleport_in(
            &engine,
            &at(archetype, WORLD_CMD, S_4940),
            REGION_CMD_PAD,
        )));
        seen.extend(bratac(labels(&fire_player_loaded(
            &engine,
            &at(archetype, WORLD_CMD, S_4942),
            CMD_NAME,
        ))));
        seen.extend(bratac(labels(&fire_dialog_choice(
            &engine,
            &at(archetype, WORLD_CMD, S_4942),
            DLG_BRATAC,
        ))));
        assert!(
            seen.is_empty(),
            "archetype {archetype} resolved Bra'tac's chains: {seen:?}"
        );
    }
}

/// **Bra'tac's chains ignore the wrong state.** Each case clears one gate and
/// checks that no chain of the three fires for it: the wrong world, the wrong
/// step for the Accept, 1655 already completed, and 1500 already active.
#[tokio::test]
async fn live_db_bratac_ignores_the_wrong_state() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;

    // The Accept at the hub (world 18) is not the Command Center's.
    let hub = at(ARCH_JAFFA, WORLD_HUB, S_4942);
    assert!(
        bratac(labels(&fire_dialog_choice(&engine, &hub, DLG_BRATAC))).is_empty(),
        "the Accept at the hub resolves nothing"
    );

    // The Accept at S4940 is too early: the step is still the beam's.
    let early = at(ARCH_JAFFA, WORLD_CMD, S_4940);
    assert!(
        bratac(labels(&fire_dialog_choice(&engine, &early, DLG_BRATAC))).is_empty(),
        "the Accept at S4940 resolves nothing"
    );

    // 1655 completed with S4942 still active: the Accept has already run.
    let done = with_mission(at(ARCH_JAFFA, WORLD_CMD, S_4942), M_1655, "completed");
    assert!(
        bratac(labels(&fire_dialog_choice(&engine, &done, DLG_BRATAC))).is_empty(),
        "a completed 1655 resolves no Accept"
    );
    assert!(
        bratac(labels(&fire_player_loaded(&engine, &done, CMD_NAME))).is_empty(),
        "a completed 1655 resolves no Command Center rebind"
    );

    // 1500 already active: neither bind nor Accept fires again.
    let offered = with_mission(at(ARCH_JAFFA, WORLD_CMD, S_4940), M_1500, "active");
    assert!(
        bratac(labels(&fire_teleport_in(&engine, &offered, REGION_CMD_PAD))).is_empty(),
        "1500 already active: no beam bind"
    );
    let offered = with_mission(at(ARCH_JAFFA, WORLD_CMD, S_4942), M_1500, "active");
    assert!(
        bratac(labels(&fire_player_loaded(&engine, &offered, CMD_NAME))).is_empty(),
        "1500 already active: no Command Center rebind"
    );
    assert!(
        bratac(labels(&fire_dialog_choice(&engine, &offered, DLG_BRATAC))).is_empty(),
        "1500 already active: no Accept"
    );
}

/// **Each faction resolves only its own induction.** Bra'tac's dialog for a
/// Human and Jackson's dialog for a Free Jaffa each resolve nothing, with both
/// offers open. This guards against a faction receiving the other's The SGU.
#[tokio::test]
async fn live_db_each_faction_resolves_only_its_own_induction() {
    let pool = require_db_or_skip!();
    let engine = build_engine(Some(&pool)).await;
    for (archetype, dialog) in [(ARCH_HUMAN, DLG_BRATAC), (ARCH_JAFFA, DLG_JACKSON)] {
        let ctx = with_mission(at(archetype, WORLD_CMD, S_4942), M_1423, "not_active");
        let resolved = labels(&fire_dialog_choice(&engine, &ctx, dialog));
        assert!(
            resolved.is_empty(),
            "archetype {archetype} on dialog {dialog} resolved {resolved:?}"
        );
    }
}
