//! Tests for `normalize_item_ids` (pure) and live-DB grant flows.

use super::*;
use crate::test_support::TestTransport;

mod normalize_item_ids_tests {
    use super::normalize_item_ids;

    #[test]
    fn drops_non_positive_ids() {
        assert_eq!(normalize_item_ids(vec![-3, 0, 1, 2]), vec![1, 2]);
    }

    #[test]
    fn dedupes_repeated_ids() {
        assert_eq!(normalize_item_ids(vec![5, 5, 5]), vec![5]);
    }

    #[test]
    fn sorts_ascending() {
        assert_eq!(normalize_item_ids(vec![3, 1, 2]), vec![1, 2, 3]);
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(normalize_item_ids(Vec::new()).is_empty());
    }

    #[test]
    fn all_invalid_returns_empty() {
        assert!(normalize_item_ids(vec![-1, 0, -100]).is_empty());
    }

    #[test]
    fn combined_dedupe_sort_filter() {
        // Mixed bag: i32::MIN, zeros, dupes, out-of-order positives. Result
        // must be sorted, deduped, and contain only the positive ids exactly
        // once each.
        let out = normalize_item_ids(vec![10, -1, 5, 10, 0, 5, i32::MIN, 7, 5]);
        assert_eq!(out, vec![5, 7, 10]);
    }
}

