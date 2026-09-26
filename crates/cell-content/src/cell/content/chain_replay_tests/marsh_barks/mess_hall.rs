//! Chain 1177 — the long-table flank cue on entering the Mess Hall.
//!
//! The region is `Castle_Cellblock.Region3`, not Region9: Region9 is the
//! corridor at the topside ring pad, Region3 is the box that contains
//! both `MessHall_Guard` spawns, and the original `Castle_CellBlock.py`
//! fires the "Level 7: Mess Hall" discovery splash on entering Region3.
//! The seed comment above chain 1177 carries the full evidence, including
//! why `docs/content/mission-chains.md` disagrees.

use super::{
    assert_refused, assert_single_bark, engine_with, load, resolve_region_enter, CHAIN_MESS_HALL,
    REGION_MESS_HALL, SCREEN_MESS_HALL,
};
use crate::test_support::require_db_or_skip;

/// Happy path: walking into the Mess Hall with 681 accepted and the room
/// not yet cleared speaks the long-table flank cue.
#[tokio::test]
async fn chain_1177_mess_hall_entry_barks_the_long_table_flank_cue() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MESS_HALL).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_MESS_HALL,
        &[
            ("mission_681_status", "active"),
            ("mission_682_status", "not_active"),
        ],
    );
    assert_single_bark(&resolved, CHAIN_MESS_HALL as i64, SCREEN_MESS_HALL);
}

/// Adjacent wrong state — phase not yet reached. Mission 681 is accepted
/// by chain 1073 on the Region9 crossing, which is on the way in; a
/// player who somehow reaches Region3 before that must not hear a cue
/// about an objective they do not have.
#[tokio::test]
async fn chain_1177_does_not_fire_before_mission_681_is_accepted() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MESS_HALL).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_MESS_HALL,
        &[
            ("mission_680_status", "active"),
            ("mission_681_status", "not_active"),
            ("mission_682_status", "not_active"),
        ],
    );
    assert_refused(
        &resolved,
        CHAIN_MESS_HALL as i64,
        "mission 681 has not been accepted yet",
    );
}

/// Adjacent wrong state — already fired, phase passed. Chain 1087
/// completes 681 and accepts 682 on the mess-hall kill counter, closing
/// both arms of this gate. Re-entering the cleared room must be silent.
///
/// This is also the state the H52 step-activation replay re-evaluates the
/// chain against: `accept_mission 682` re-fires `enter_region` for every
/// region the player is standing in, which includes Region3. If this gate
/// did not close, the player would hear the line twice on one crossing.
#[tokio::test]
async fn chain_1177_does_not_re_bark_once_the_mess_hall_is_cleared() {
    let pool = require_db_or_skip!();
    let engine = engine_with(load(&pool, CHAIN_MESS_HALL).await);

    let resolved = resolve_region_enter(
        &engine,
        REGION_MESS_HALL,
        &[
            ("mission_681_status", "completed"),
            ("mission_682_status", "active"),
        ],
    );
    assert_refused(
        &resolved,
        CHAIN_MESS_HALL as i64,
        "the Mess Hall guards are dead — 681 is completed and 682 accepted \
         (this is also the post-mutation state the H52 region replay \
         re-evaluates the chain against on the very same crossing)",
    );
}
