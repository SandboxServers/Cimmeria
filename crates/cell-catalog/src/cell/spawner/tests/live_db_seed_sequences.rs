//! Live-DB guard that the id sequences of the tables campaigns seed explicit
//! id blocks into are past every seeded row after a load.
//!
//! Content authoring inserts with default ids (`nextval`). A seed footer that
//! sets its sequence to a fixed value goes stale as soon as a campaign seeds
//! ids above it, and the next default-id insert collides with a seeded row.
mod live_db {
    use crate::test_support::require_db_or_skip;

    /// `(sequence, table, id column)`, all in `resources`.
    const SEQUENCES: [(&str, &str, &str); 4] = [
        (
            "entity_templates_template_id_seq",
            "entity_templates",
            "template_id",
        ),
        ("spawnlist_spawn_id_seq", "spawnlist", "spawn_id"),
        ("item_lists_item_list_id_seq", "item_lists", "item_list_id"),
        ("item_list_items_item_id_seq", "item_list_items", "item_id"),
    ];

    /// Live-DB test sentinels start here (`0x7000_0000`, TESTING.md). Some
    /// tests leave a shared sentinel fixture row in these tables on purpose
    /// (the paid-recharge link at `0x7FFF_BBBC`); those are not seed rows.
    const FIRST_TEST_SENTINEL: i64 = 0x7000_0000;

    #[tokio::test]
    async fn seeded_sequences_allocate_past_every_seeded_row() {
        let pool = require_db_or_skip!();
        for (seq, table, column) in SEQUENCES {
            // What the next `nextval` returns, without consuming it.
            let sql = format!(
                "SELECT CASE WHEN s.is_called THEN s.last_value + 1 ELSE s.last_value END, \
                        (SELECT MAX({column}) FROM resources.{table} \
                         WHERE {column} < {FIRST_TEST_SENTINEL})::int8 \
                 FROM resources.{seq} s"
            );
            let (next, max): (i64, Option<i64>) = sqlx::query_as(sqlx::AssertSqlSafe(sql))
                .fetch_one(&pool)
                .await
                .unwrap_or_else(|e| panic!("{seq} query must succeed: {e}"));
            let max = max.unwrap_or(0);
            assert!(
                next > max,
                "{seq} would hand out {next}, but resources.{table} already holds {column} {max}"
            );
        }
    }
}
