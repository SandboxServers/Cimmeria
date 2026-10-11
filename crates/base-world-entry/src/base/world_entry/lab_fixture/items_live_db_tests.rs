//! Live-DB guards for a fixture's exact item counts (lab-fixtures FX-03).
//!
//! Sentinel block `0x7046_0300`; each test takes a `0x10` slot, account at
//! the slot base and player one above (`tests::seed_player`). Deleting the
//! player cascades to its inventory rows, so `tests::cleanup` clears them.

use sqlx::PgPool;

use cimmeria_base_session::base::world_entry::lab_fixture::StagedItem;

use super::items::{plan_items, write_items};
use super::tests::{
    active, apply, cleanup, fixture, mission_row, seed_mission, seed_player, steps_of, MISSION,
};
use crate::test_support::require_db_or_skip;

const REPLACES: i32 = 0x7046_0310;
const ZERO: i32 = 0x7046_0320;
const MISSION_BAG: i32 = 0x7046_0330;
const ALL_OR_NOTHING: i32 = 0x7046_0340;
const FULL_BACKPACK: i32 = 0x7046_0350;
const REFUSALS: i32 = 0x7046_0360;
const BANDOLIER: i32 = 0x7046_0370;
const BANDOLIER_KEPT: i32 = 0x7046_0380;

const MAIN: i32 = 1;
const MISSION_CONTAINER: i32 = 2;
const BANDOLIER_CONTAINER: i32 = 3;
const CHEST: i32 = 7;
const BUYBACK: i32 = 16;
const BANK: i32 = 17;
/// `bag_max_slots(INV_MAIN)`.
const MAIN_SLOTS: i32 = 40;

/// Two unstackable designs (weapons, armour) the backpack allows, by the
/// grant path's rule, from the seed, lowest ids first.
async fn unstackable(pool: &PgPool) -> (i32, i32) {
    let ids: Vec<i32> = sqlx::query_scalar(
        "SELECT item_id FROM resources.items WHERE max_stack_size = 1 \
         AND (container_sets IS NULL OR cardinality(container_sets) = 0 \
              OR 1 = ANY(container_sets)) \
         ORDER BY item_id LIMIT 2",
    )
    .fetch_all(pool)
    .await
    .expect("read unstackable designs");
    assert_eq!(
        ids.len(),
        2,
        "fixture sanity: the seed has unstackable items"
    );
    (ids[0], ids[1])
}

/// A mission-bag design that stacks to 2..=4, so a count of 5 needs at
/// least two rows.
async fn small_stack(pool: &PgPool) -> (i32, i32) {
    let row: Option<(i32, i32)> = sqlx::query_as(
        "SELECT item_id, max_stack_size FROM resources.items \
         WHERE max_stack_size BETWEEN 2 AND 4 AND 2 = ANY(container_sets) \
         ORDER BY item_id LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .expect("read stackable design");
    row.expect("fixture sanity: the seed has a mission design stacking to 2..=4")
}

/// A stackable design the backpack allows, distinct from [`unstackable`]'s.
async fn backpack_stackable(pool: &PgPool) -> i32 {
    first_design(
        pool,
        "SELECT item_id FROM resources.items \
         WHERE max_stack_size > 1 AND 1 = ANY(container_sets) ORDER BY item_id LIMIT 1",
        "a stackable backpack design",
    )
    .await
}

/// The first design a query names.
async fn first_design(pool: &PgPool, sql: &'static str, what: &str) -> i32 {
    let id: Option<i32> = sqlx::query_scalar(sql)
        .fetch_optional(pool)
        .await
        .expect("read design");
    id.unwrap_or_else(|| panic!("fixture sanity: the seed has {what}"))
}

/// One inventory row, the way any grant leaves it; returns its instance id.
async fn put(pool: &PgPool, player_id: i32, type_id: i32, container: i32, slot: i32) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id) \
         VALUES ($1, $2, 1, $3, $4) RETURNING item_id",
    )
    .bind(player_id)
    .bind(type_id)
    .bind(slot)
    .bind(container)
    .fetch_one(pool)
    .await
    .expect("insert inventory row")
}

/// `(container, slot, stack, instance)` of every row of `type_id`, in
/// container and slot order.
async fn rows_of(pool: &PgPool, player_id: i32, type_id: i32) -> Vec<(i32, i32, i32, i32)> {
    sqlx::query_as(
        "SELECT container_id, slot_id, stack_size, item_id FROM sgw_inventory \
         WHERE character_id = $1 AND type_id = $2 ORDER BY container_id, slot_id",
    )
    .bind(player_id)
    .bind(type_id)
    .fetch_all(pool)
    .await
    .expect("read inventory rows")
}

fn item(type_id: i32, count: u32, container_id: i32) -> StagedItem {
    StagedItem {
        type_id,
        count,
        container_id,
    }
}

