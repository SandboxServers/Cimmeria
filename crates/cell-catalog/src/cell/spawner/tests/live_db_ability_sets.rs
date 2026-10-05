//! Live-DB guard for [`load_ability_sets`] (AB-N2, `gmSetMobAbilitySet`):
//! a set's rows load grouped under its id, every row present, ascending.
//! Fails if the query drops rows, mis-groups them, or loses the ordering.
//!
//! The set is a sentinel this test inserts (`SENTINEL_SET`, reserved block
//! `0x7031_0Axx`) over real ability ids picked at run time, so no assertion
//! leans on production seed ids. Cleanup deletes exactly the sentinel.

mod live_db {
    use crate::cell::spawner::load_ability_sets;
    use crate::test_support::require_db_or_skip;

    const SENTINEL_SET: i32 = 0x7031_0A00;

    async fn cleanup(pool: &sqlx::PgPool) {
        for sql in [
            "DELETE FROM resources.ability_set_abilities WHERE ability_set_id = $1",
            "DELETE FROM resources.ability_sets WHERE ability_set_id = $1",
        ] {
            sqlx::query(sql)
                .bind(SENTINEL_SET)
                .execute(pool)
                .await
                .expect("delete the sentinel set");
        }
    }

    #[tokio::test]
    async fn live_db_ability_sets_load_grouped_by_set() {
        let pool = require_db_or_skip!();
        cleanup(&pool).await;
        let mut picked: Vec<i32> = sqlx::query_scalar(
            "SELECT ability_id FROM resources.abilities ORDER BY ability_id LIMIT 3",
        )
        .fetch_all(&pool)
        .await
        .expect("pick abilities");
        assert_eq!(picked.len(), 3, "fixture: the seed must have 3 abilities");
        sqlx::query("INSERT INTO resources.ability_sets (ability_set_id, description) VALUES ($1, 'AB-N2 test set')")
            .bind(SENTINEL_SET)
            .execute(&pool)
            .await
            .expect("insert the sentinel set");
        // Out of order, so the load's ordering is what sorts them.
        for id in [picked[2], picked[0], picked[1]] {
            let r = sqlx::query(
                "INSERT INTO resources.ability_set_abilities (ability_set_id, ability_id) \
                 VALUES ($1, $2)",
            )
            .bind(SENTINEL_SET)
            .bind(id)
            .execute(&pool)
            .await
            .expect("insert a sentinel row");
            assert_eq!(r.rows_affected(), 1);
        }

        let sets = load_ability_sets(&pool).await.expect("load ability sets");
        picked.sort_unstable();
        assert_eq!(
            sets.get(&SENTINEL_SET),
            Some(&picked),
            "every row of the set, grouped and ascending"
        );
        cleanup(&pool).await;
    }
}
