//! Live-DB guard that the Cellblock → Castle world hop cannot lose the
//! player's existing mission state (TESTING.md type 3).
//!
//! Mission 701's arrival chain (1201) fires on `player_loaded Castle`
//! after chain 1109 cross-world teleports the player out of the
//! Cellblock. Two missions must survive that hop: 1360 (Frost's letter,
//! still active) and 688 (Secure the Armory, just completed). If either
//! were dropped, the player would arrive in Castle with a corrupted quest
//! log and no way to notice until much later.
//!
//! **Why this shape and not an end-to-end hop.** The real hop is
//! `Action::CrossWorldTeleport` → `CellToBaseMsg::GateTravel`, whose
//! cell-side half destroys the entity (`cell/gate_travel.rs`,
//! `executor/transport.rs`) and hands off to the base service, which
//! re-creates it in the destination world from persisted rows. Asserting
//! across that needs a fixture driving both services over a world
//! boundary, and none exists in `crates/services` — everything today is
//! either cell-only (a `SpaceManager` plus a channel) or DB-only.
//! Building one is test-infra work, not content work.
//!
//! So this guards the actual invariant the hop depends on instead: the
//! persisted mission rows are keyed by player and mission alone, and the
//! restore query filters on nothing else, so changing the player's world
//! cannot change what comes back. That is checkable today against the
//! real production query. The end-to-end behaviour stays a UAT step (M1:
//! "arrive near Gerschon, letter 1360 still active"), recorded in
//! `docs/analysis/castle-rebuild/worknotes/m701.md`.
//!
//! Placed in this module rather than beside `query_saved_missions` in
//! `base/world_entry/methods/missions.rs` because that file is at 691
//! lines, one test short of the 700-line hard cap, and is shared base-side
//! surface that several concurrent packets touch.
//!
//! Sentinel range `0x7000_6000` — steps past every sibling reservation in
//! `crates/services` (`0x7000_0100`-`0x7000_0400`,
//! `0x7000_1000`-`0x7000_1B00`, `0x7000_2000`, `0x7000_3000`,
//! `0x7000_4000`, `0x7000_4242`, `0x7000_5000`). Cleanup deletes the exact
//! ids inserted, never a range.

use std::sync::Arc;

use sqlx::PgPool;

use crate::base::world_entry::methods::query_saved_missions;
use crate::test_support::require_db_or_skip;

const TEST_ACCOUNT: i32 = 0x7000_6000;
const TEST_PLAYER: i32 = 0x7000_6001;

/// Frost's letter — accepted in the Cellblock, still active on arrival.
const MISSION_LETTER: i32 = 1360;
/// Secure the Armory — completed by chain 1109 on the way out.
const MISSION_ARMORY: i32 = 688;

const STATUS_ACTIVE: i32 = 1;
const STATUS_COMPLETED: i32 = 2;

async fn cleanup(pool: &PgPool) {
    let _ = sqlx::query("DELETE FROM sgw_mission WHERE player_id = $1")
        .bind(TEST_PLAYER)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
        .bind(TEST_ACCOUNT)
        .execute(pool)
        .await;
}

/// Insert the account + player, parked in the Cellblock.
async fn seed_player_in_cellblock(pool: &PgPool) {
    sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
        .bind(TEST_ACCOUNT)
        .bind(format!("m701-hop-{TEST_ACCOUNT}"))
        .execute(pool)
        .await
        .expect("insert account");

    sqlx::query(
        "INSERT INTO sgw_player (\
            account_id, player_id, level, alignment, archetype, gender, \
            player_name, extra_name, world_location, bodyset, \
            pos_x, pos_y, pos_z, skin_color_id, naquadah\
         ) VALUES ($1, $2, 3, 0, 1, 1, $3, '', 'Castle_CellBlock', \
                   'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0, 0)",
    )
    .bind(TEST_ACCOUNT)
    .bind(TEST_PLAYER)
    .bind(format!("m701hop{TEST_PLAYER}"))
    .execute(pool)
    .await
    .expect("insert player");
}