/// **Guard: a count replaces every carried instance and nothing else.**
/// Two in the backpack, one in the bandolier, one in buyback (16, the first
/// container past the carried range) and one in the bank; `count = 1`
/// leaves exactly one carried instance, in the backpack, and the buyback
/// and bank rows as they were. Delete only one row or widen the container
/// range, even by one to 16, and an assertion fails.
#[tokio::test]
async fn live_db_fixture_item_count_replaces_every_carried_instance() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, REPLACES).await;
    let (design, _) = unstackable(&pool).await;
    put(&pool, player_id, design, MAIN, 0).await;
    put(&pool, player_id, design, MAIN, 1).await;
    put(&pool, player_id, design, BANDOLIER_CONTAINER, 0).await;
    let bought_back = put(&pool, player_id, design, BUYBACK, 0).await;
    let banked = put(&pool, player_id, design, BANK, 0).await;

    let mut f = fixture(player_id, Vec::new());
    f.items = vec![item(design, 1, MAIN)];
    let applied = apply(&pool, REPLACES, &f).await;
    let rows = rows_of(&pool, player_id, design).await;
    cleanup(&pool, REPLACES).await;

    let applied = applied.expect("applied");
    assert_eq!((applied.items_removed, applied.items_granted), (3, 1));
    let carried: Vec<_> = rows
        .iter()
        .filter(|r| r.0 != BANK && r.0 != BUYBACK)
        .collect();
    assert_eq!(carried.len(), 1, "one carried instance: {rows:?}");
    assert_eq!((carried[0].0, carried[0].2), (MAIN, 1));
    assert!(
        rows.contains(&(BUYBACK, 0, 1, bought_back)),
        "the buyback instance is untouched: {rows:?}"
    );
    assert!(
        rows.contains(&(BANK, 0, 1, banked)),
        "the bank instance is untouched: {rows:?}"
    );
}

/// **Guard: zero removes every carried instance**, the equipment slots and
/// the mission bag included, and still leaves the bank alone.
#[tokio::test]
async fn live_db_fixture_item_count_zero_removes_all_carried() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, ZERO).await;
    let (design, _) = unstackable(&pool).await;
    put(&pool, player_id, design, MAIN, 3).await;
    put(&pool, player_id, design, MISSION_CONTAINER, 0).await;
    put(&pool, player_id, design, CHEST, 0).await;
    let banked = put(&pool, player_id, design, BANK, 2).await;

    let mut f = fixture(player_id, Vec::new());
    f.items = vec![item(design, 0, MAIN)];
    let applied = apply(&pool, ZERO, &f).await;
    let rows = rows_of(&pool, player_id, design).await;
    cleanup(&pool, ZERO).await;

    let applied = applied.expect("applied");
    assert_eq!((applied.items_removed, applied.items_granted), (3, 0));
    assert_eq!(rows, vec![(BANK, 2, 1, banked)]);
}

/// **Guard: a count lands in the mission bag, split by the design's stack
/// size**: the stacks sum to the count, none is over the maximum, and none
/// is left in the backpack.
#[tokio::test]
async fn live_db_fixture_item_into_the_mission_inventory() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, MISSION_BAG).await;
    let (design, max_stack) = small_stack(&pool).await;
    put(&pool, player_id, design, MAIN, 0).await;

    let mut f = fixture(player_id, Vec::new());
    f.items = vec![item(design, 5, MISSION_CONTAINER)];
    let applied = apply(&pool, MISSION_BAG, &f).await;
    let rows = rows_of(&pool, player_id, design).await;
    cleanup(&pool, MISSION_BAG).await;

    let applied = applied.expect("applied");
    let stacks = 5_i32.div_euclid(max_stack) + i32::from(5 % max_stack != 0);
    assert_eq!(applied.items_granted, stacks as u64);
    assert!(rows.iter().all(|r| r.0 == MISSION_CONTAINER), "{rows:?}");
    assert!(rows.iter().all(|r| r.2 <= max_stack), "{rows:?}");
    assert_eq!(rows.iter().map(|r| r.2).sum::<i32>(), 5);
    assert_eq!(rows.len(), stacks as usize);
}

