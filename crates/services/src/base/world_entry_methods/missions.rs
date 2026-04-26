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
    }

    match sqlx::query_as::<_, MissionRow>(
        "SELECT mission_id, status, current_step_id, \
         completed_step_ids, completed_objective_ids, active_objective_ids, failed_objective_ids \
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
                    let status = match i8::try_from(r.status) {
                        Ok(s) => s,
                        Err(_) => {
                            tracing::warn!(
                                player_id,
                                mission_id = r.mission_id,
                                db_status = r.status,
                                "Mission status out of i8 range, clamping"
                            );
                            127i8.min((-128i8).max(r.status as i8))
                        }
                    };
                    SavedMission {
                        mission_id: r.mission_id,
                        status,
                        current_step_id: r.current_step_id,
                        completed_step_ids: r.completed_step_ids,
                        completed_objective_ids: r.completed_objective_ids,
                        active_objective_ids: r.active_objective_ids,
                        failed_objective_ids: r.failed_objective_ids,
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
    db_pool: &Option<Arc<PgPool>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, mission_id, "MissionUpdate: no DB pool");
            return;
        }
    };

    let result = sqlx::query(
        "INSERT INTO sgw_mission (player_id, mission_id, status, current_step_id, \
         completed_step_ids, completed_objective_ids, active_objective_ids, failed_objective_ids) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         ON CONFLICT (player_id, mission_id) DO UPDATE SET \
         status = EXCLUDED.status, \
         current_step_id = EXCLUDED.current_step_id, \
         completed_step_ids = EXCLUDED.completed_step_ids, \
         completed_objective_ids = EXCLUDED.completed_objective_ids, \
         active_objective_ids = EXCLUDED.active_objective_ids, \
         failed_objective_ids = EXCLUDED.failed_objective_ids",
    )
    .bind(player_id)
    .bind(mission_id)
    .bind(status as i32)
    .bind(current_step_id)
    .bind(completed_step_ids)
    .bind(completed_objective_ids)
    .bind(active_objective_ids)
    .bind(failed_objective_ids)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => tracing::debug!(player_id, mission_id, status, "Mission state persisted"),
        Err(e) => tracing::error!(player_id, mission_id, "Failed to persist mission: {e}"),
    }
}