async fn seed_mission(pool: &PgPool, mission_id: i32, status: i32, current_step: Option<i32>) {
    sqlx::query(
        "INSERT INTO sgw_mission (\
            player_id, mission_id, status, current_step_id, completed_step_ids, \
            completed_objective_ids, active_objective_ids, failed_objective_ids, repeats\
         ) VALUES ($1, $2, $3, $4, '{}', '{}', '{}', '{}', 0)",
    )
    .bind(TEST_PLAYER)
    .bind(mission_id)
    .bind(status)
    .bind(current_step)
    .execute(pool)
    .await
    .expect("insert sgw_mission row");
}

/// Changing the player's `world_location` from `Castle_CellBlock` to
/// `Castle` — the only persisted effect the hop has on the player row —
/// must not change what `query_saved_missions` restores.
///
/// The assertion is on the production restore path, not on a hand-written
/// query: `query_saved_missions` is what the base service calls to rebuild
/// `MissionManager` on world entry. If someone ever added a world or space
/// filter to its `WHERE` clause, this fails.
#[tokio::test]
async fn missions_survive_the_cellblock_to_castle_world_change() {
    let pool = require_db_or_skip!();

    // A previous panicking run may have leaked rows.
    cleanup(&pool).await;
    seed_player_in_cellblock(&pool).await;
    seed_mission(&pool, MISSION_LETTER, STATUS_ACTIVE, Some(2344)).await;
    seed_mission(&pool, MISSION_ARMORY, STATUS_COMPLETED, None).await;

    let db = Some(Arc::new(pool.clone()));

    let before = query_saved_missions(&db, TEST_PLAYER).await;
    let after_move = {
        sqlx::query("UPDATE sgw_player SET world_location = 'Castle' WHERE player_id = $1")
            .bind(TEST_PLAYER)
            .execute(&pool)
            .await
            .expect("world_location update must succeed");
        query_saved_missions(&db, TEST_PLAYER).await
    };

    // Drop the sentinel rows before asserting so a failure can't leave a
    // stray player in the shared test database.
    cleanup(&pool).await;

    let summarize = |missions: &[crate::cell::messages::SavedMission]| {
        let mut v: Vec<(i32, i8, Option<i32>)> = missions
            .iter()
            .map(|m| (m.mission_id, m.status, m.current_step_id))
            .collect();
        v.sort();
        v
    };

    assert_eq!(
        summarize(&before),
        vec![
            (MISSION_ARMORY, STATUS_COMPLETED as i8, None),
            (MISSION_LETTER, STATUS_ACTIVE as i8, Some(2344)),
        ],
        "fixture sanity: both missions must load while the player is still \
         recorded in the Cellblock",
    );
    assert_eq!(
        summarize(&after_move),
        summarize(&before),
        "changing the player's world_location to Castle must not change the \
         restored mission set — sgw_mission is keyed by (player_id, \
         mission_id) with no world column, and query_saved_missions filters \
         on player_id alone. A world or space filter added to that query \
         would strand mission 1360 and 688 on the Cellblock side of the hop.",
    );
}

/// Companion schema guard, cheap and blunt: `sgw_mission` must carry no
/// world-scoping column at all.
///
/// The test above proves today's query ignores the world. This one proves
/// the *table* offers nothing to scope by, so a future "just filter by
/// world" change has to add a column and will trip here first. Named
/// columns rather than a whole-schema snapshot so ordinary column
/// additions don't churn it.
#[tokio::test]
async fn sgw_mission_has_no_world_scoping_column() {
    let pool = require_db_or_skip!();

    let cols: Vec<String> = sqlx::query_scalar(
        "SELECT column_name::text FROM information_schema.columns \
         WHERE table_name = 'sgw_mission'",
    )
    .fetch_all(&pool)
    .await
    .expect("information_schema query must succeed");

    assert!(
        !cols.is_empty(),
        "sgw_mission must exist — an empty column list means the schema \
         wasn't loaded, not that the guard passed",
    );

    let scoping: Vec<&String> = cols
        .iter()
        .filter(|c| {
            let c = c.to_ascii_lowercase();
            c.contains("world") || c.contains("space") || c.contains("zone")
        })
        .collect();
    assert!(
        scoping.is_empty(),
        "sgw_mission gained a world-scoping column {scoping:?} — mission \
         state must stay world-independent or a cross-world hop can drop it",
    );
}