/// **Guard: the item write is one transaction.** The plan is made while the
/// backpack has room; then the backpack fills (as a concurrent write
/// would), so the grant fails after the deletes ran. The deletes roll back:
/// the old instances are still there and no new one is. Run the deletes
/// outside the transaction and the old rows are gone.
#[tokio::test]
async fn live_db_fixture_items_are_all_or_nothing() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, ALL_OR_NOTHING).await;
    let (old, filler) = unstackable(&pool).await;
    let granted = backpack_stackable(&pool).await;
    // Outside the backpack, so deleting them frees no backpack slot.
    let old_rows = vec![
        put(&pool, player_id, old, BANDOLIER_CONTAINER, 1).await,
        put(&pool, player_id, old, CHEST, 0).await,
    ];
    let plans = plan_items(
        &pool,
        player_id,
        &[item(old, 0, MAIN), item(granted, 1, MAIN)],
    )
    .await;
    for slot in 0..MAIN_SLOTS {
        put(&pool, player_id, filler, MAIN, slot).await;
    }
    let written = match &plans {
        Ok(plans) => Some(write_items(&pool, player_id, plans).await),
        Err(_) => None,
    };
    let old_after = rows_of(&pool, player_id, old).await;
    let granted_after = rows_of(&pool, player_id, granted).await;
    cleanup(&pool, ALL_OR_NOTHING).await;

    plans.expect("the plan is made while the backpack has room");
    let error = written
        .expect("written")
        .expect_err("a full backpack fails the grant");
    assert!(error.contains("no room"), "{error}");
    assert_eq!(
        old_after.iter().map(|r| r.3).collect::<Vec<_>>(),
        old_rows,
        "the deletes rolled back"
    );
    assert_eq!(granted_after, Vec::new());
}

/// **Guard: no room for the stacks refuses the whole fixture before any
/// write**: the precheck's dry run fails, so the fixture's mission is not
/// written and the old instances are still there. Drop the dry run and the
/// mission row changes.
#[tokio::test]
async fn live_db_fixture_full_backpack_writes_nothing() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, FULL_BACKPACK).await;
    seed_mission(&pool, player_id, MISSION, 2, 1).await;
    let steps = steps_of(&pool, MISSION).await;
    let mission_before = mission_row(&pool, player_id, MISSION).await;
    let (filler, _) = unstackable(&pool).await;
    let granted = backpack_stackable(&pool).await;
    for slot in 0..MAIN_SLOTS {
        put(&pool, player_id, filler, MAIN, slot).await;
    }

    let mut f = fixture(player_id, vec![active(MISSION, steps[1])]);
    f.items = vec![item(granted, 1, MAIN)];
    let applied = apply(&pool, FULL_BACKPACK, &f).await;
    let mission_after = mission_row(&pool, player_id, MISSION).await;
    let filler_after = rows_of(&pool, player_id, filler).await.len();
    let granted_after = rows_of(&pool, player_id, granted).await;
    cleanup(&pool, FULL_BACKPACK).await;

    let error = applied.expect_err("no room refuses the fixture");
    assert!(error.contains("no room"), "{error}");
    assert!(mission_before.is_some());
    assert_eq!(mission_after, mission_before, "the mission is not written");
    assert_eq!(filler_after, MAIN_SLOTS as usize);
    assert_eq!(granted_after, Vec::new());
}

