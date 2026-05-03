use std::sync::Arc;

use sqlx::PgPool;

use crate::cell::messages::SavedMission;

pub async fn query_saved_missions(
    db_pool: &Option<Arc<PgPool>>,
    player_id: i32,
) -> Vec<SavedMission> {
    let pool = match db_pool {
        Some(p) => p,
        None => return vec![],
    };

    #[derive(sqlx::FromRow)]
    struct MissionRow {
        mission_id: i32,
        status: i32,
        current_step_id: Option<i32>,
        completed_step_ids: Vec<i32>,
        completed_objective_ids: Vec<i32>,
        active_objective_ids: Vec<i32>,
        failed_objective_ids: Vec<i32>,
        repeats: i32,
    }

    match sqlx::query_as::<_, MissionRow>(
        "SELECT mission_id, status, current_step_id, \
         completed_step_ids, completed_objective_ids, active_objective_ids, failed_objective_ids, \
         repeats \
         FROM sgw_mission WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_all(pool.as_ref())
    .await
    {
        Ok(rows) => {
            let missions: Vec<_> = rows
                .into_iter()
                .map(|r| {
                    let status = i8::try_from(r.status).unwrap_or_else(|_| {
                        tracing::warn!(
                            player_id,
                            mission_id = r.mission_id,
                            db_status = r.status,
                            "Mission status out of i8 range, clamping"
                        );
                        // Clamp at i32, then cast — casting first wraps modulo 256.
                        r.status.clamp(i8::MIN as i32, i8::MAX as i32) as i8
                    });
                    SavedMission {
                        mission_id: r.mission_id,
                        status,
                        current_step_id: r.current_step_id,
                        completed_step_ids: r.completed_step_ids,
                        completed_objective_ids: r.completed_objective_ids,
                        active_objective_ids: r.active_objective_ids,
                        failed_objective_ids: r.failed_objective_ids,
                        repeats: r.repeats,
                    }
                })
                .collect();
            tracing::info!(
                player_id,
                count = missions.len(),
                "Loaded saved missions from DB"
            );
            missions
        }
        Err(e) => {
            tracing::error!(player_id, "Failed to query saved missions: {e}");
            vec![]
        }
    }
}

pub async fn handle_mission_update(
    player_id: i32,
    mission_id: i32,
    status: i8,
    current_step_id: Option<i32>,
    completed_step_ids: &[i32],
    completed_objective_ids: &[i32],
    active_objective_ids: &[i32],
    failed_objective_ids: &[i32],
    repeats: i32,
    db_pool: &Option<Arc<PgPool>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, mission_id, "MissionUpdate: no DB pool");
            return;
        }
    };

    // `repeats = EXCLUDED.repeats` is the fix for #118 — the prior UPSERT
    // omitted this column, so re-completing a repeatable mission appeared to
    // reset the counter on relog instead of advancing it. Cell is the
    // authoritative source for the post-bump value (set by
    // `MissionInstance::complete`/`fail`).
    let result = sqlx::query(
        "INSERT INTO sgw_mission (player_id, mission_id, status, current_step_id, \
         completed_step_ids, completed_objective_ids, active_objective_ids, failed_objective_ids, \
         repeats) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
         ON CONFLICT (player_id, mission_id) DO UPDATE SET \
         status = EXCLUDED.status, \
         current_step_id = EXCLUDED.current_step_id, \
         completed_step_ids = EXCLUDED.completed_step_ids, \
         completed_objective_ids = EXCLUDED.completed_objective_ids, \
         active_objective_ids = EXCLUDED.active_objective_ids, \
         failed_objective_ids = EXCLUDED.failed_objective_ids, \
         repeats = EXCLUDED.repeats",
    )
    .bind(player_id)
    .bind(mission_id)
    .bind(status as i32)
    .bind(current_step_id)
    .bind(completed_step_ids)
    .bind(completed_objective_ids)
    .bind(active_objective_ids)
    .bind(failed_objective_ids)
    .bind(repeats)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => tracing::debug!(
            player_id,
            mission_id,
            status,
            repeats,
            "Mission state persisted"
        ),
        Err(e) => tracing::error!(player_id, mission_id, "Failed to persist mission: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for mission tests. Distinct from grant_cash (0x7000_0100),
    /// move_inventory (0x7000_0200), grant_item (0x7000_0300).
    const TEST_PLAYER_BASE: i32 = 0x7000_0400;

    /// sgw_mission has a FK to sgw_player. Cleanup deletes the account, which
    /// cascades sgw_player rows; sgw_mission rows go away when the player rows
    /// they reference are deleted (or via the FK rule). Mission rows are also
    /// deleted explicitly first to avoid relying on cascade ordering.
    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        let _ = sqlx::query("DELETE FROM sgw_mission WHERE player_id = $1")
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
        .bind(format!("mission-test-{account_id}"))
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

    /// Happy path: handle_mission_update INSERTs a row with all fields present.
    #[tokio::test]
    async fn inserts_new_mission_row_with_all_fields() {
        let pool = require_db_or_skip!();
        let account_id = TEST_PLAYER_BASE;
        let player_id = TEST_PLAYER_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let db_pool = Some(Arc::new(pool.clone()));

        handle_mission_update(
            player_id,
            12345,
            2,
            Some(7),
            &[1, 2, 3],
            &[10, 20],
            &[30],
            &[],
            5,
            &db_pool,
        )
        .await;

        // Filter on (player_id, mission_id) and use fetch_one so a regression
        // that inserted multiple rows would surface either as the count check
        // failing or as fetch_one's "expected one row" error.
        let row_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM sgw_mission WHERE player_id = $1 AND mission_id = $2",
        )
        .bind(player_id)
        .bind(12345)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row_count, 1, "INSERT must produce exactly one row");

        let row: (
            i32,
            i32,
            Option<i32>,
            Vec<i32>,
            Vec<i32>,
            Vec<i32>,
            Vec<i32>,
            i32,
        ) = sqlx::query_as(
            "SELECT mission_id, status, current_step_id, \
                    completed_step_ids, completed_objective_ids, \
                    active_objective_ids, failed_objective_ids, repeats \
             FROM sgw_mission WHERE player_id = $1 AND mission_id = $2",
        )
        .bind(player_id)
        .bind(12345)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(row.0, 12345, "mission_id");
        assert_eq!(row.1, 2, "status");
        assert_eq!(row.2, Some(7), "current_step_id");
        assert_eq!(row.3, vec![1, 2, 3], "completed_step_ids");
        assert_eq!(row.4, vec![10, 20], "completed_objective_ids");
        assert_eq!(row.5, vec![30], "active_objective_ids");
        assert_eq!(row.6, Vec::<i32>::new(), "failed_objective_ids");
        assert_eq!(row.7, 5, "repeats");

        cleanup(&pool, account_id, player_id).await;
    }

    /// Regression guard: the prior UPSERT omitted `repeats = EXCLUDED.repeats`,
    /// so re-completing a repeatable mission would appear to reset the counter
    /// on relog instead of advancing it. This test seeds a row with repeats=3,
    /// then runs an UPSERT with repeats=4 — the row must persist 4, not 3.
    #[tokio::test]
    async fn upsert_propagates_repeats_column_on_conflict() {
        let pool = require_db_or_skip!();
        let account_id = TEST_PLAYER_BASE + 100;
        let player_id = TEST_PLAYER_BASE + 101;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        let db_pool = Some(Arc::new(pool.clone()));

        // Seed: mission already at repeats=3, status=1.
        handle_mission_update(player_id, 9999, 1, None, &[], &[], &[], &[], 3, &db_pool).await;

        // Re-complete: cell now sends status=2, repeats=4.
        handle_mission_update(player_id, 9999, 2, None, &[], &[], &[], &[], 4, &db_pool).await;

        let (status, repeats): (i32, i32) = sqlx::query_as(
            "SELECT status, repeats FROM sgw_mission \
             WHERE player_id = $1 AND mission_id = $2",
        )
        .bind(player_id)
        .bind(9999)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(status, 2, "status must update on conflict");
        assert_eq!(
            repeats, 4,
            "repeats must propagate via EXCLUDED.repeats on conflict — \
             a 3 here means the UPSERT regressed to omitting the column",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// Regression guard: query_saved_missions uses i8::try_from(r.status) with
    /// a clamp fallback. A pathological row whose `status` column doesn't fit
    /// in i8 must clamp to i8::MIN/MAX rather than wrap modulo 256 via `as i8`
    /// (which would silently turn a 200 into -56). Done at the query layer
    /// because the `status` column is `integer` so legitimate writes can
    /// outpace the i8 wire/cell representation.
    #[tokio::test]
    async fn query_saved_missions_clamps_out_of_range_status_to_i8_max() {
        let pool = require_db_or_skip!();
        let account_id = TEST_PLAYER_BASE + 200;
        let player_id = TEST_PLAYER_BASE + 201;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;

        // Insert directly via SQL so we can write a status that handle_mission_update
        // (which takes i8) couldn't produce. 500 is well outside i8 range.
        sqlx::query(
            "INSERT INTO sgw_mission \
                (player_id, mission_id, status, current_step_id, \
                 completed_step_ids, completed_objective_ids, \
                 active_objective_ids, failed_objective_ids, repeats) \
             VALUES ($1, 1, 500, NULL, '{}', '{}', '{}', '{}', 0)",
        )
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("insert oversize-status mission");

        let db_pool = Some(Arc::new(pool.clone()));
        let missions = query_saved_missions(&db_pool, player_id).await;
        assert_eq!(missions.len(), 1);
        assert_eq!(
            missions[0].status,
            i8::MAX,
            "status 500 (outside i8) must clamp to i8::MAX, not wrap modulo 256 to -12",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// Companion: clamp on the negative side too. A status of -200 must
    /// clamp to i8::MIN (-128), not wrap to +56.
    #[tokio::test]
    async fn query_saved_missions_clamps_out_of_range_status_to_i8_min() {
        let pool = require_db_or_skip!();
        let account_id = TEST_PLAYER_BASE + 300;
        let player_id = TEST_PLAYER_BASE + 301;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;

        sqlx::query(
            "INSERT INTO sgw_mission \
                (player_id, mission_id, status, current_step_id, \
                 completed_step_ids, completed_objective_ids, \
                 active_objective_ids, failed_objective_ids, repeats) \
             VALUES ($1, 1, -200, NULL, '{}', '{}', '{}', '{}', 0)",
        )
        .bind(player_id)
        .execute(&pool)
        .await
        .expect("insert negative-status mission");

        let db_pool = Some(Arc::new(pool.clone()));
        let missions = query_saved_missions(&db_pool, player_id).await;
        assert_eq!(missions.len(), 1);
        assert_eq!(
            missions[0].status,
            i8::MIN,
            "status -200 must clamp to i8::MIN, not wrap modulo 256 to +56",
        );

        cleanup(&pool, account_id, player_id).await;
    }
}
