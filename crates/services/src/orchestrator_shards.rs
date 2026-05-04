//! Shard lookup helpers for the orchestrator.
//!
//! Reads shard rows from the `shards` table at startup so the auth service
//! can advertise them in Phase 1 responses.

use std::sync::Arc;

use sqlx::PgPool;

/// Row returned by [`query_all_shards`].
pub(crate) struct ShardRow {
    pub(crate) name: String,
    pub(crate) protected: bool,
}

/// Query all shards from the `shards` table, ordered by shard_id.
/// Returns a fallback single-entry list on error.
pub(crate) async fn query_all_shards(pool: &Arc<PgPool>) -> Vec<ShardRow> {
    #[derive(sqlx::FromRow)]
    struct DbShardRow {
        name: Option<String>,
        protected: bool,
    }

    match sqlx::query_as::<_, DbShardRow>("SELECT name, protected FROM shards ORDER BY shard_id")
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(rows) => {
            let shards: Vec<ShardRow> = rows
                .into_iter()
                .filter_map(|r| {
                    r.name.map(|n| ShardRow {
                        name: n,
                        protected: r.protected,
                    })
                })
                .collect();
            if shards.is_empty() {
                tracing::error!("No shards found in database — using fallback name 'Shard'. Run: INSERT INTO shards (shard_id, name, key, protected) VALUES (1, 'Test', '', false);");
                vec![ShardRow {
                    name: "Shard".to_string(),
                    protected: false,
                }]
            } else {
                tracing::info!(
                    count = shards.len(),
                    "Loaded shards from database: {:?}",
                    shards.iter().map(|s| &s.name).collect::<Vec<_>>()
                );
                shards
            }
        }
        Err(e) => {
            tracing::error!("Failed to query shards table: {e} — using fallback name 'Shard'. Ensure the 'shards' table exists (re-run Initialize-CimmeriaDatabase -Force).");
            vec![ShardRow {
                name: "Shard".to_string(),
                protected: false,
            }]
        }
    }
}

#[cfg(test)]
mod tests {
    //! Live-DB tests for the shard loader.
    //!
    //! `query_all_shards` is the only contract the auth service has on
    //! shard discovery — Phase 1 responses to the client mirror what
    //! comes out of this function. A drift between `SELECT name, protected
    //! ORDER BY shard_id` and the column types / NULL handling silently
    //! changes which shards the client sees.
    //!
    //! Test-isolation strategy: insert sentinel `shard_id` values in the
    //! 0x7000_xxxx slot (well outside any production ID we'd ever assign),
    //! filter the function's output to those sentinels, assert ordering
    //! and content, then DELETE by sentinel on cleanup. The seeded
    //! `(1, 'Test', '', false)` row is left untouched.
    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel `shard_id` base. Keep above `i32::MAX / 2` so a bug that
    /// truncates to `i16` would land somewhere obviously wrong.
    const TEST_SHARD_BASE: i32 = 0x7000_0000;

    async fn cleanup(pool: &PgPool) {
        let _ = sqlx::query("DELETE FROM shards WHERE shard_id >= $1")
            .bind(TEST_SHARD_BASE)
            .execute(pool)
            .await;
    }

    /// The seeded `(1, 'Test', '', false)` row from
    /// `db/sgw/Shards/Seed/shards.sql` must round-trip through
    /// `query_all_shards`. This is the smoke that proves the column
    /// names + types in the query line up with the schema.
    #[tokio::test]
    async fn query_all_shards_returns_seeded_test_shard() {
        let pool = Arc::new(require_db_or_skip!());
        let rows = query_all_shards(&pool).await;

        // The seeded shard_id=1 'Test' row is one of the canonical
        // entries; assert its presence rather than equality on length
        // (a dev's local DB may have additional shards seeded for
        // experimentation).
        let test_shard = rows.iter().find(|r| r.name == "Test");
        assert!(
            test_shard.is_some(),
            "expected the seeded 'Test' shard (shard_id=1) in query_all_shards output; \
             got names={:?}",
            rows.iter().map(|r| &r.name).collect::<Vec<_>>(),
        );
        assert!(
            !test_shard.unwrap().protected,
            "seeded 'Test' shard is protected=false; query mapping must preserve the column"
        );
    }