/// **Guard: every item check runs in the precheck.** Each bad list (an
/// unknown design, a count over 10, a container that is not main or
/// mission, a design listed twice, a container the design's
/// `container_sets` excludes, a design with no `container_sets` asked into
/// the mission bag) refuses the fixture with the reason, and neither the
/// fixture's mission nor the carried instance changes. Each case has its
/// own character (accounts `REFUSALS + 2 * i`), and every failing case is
/// reported, so one dropped check shows on its own.
#[tokio::test]
async fn live_db_fixture_item_refusals_write_nothing() {
    let pool = require_db_or_skip!();
    let steps = steps_of(&pool, MISSION).await;
    let (design, _) = unstackable(&pool).await;
    // The grant path's container rule (`item_allows_container`): a listed
    // set must name the container, and an empty one allows only the
    // backpack.
    let not_in_backpack = first_design(
        &pool,
        "SELECT item_id FROM resources.items \
         WHERE cardinality(container_sets) > 0 AND NOT 1 = ANY(container_sets) \
         ORDER BY item_id LIMIT 1",
        "a design whose container_sets exclude the backpack",
    )
    .await;
    let no_sets = first_design(
        &pool,
        "SELECT item_id FROM resources.items \
         WHERE container_sets IS NULL OR cardinality(container_sets) = 0 \
         ORDER BY item_id LIMIT 1",
        "a design with no container_sets",
    )
    .await;

    let cases: [(&str, Vec<StagedItem>, &str); 6] = [
        (
            "unknown design",
            vec![item(design, 0, MAIN), item(i32::MAX, 1, MAIN)],
            "not in resources.items",
        ),
        (
            "count",
            vec![item(design, 11, MAIN)],
            "over the fixture limit",
        ),
        (
            "container",
            vec![item(design, 1, BANDOLIER_CONTAINER)],
            "is not main",
        ),
        (
            "duplicate",
            vec![item(design, 0, MAIN), item(design, 1, MAIN)],
            "listed twice",
        ),
        (
            "container_sets exclude the container",
            vec![item(design, 0, MAIN), item(not_in_backpack, 1, MAIN)],
            "does not allow container 1",
        ),
        (
            "no container_sets, mission bag",
            vec![item(design, 0, MAIN), item(no_sets, 1, MISSION_CONTAINER)],
            "does not allow container 2",
        ),
    ];
    let mut failures = Vec::new();
    for (i, (label, items, want)) in cases.iter().enumerate() {
        let account_id = REFUSALS + 2 * i as i32;
        let player_id = seed_player(&pool, account_id).await;
        seed_mission(&pool, player_id, MISSION, 2, 1).await;
        let carried = put(&pool, player_id, design, MAIN, 0).await;
        let mission_before = mission_row(&pool, player_id, MISSION).await;

        let mut f = fixture(player_id, vec![active(MISSION, steps[1])]);
        f.items = items.clone();
        let applied = apply(&pool, account_id, &f).await;
        let mission = mission_row(&pool, player_id, MISSION).await;
        let rows = rows_of(&pool, player_id, design).await;
        cleanup(&pool, account_id).await;

        assert!(mission_before.is_some(), "{label}: fixture sanity");
        match applied {
            Ok(a) => failures.push(format!("{label}: applied {a:?}")),
            Err(e) if !e.contains(want) => failures.push(format!("{label}: wrong reason {e}")),
            Err(_) => {}
        }
        if mission != mission_before {
            failures.push(format!("{label}: the mission was written: {mission:?}"));
        }
        if rows != vec![(MAIN, 0, 1, carried)] {
            failures.push(format!("{label}: the items changed: {rows:?}"));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

/// **Guard: removing the active weapon leaves no dangling bandolier
/// pointer.** `bandolier_slot` points at the removed weapon's slot; another
/// weapon stays in the bandolier. The next `onClientReady` reads the slot
/// and the bandolier rows, so the slot must move to the remaining weapon
/// (the lowest occupied slot, as `sync_bandolier_after_inventory_change`
/// picks it). Drop the reconcile and the slot stays on the empty one.
#[tokio::test]
async fn live_db_fixture_removing_the_active_weapon_moves_the_bandolier_slot() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, BANDOLIER).await;
    let (removed, kept) = unstackable(&pool).await;
    put(&pool, player_id, removed, BANDOLIER_CONTAINER, 1).await;
    put(&pool, player_id, kept, BANDOLIER_CONTAINER, 2).await;
    sqlx::query("UPDATE sgw_player SET bandolier_slot = 1 WHERE player_id = $1")
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("set the active slot");

    let mut f = fixture(player_id, Vec::new());
    f.items = vec![item(removed, 0, MAIN)];
    let applied = apply(&pool, BANDOLIER, &f).await;
    let slot: i32 =
        sqlx::query_scalar("SELECT bandolier_slot FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(&pool)
            .await
            .expect("read the active slot");
    cleanup(&pool, BANDOLIER).await;

    assert_eq!(applied.expect("applied").items_removed, 1);
    assert_eq!(slot, 2, "the active slot moves to the remaining weapon");
}

/// **Guard: a fixture that removes nothing from the bandolier leaves
/// `bandolier_slot` alone.** The active slot is 0 and empty; the only
/// weapon sits in bandolier slot 1, and the fixture lists an unrelated
/// backpack design. Nothing left the bandolier, so the slot stays 0 and
/// the weapon stays put. Reconcile unconditionally and the slot moves to 1.
#[tokio::test]
async fn live_db_fixture_items_leave_the_bandolier_slot_when_the_bandolier_is_untouched() {
    let pool = require_db_or_skip!();
    let player_id = seed_player(&pool, BANDOLIER_KEPT).await;
    let (weapon, unrelated) = unstackable(&pool).await;
    let readied = put(&pool, player_id, weapon, BANDOLIER_CONTAINER, 1).await;
    sqlx::query("UPDATE sgw_player SET bandolier_slot = 0 WHERE player_id = $1")
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("set the active slot");

    let mut f = fixture(player_id, Vec::new());
    f.items = vec![item(unrelated, 1, MAIN)];
    let applied = apply(&pool, BANDOLIER_KEPT, &f).await;
    let slot: i32 =
        sqlx::query_scalar("SELECT bandolier_slot FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .fetch_one(&pool)
            .await
            .expect("read the active slot");
    let weapon_rows = rows_of(&pool, player_id, weapon).await;
    cleanup(&pool, BANDOLIER_KEPT).await;

    let applied = applied.expect("applied");
    assert_eq!((applied.items_removed, applied.items_granted), (0, 1));
    assert_eq!(slot, 0, "the active slot is not the fixture's to move");
    assert_eq!(weapon_rows, vec![(BANDOLIER_CONTAINER, 1, 1, readied)]);
}