mod handle_grant_item_tests {
    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for grant-item tests. Distinct from move_inventory
    /// (0x7000_0200) and grant_cash (0x7000_0100).
    const TEST_BASE: i32 = 0x7000_0300;

    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32, entity_id: u32) {
        let _ = sqlx::query("DELETE FROM cell_event_outbox WHERE entity_id = $1")
            .bind(entity_id as i32)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32) {
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id)
        .bind(format!("grant-item-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");

        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("test-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }

    async fn pick_main_bag_type_id(pool: &PgPool) -> i32 {
        sqlx::query_scalar::<_, i32>(
            "SELECT item_id FROM resources.items \
             WHERE container_sets IS NULL OR 1 = ANY(container_sets) \
             ORDER BY item_id LIMIT 1",
        )
        .fetch_one(pool)
        .await
        .expect("pick item_id")
    }

    fn make_state(
        entity_id: u32,
    ) -> (
        Arc<dyn Transport>,
        Arc<Mutex<HashMap<u32, SocketAddr>>>,
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    ) {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let fake_addr: SocketAddr = "127.0.0.1:65535".parse().unwrap();
        let entity_to_addr = Arc::new(Mutex::new({
            let mut m = HashMap::new();
            m.insert(entity_id, fake_addr);
            m
        }));
        let connected = Arc::new(Mutex::new(HashMap::new()));
        (transport, entity_to_addr, connected)
    }

    /// Happy path: a grant inserts exactly one row at the lowest free slot
    /// in the target container, with the requested type_id and stack size.
    /// Pins the basic INSERT contract before the concurrency test stresses it.
    #[tokio::test]
    async fn grants_single_item_into_lowest_free_slot() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        let entity_id: u32 = 0x7000_0301;
        cleanup(&pool, account_id, player_id, entity_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let type_id = pick_main_bag_type_id(&pool).await;

        let (transport, e2a, conn) = make_state(entity_id);
        let db_pool = Some(Arc::new(pool.clone()));

        handle_grant_item(
            entity_id, player_id, type_id, 1, 3, false, &db_pool, &None, &transport, &conn, &e2a,
        )
        .await;

        // Assert "exactly one row" structurally rather than via fetch_optional,
        // which would silently pick whichever row matched first and could
        // mask a regression that inserted multiple rows.
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sgw_inventory \
             WHERE character_id = $1 AND container_id = 1",
        )
        .bind(player_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, 1, "grant must INSERT exactly one row");

        let row: (i32, i32, i32) = sqlx::query_as(
            "SELECT type_id, slot_id, stack_size FROM sgw_inventory \
             WHERE character_id = $1 AND container_id = 1",
        )
        .bind(player_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            row,
            (type_id, 0, 3),
            "grant inserted at slot 0 with the requested type_id and count",
        );

        cleanup(&pool, account_id, player_id, entity_id).await;
    }

    /// Regression guard: pg_advisory_xact_lock on (player_id, container_id)
    /// inside `reserve_free_inventory_slots` must serialize concurrent grants
    /// so each picks a distinct slot. Without it, multiple calls see the same
    /// slot free, all INSERT into it, and the unique-slot index forces all
    /// but one to fail — turning a routine grant into a user-visible error.
    ///
    /// Runs four grants on separately-spawned tasks (so the scheduler can't
    /// trivially serialize them onto a single connection), each holding its
    /// own pool connection (test_pool() bounds at 4 — exact match). A barrier
    /// forces all four to call into reserve_free_inventory_slots near
    /// simultaneously, maximising contention on the advisory lock.
    ///
    /// The single-pair version of this test could plausibly false-negative
    /// when scheduling/DB timing happens to serialize the two futures and
    /// they pick slots 0 then 1 anyway. Four grants colliding makes a
    /// no-lock implementation overwhelmingly likely to drop at least one
    /// row to the unique-slot index.
    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_grants_to_same_container_get_distinct_slots() {
        use tokio::sync::Barrier;

        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 100;
        let player_id = TEST_BASE + 101;
        let entity_id: u32 = 0x7000_0302;
        cleanup(&pool, account_id, player_id, entity_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let type_id = pick_main_bag_type_id(&pool).await;

        let (transport, e2a, conn) = make_state(entity_id);
        let db_pool = Some(Arc::new(pool.clone()));
        const N: usize = 4;
        let barrier = Arc::new(Barrier::new(N));

        let mut handles = Vec::with_capacity(N);
        for _ in 0..N {
            let db_pool = db_pool.clone();
            let transport = transport.clone();
            let conn = conn.clone();
            let e2a = e2a.clone();
            let barrier = barrier.clone();
            handles.push(tokio::spawn(async move {
                // Synchronise so all N calls hit the slot-reservation lock
                // attempt at roughly the same moment.
                barrier.wait().await;
                handle_grant_item(
                    entity_id, player_id, type_id, 1, 1, false, &db_pool, &None, &transport, &conn,
                    &e2a,
                )
                .await;
            }));
        }
        for h in handles {
            h.await.expect("grant task panicked");
        }

        let slots: Vec<i32> = sqlx::query_scalar(
            "SELECT slot_id FROM sgw_inventory \
             WHERE character_id = $1 AND container_id = 1 \
             ORDER BY slot_id",
        )
        .bind(player_id)
        .fetch_all(&pool)
        .await
        .unwrap();

        assert_eq!(
            slots.len(),
            N,
            "all {N} concurrent grants must INSERT (got {} rows). A missing \
             row means the unique-slot index rejected one — exactly the \
             regression the advisory lock prevents.",
            slots.len(),
        );
        assert_eq!(
            slots,
            (0..N as i32).collect::<Vec<_>>(),
            "concurrent grants must pick distinct, sequential slots 0..{N}",
        );

        cleanup(&pool, account_id, player_id, entity_id).await;
    }

    /// Health Slappack TC1 (item 2893). The canonical seed gives it
    /// `max_stack_size = 10` and `container_sets = '{1,17}'`, which
    /// makes it the obvious stackable consumable to pin the merge
    /// path against. Hardcoding the id (instead of a SELECT) ties
    /// the test to the actual on-disk seed shape — a regression
    /// that drops the slappack's `max_stack_size` back to 1 would
    /// trip these tests with a clear "merge expected, allocation
    /// observed" message.
    const SLAPPACK_TYPE_ID: i32 = 2893;
    /// Mirrors the seed value. Used by the "full stack rejects
    /// merge" guard.
    const SLAPPACK_MAX_STACK: i32 = 10;

    /// Two consecutive grants of the same stackable item must
    /// produce ONE row with the combined `stack_size`, not two
    /// rows occupying two slots. The merge fast path is what makes
    /// looted slappacks stop eating one bag slot per pickup —
    /// pre-fix every grant unconditionally reserved a fresh slot.
    #[tokio::test]
    async fn stackable_item_grants_merge_into_one_row() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 200;
        let player_id = TEST_BASE + 201;
        let entity_id: u32 = 0x7000_0303;
        cleanup(&pool, account_id, player_id, entity_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;

        let (transport, e2a, conn) = make_state(entity_id);
        let db_pool = Some(Arc::new(pool.clone()));

        // First grant: 2 slappacks into main bag (container 1).
        // Second grant: 3 more.  Expect one row at stack_size = 5.
        handle_grant_item(
            entity_id,
            player_id,
            SLAPPACK_TYPE_ID,
            1,
            2,
            false,
            &db_pool,
            &None,
            &transport,
            &conn,
            &e2a,
        )
        .await;
        handle_grant_item(
            entity_id,
            player_id,
            SLAPPACK_TYPE_ID,
            1,
            3,
            false,
            &db_pool,
            &None,
            &transport,
            &conn,
            &e2a,
        )
        .await;

        let rows: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT slot_id, stack_size FROM sgw_inventory \
             WHERE character_id = $1 AND container_id = 1 \
               AND type_id = $2 \
             ORDER BY slot_id",
        )
        .bind(player_id)
        .bind(SLAPPACK_TYPE_ID)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            rows.len(),
            1,
            "two grants of slappack (max_stack_size=10) must merge into ONE row — \
             pre-fix the grant path reserved a fresh slot per pickup and produced \
             two rows for what should be one stack of 5"
        );
        assert_eq!(
            rows[0],
            (0, 5),
            "merged stack must occupy the first reserved slot with \
             combined stack_size = 2 + 3 = 5"
        );

        cleanup(&pool, account_id, player_id, entity_id).await;
    }

    /// When the existing stack has no room for the full new count,
    /// the merge path must REFUSE and allocate a fresh slot
    /// instead. Partial-merge "fill the current stack, spill the
    /// remainder" would produce a multi-row outbox payload from
    /// one grant, which the cell side isn't shaped to consume.
    /// Pinning the all-or-nothing rule here means the simpler
    /// invariant — one grant = one row write — survives.
    #[tokio::test]
    async fn full_stack_falls_back_to_new_slot() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 210;
        let player_id = TEST_BASE + 211;
        let entity_id: u32 = 0x7000_0304;
        cleanup(&pool, account_id, player_id, entity_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;

        let (transport, e2a, conn) = make_state(entity_id);
        let db_pool = Some(Arc::new(pool.clone()));

        // Fill a full stack (10 slappacks) into slot 0.
        handle_grant_item(
            entity_id,
            player_id,
            SLAPPACK_TYPE_ID,
            1,
            SLAPPACK_MAX_STACK,
            false,
            &db_pool,
            &None,
            &transport,
            &conn,
            &e2a,
        )
        .await;
        // One more — the existing stack is at max, so this must
        // allocate slot 1 rather than try to push the existing
        // row past its `max_stack_size`.
        handle_grant_item(
            entity_id,
            player_id,
            SLAPPACK_TYPE_ID,
            1,
            1,
            false,
            &db_pool,
            &None,
            &transport,
            &conn,
            &e2a,
        )
        .await;

        let rows: Vec<(i32, i32)> = sqlx::query_as(
            "SELECT slot_id, stack_size FROM sgw_inventory \
             WHERE character_id = $1 AND container_id = 1 \
               AND type_id = $2 \
             ORDER BY slot_id",
        )
        .bind(player_id)
        .bind(SLAPPACK_TYPE_ID)
        .fetch_all(&pool)
        .await
        .unwrap();
        assert_eq!(
            rows.len(),
            2,
            "full stack must NOT accept a merge — the second grant has \
             to land in a fresh slot. Pre-fix bug shape: the merge \
             gate `stack_size + count <= max_stack_size` regresses to \
             `stack_size < max_stack_size` and a single +1 squeezes \
             into a 10/10 stack, exceeding the cap."
        );
        assert_eq!(rows[0], (0, SLAPPACK_MAX_STACK));
        assert_eq!(rows[1], (1, 1));

        cleanup(&pool, account_id, player_id, entity_id).await;
    }

    /// Two grants for two DIFFERENT type_ids must not merge —
    /// stacking is keyed on the type_id, not the container. Pin
    /// this so a future refactor that drops the `inv.type_id = $3`
    /// clause from the merge query doesn't silently start
    /// merging a slappack pickup into the wrong stack.
    #[tokio::test]
    async fn different_type_ids_do_not_merge() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 220;
        let player_id = TEST_BASE + 221;
        let entity_id: u32 = 0x7000_0305;
        cleanup(&pool, account_id, player_id, entity_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;

        let (transport, e2a, conn) = make_state(entity_id);
        let db_pool = Some(Arc::new(pool.clone()));

        // Slappack (2893) and an arbitrary non-stackable item in
        // the same bag. `pick_main_bag_type_id` returns the
        // lowest item_id with container_sets containing 1 —
        // distinct from 2893 by construction.
        let other_type_id = pick_main_bag_type_id(&pool).await;
        assert_ne!(
            other_type_id, SLAPPACK_TYPE_ID,
            "fixture sanity: pick_main_bag_type_id must return a \
             type_id different from the slappack so this test \
             actually exercises the different-type path"
        );

        handle_grant_item(
            entity_id,
            player_id,
            SLAPPACK_TYPE_ID,
            1,
            1,
            false,
            &db_pool,
            &None,
            &transport,
            &conn,
            &e2a,
        )
        .await;
        handle_grant_item(
            entity_id,
            player_id,
            other_type_id,
            1,
            1,
            false,
            &db_pool,
            &None,
            &transport,
            &conn,
            &e2a,
        )
        .await;

        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sgw_inventory \
             WHERE character_id = $1 AND container_id = 1",
        )
        .bind(player_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            count, 2,
            "different type_ids must each take their own slot — \
             merging across type_ids would corrupt inventory by \
             increasing a slappack stack with an unrelated item"
        );

        cleanup(&pool, account_id, player_id, entity_id).await;
    }
}
