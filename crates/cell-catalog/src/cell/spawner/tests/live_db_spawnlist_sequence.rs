//! Live-DB guard for the `spawnlist` id sequence.
//!
//! Campaigns reserve `spawn_id` blocks and insert their rows with explicit
//! ids, but `.savespawn` emits `INSERT INTO resources.spawnlist` without one,
//! so its rows take `nextval('spawnlist_spawn_id_seq')`. A sequence left
//! below a reserved block hands a later saved spawn a reserved id, and the
//! seed then fails on a duplicate key (or two campaigns share one id).
mod live_db {
    use crate::test_support::require_db_or_skip;

    /// The highest reserved campaign spawn id: debug hub 400-404, crafting
    /// 410-429, organizations 430-449, pets 450-469, bank 470-489, social
    /// 490-499. Raise it with the seed's `setval` when a block is added.
    const HIGHEST_RESERVED_SPAWN_ID: i64 = 499;

    #[tokio::test]
    async fn spawnlist_sequence_starts_past_every_reserved_spawn_id() {
        let pool = require_db_or_skip!();
        let (last_value, is_called): (i64, bool) =
            sqlx::query_as("SELECT last_value, is_called FROM resources.spawnlist_spawn_id_seq")
                .fetch_one(&pool)
                .await
                .expect("read spawnlist_spawn_id_seq");
        // The floor is checked on the sequence itself, not only against
        // MAX(spawn_id), so a reserved block with no seeded rows yet stays
        // protected.
        assert!(
            is_called && last_value >= HIGHEST_RESERVED_SPAWN_ID,
            "spawnlist_spawn_id_seq is at {last_value} (is_called {is_called}); it must be set to at least {HIGHEST_RESERVED_SPAWN_ID}"
        );
        // Read, not nextval: the test must not advance the sequence. With
        // is_called = true, nextval returns last_value + 1.
        let next = last_value + 1;
        let max_seeded: i64 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(spawn_id), 0)::bigint FROM resources.spawnlist",
        )
        .fetch_one(&pool)
        .await
        .expect("max spawn_id");
        assert!(
            next > max_seeded,
            "the next default spawn_id {next} collides with a seeded row (max {max_seeded})"
        );
    }
}
