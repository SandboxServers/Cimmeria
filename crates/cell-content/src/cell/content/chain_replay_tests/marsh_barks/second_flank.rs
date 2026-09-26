//! Chain 1178 — the second flank cue on entering Hallway05.
//!
//! Screen 96354 is screen 96352 with the table taken out of it: the same
//! play, phrased for the zone's only other flanking encounter. The region
//! is `Castle_Cellblock.Region5`, which contains both `Hallway05_Guard`
//! spawns and is the same region chain 1083 binds to accept mission 686.
//!
//! The gate here is a deliberate byte-for-byte copy of chain 1083's, and
//! [`chain_1178_is_gated_identically_to_the_mission_accept_it_rides`] is
//! what stops the two drifting apart: if 1083 is ever loosened, this bark
//! starts repeating on every Region5 re-entry.

use cimmeria_content_engine::actions::Action;
use cimmeria_content_engine::chain::ChainEngine;

use super::{
    assert_refused, assert_single_bark, bark_screens, engine_with, load, resolve_region_enter,
    CHAIN_HALLWAY05, REGION_HALLWAY05, SCREEN_HALLWAY05,
};
use crate::test_support::require_db_or_skip;

/// The mission-accept chain 1178 rides, and must stay co-gated with.
const CHAIN_ACCEPT_686: i64 = 1083;

/// Happy path: crossing into Hallway05 with 685 cleared and 686 not yet
/// accepted speaks the second flank cue.
#[tokio::test]
async fn chain_1178_hallway05_entry_barks_the_second_flank_cue() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_HALLWAY05).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_HALLWAY05,
        &[
            ("mission_685_status", "completed"),
            ("mission_686_status", "not_active"),
        ],
    );
    assert_single_bark(&resolved, CHAIN_HALLWAY05 as i64, SCREEN_HALLWAY05);
}

/// Adjacent wrong state — phase not yet reached. Hallway04 (mission 685)
/// is still being fought.
#[tokio::test]
async fn chain_1178_does_not_fire_before_hallway04_is_cleared() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_HALLWAY05).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_HALLWAY05,
        &[
            ("mission_685_status", "active"),
            ("mission_686_status", "not_active"),
        ],
    );
    assert_refused(
        &resolved,
        CHAIN_HALLWAY05 as i64,
        "mission 685 (Hallway04) is not yet complete",
    );
}

/// Adjacent wrong state — already fired. Chain 1083 accepts 686 on this
/// same crossing, so every later crossing, and the H52 replay that
/// `accept_mission 686` kicks off, must find the gate shut.
#[tokio::test]
async fn chain_1178_does_not_re_bark_once_686_is_accepted() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_HALLWAY05).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_HALLWAY05,
        &[
            ("mission_685_status", "completed"),
            ("mission_686_status", "active"),
        ],
    );
    assert_refused(
        &resolved,
        CHAIN_HALLWAY05 as i64,
        "mission 686 was accepted by chain 1083 on this same crossing",
    );
}

/// The bark chain that shares a region key with a mission chain must
/// share its gate exactly. If chain 1083 is ever loosened, 1178 starts
/// repeating — the seed says so in a comment on both chains; this says so
/// in a test.
#[tokio::test]
async fn chain_1178_is_gated_identically_to_the_mission_accept_it_rides() {
    let pool = require_db_or_skip!();
    let engine = {
        let mut e = ChainEngine::new();
        e.register_chain(load(&pool, CHAIN_ACCEPT_686 as i32).await);
        e.register_chain(load(&pool, CHAIN_HALLWAY05).await);
        e
    };

    // Every context in which 1083 accepts 686 must also bark, and vice
    // versa. Walk the three states that matter.
    for (params, both_fire, what) in [
        (
            vec![
                ("mission_685_status", "completed"),
                ("mission_686_status", "not_active"),
            ],
            true,
            "Hallway04 cleared, 686 not yet accepted",
        ),
        (
            vec![
                ("mission_685_status", "active"),
                ("mission_686_status", "not_active"),
            ],
            false,
            "Hallway04 still contested",
        ),
        (
            vec![
                ("mission_685_status", "completed"),
                ("mission_686_status", "active"),
            ],
            false,
            "686 already accepted",
        ),
    ] {
        let resolved = resolve_region_enter(&engine, REGION_HALLWAY05, &params);
        let accepts = resolved
            .actions
            .iter()
            .filter(|(id, action)| {
                *id == CHAIN_ACCEPT_686
                    && matches!(action, Action::AcceptMission { mission_id: 686 })
            })
            .count();
        let barks = bark_screens(&resolved, CHAIN_HALLWAY05 as i64).len();
        assert_eq!(
            (accepts > 0, barks > 0),
            (both_fire, both_fire),
            "with {what}: chain 1083 accept and chain 1178 bark must agree \
             (accepts={accepts}, barks={barks}). They are co-gated by \
             duplicated conditions, so a divergence here means somebody \
             edited one gate and not the other"
        );
    }
}