    /// Multi-shard ordering: when two extra rows are inserted with
    /// `shard_id` ascending, `query_all_shards` must emit them in
    /// `shard_id` order. The auth service relies on stable ordering
    /// for Phase 1 advertisement — clients pin shards by index, so a
    /// flipped order silently re-maps which shard the user lands on.
    #[tokio::test]
    async fn query_all_shards_orders_results_by_shard_id() {
        let pool = Arc::new(require_db_or_skip!());
        cleanup(&pool).await;

        // Insert in REVERSE shard_id order so a missing ORDER BY would
        // surface as the inserted-order list.
        let high_id = TEST_SHARD_BASE + 0x20;
        let low_id = TEST_SHARD_BASE + 0x10;
        sqlx::query(
            "INSERT INTO shards (shard_id, name, key, protected) VALUES ($1, $2, '', false)",
        )
        .bind(high_id)
        .bind("ZShardHigh")
        .execute(pool.as_ref())
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO shards (shard_id, name, key, protected) VALUES ($1, $2, '', false)",
        )
        .bind(low_id)
        .bind("AShardLow")
        .execute(pool.as_ref())
        .await
        .unwrap();

        let rows = query_all_shards(&pool).await;
        let positions: Vec<usize> = ["AShardLow", "ZShardHigh"]
            .iter()
            .filter_map(|name| rows.iter().position(|r| r.name == *name))
            .collect();
        assert_eq!(
            positions.len(),
            2,
            "both inserted sentinels must round-trip through query_all_shards"
        );
        assert!(
            positions[0] < positions[1],
            "AShardLow (shard_id={low_id}) must appear before ZShardHigh (shard_id={high_id}) — query missed ORDER BY"
        );

        cleanup(&pool).await;
    }

    /// `query_all_shards` filters out rows whose `name` is NULL — the
    /// `name: Option<String>` projection is `filter_map`'d on the
    /// caller side. Without this filter, the auth service would
    /// advertise a `Shard { name: "" }`-shaped shard to the client
    /// (deserialization-dependent behavior), which the client treats
    /// as a malformed catalog entry. Pin the filter explicitly.
    #[tokio::test]
    async fn query_all_shards_filters_null_name_rows() {
        let pool = Arc::new(require_db_or_skip!());
        cleanup(&pool).await;

        let null_name_id = TEST_SHARD_BASE + 0x30;
        let real_name_id = TEST_SHARD_BASE + 0x31;
        sqlx::query(
            "INSERT INTO shards (shard_id, name, key, protected) VALUES ($1, NULL, '', false)",
        )
        .bind(null_name_id)
        .execute(pool.as_ref())
        .await
        .unwrap();
        sqlx::query("INSERT INTO shards (shard_id, name, key, protected) VALUES ($1, 'NamedSentinel', '', false)")
            .bind(real_name_id)
            .execute(pool.as_ref())
            .await
            .unwrap();

        let rows = query_all_shards(&pool).await;
        // The named row must come through.
        assert!(
            rows.iter().any(|r| r.name == "NamedSentinel"),
            "real-name sentinel must round-trip even when sibling NULL-name row is present"
        );
        // No row in the output should have an empty name (the null-name
        // row would manifest as either empty or a phantom entry — neither
        // is acceptable in the auth Phase 1 advertisement).
        assert!(
            !rows.iter().any(|r| r.name.is_empty()),
            "NULL-name row must be filter_map'd away, not surface as an empty-string entry"
        );

        cleanup(&pool).await;
    }

    /// `protected` column round-trips end-to-end. Auth Phase 1 advertises
    /// this flag so the client knows whether the shard requires elevated
    /// auth — a column-mapping drift would silently flip every shard's
    /// gating.
    #[tokio::test]
    async fn query_all_shards_preserves_protected_flag() {
        let pool = Arc::new(require_db_or_skip!());
        cleanup(&pool).await;

        let protected_id = TEST_SHARD_BASE + 0x40;
        let unprotected_id = TEST_SHARD_BASE + 0x41;
        sqlx::query("INSERT INTO shards (shard_id, name, key, protected) VALUES ($1, 'ProtSentinel', '', true)")
            .bind(protected_id)
            .execute(pool.as_ref())
            .await
            .unwrap();
        sqlx::query("INSERT INTO shards (shard_id, name, key, protected) VALUES ($1, 'OpenSentinel', '', false)")
            .bind(unprotected_id)
            .execute(pool.as_ref())
            .await
            .unwrap();

        let rows = query_all_shards(&pool).await;
        let prot = rows
            .iter()
            .find(|r| r.name == "ProtSentinel")
            .expect("protected sentinel missing");
        let open = rows
            .iter()
            .find(|r| r.name == "OpenSentinel")
            .expect("open sentinel missing");
        assert!(prot.protected, "protected=true must round-trip");
        assert!(!open.protected, "protected=false must round-trip");

        cleanup(&pool).await;
    }
}
