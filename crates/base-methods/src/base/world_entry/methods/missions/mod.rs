use std::sync::Arc;

use cimmeria_entity::known_names;
use cimmeria_entity::missions::MISSION_NOT_ACTIVE;
use sqlx::PgPool;

use crate::cell::messages::SavedMission;

pub mod letter_turn_in;
pub mod state_lock;

/// Read the saved missions, falling back to none when the read fails.
///
/// World entry does not use this: it goes through
/// [`state_lock::load_saved_missions_with_retry`], which locks the missions
/// instead of falling back (TG-DB-12). The fallback stays for the callers
/// that only read, and it still says what it costs.
pub async fn query_saved_missions(
    db_pool: &Option<Arc<PgPool>>,
    player_id: i32,
) -> Vec<SavedMission> {
    match try_query_saved_missions(db_pool, player_id).await {
        Ok(missions) => missions,
        Err(e) => {
            tracing::error!(
                event = "player_load_degraded",
                reason = "missions_query_failed",
                player_id,
                player_name = known_names::player_name(player_id),
                error = %e,
                "Saved missions query failed when loading player data; the player loads with no missions, a re-accept will overwrite saved progress"
            );
            vec![]
        }
    }
}

/// Read the saved missions. No pool reads as none (dev only); a failed
/// query is the caller's to handle.
pub async fn try_query_saved_missions(
    db_pool: &Option<Arc<PgPool>>,
    player_id: i32,
) -> Result<Vec<SavedMission>, sqlx::Error> {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(
                player_id,
                player_name = known_names::player_name(player_id),
                reason = "no_db_pool",
                "Saved missions not read: no DB pool"
            );
            return Ok(vec![]);
        }
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

    let rows = sqlx::query_as::<_, MissionRow>(
        "SELECT mission_id, status, current_step_id, \
         completed_step_ids, completed_objective_ids, active_objective_ids, failed_objective_ids, \
         repeats \
         FROM sgw_mission WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_all(pool.as_ref())
    .await?;
    let missions: Vec<_> = rows
        .into_iter()
        .map(|r| {
            let status = i8::try_from(r.status).unwrap_or_else(|_| {
                tracing::warn!(
                    player_id,
                    player_name = known_names::player_name(player_id),
                    mission_id = r.mission_id,
                    mission_name = cimmeria_names::book().mission(r.mission_id),
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
        player_name = known_names::player_name(player_id),
        count = missions.len(),
        "Loaded saved missions from DB"
    );
    Ok(missions)
}

#[tracing::instrument(
    name = "mission.persist",
    level = "info",
    skip_all,
    fields(
        player_id,
        player_name = tracing::field::Empty,
        mission_id,
        mission_name = tracing::field::Empty,
        status,
        current_step_id,
        current_step_name = tracing::field::Empty,
        repeats,
    ),
)]
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
    let player_name = known_names::player_name(player_id);
    {
        // The guard is dropped here: never held across the `.await` below.
        let book = cimmeria_names::book();
        let span = tracing::Span::current();
        span.record("player_name", player_name);
        span.record("mission_name", book.mission(mission_id));
        span.record(
            "current_step_name",
            current_step_id.and_then(|s| book.mission_step(s)),
        );
    }
    // The step name rides on the persist rows beside its id (Rule 6).
    let current_step_name =
        current_step_id.and_then(|s| cimmeria_names::book().mission_step(s).map(str::to_owned));
    // TG-DB-12 backstop: the cell refuses mission changes while the saved
    // missions are unreadable, so nothing should reach here; if something
    // does, it was built from an empty tracker and would overwrite the row.
    if state_lock::refuse_persist_if_locked(player_id, mission_id, status) {
        return;
    }
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(
                player_id,
                player_name,
                mission_id,
                mission_name = cimmeria_names::book().mission(mission_id),
                reason = "no_db_pool",
                "MissionUpdate: no DB pool"
            );
            return;
        }
    };

    if mission_id == 1360 {
        let result = letter_turn_in::persist_active_letter_mission(
            pool.as_ref(),
            player_id,
            status,
            current_step_id,
            completed_step_ids,
            completed_objective_ids,
            active_objective_ids,
            failed_objective_ids,
            repeats,
        )
        .await;
        match result {
            Ok(true) => tracing::debug!(
                player_id,
                mission_id,
                "Mission 1360 active snapshot persisted"
            ),
            Ok(false) => tracing::warn!(
                player_id,
                mission_id,
                status,
                current_step_id,
                reason = "invalid_or_stale_snapshot",
                "Mission 1360 generic update refused"
            ),
            Err(e) => tracing::error!(
                player_id,
                mission_id,
                reason = "db_write_failed",
                error = %e,
                "Mission 1360 active snapshot write failed"
            ),
        }
        return;
    }

    // An abandon (#1315) arrives as `MISSION_NOT_ACTIVE`. With no completed
    // run to remember there is nothing to keep, so the row goes: the
    // mission loads as never taken. With `repeats > 0` the UPSERT below keeps
    // a not-active row, so the count survives (#118).
    if status == MISSION_NOT_ACTIVE && repeats == 0 {
        let result =
            sqlx::query("DELETE FROM sgw_mission WHERE player_id = $1 AND mission_id = $2")
                .bind(player_id)
                .bind(mission_id)
                .execute(pool.as_ref())
                .await;
        match result {
            Ok(done) => tracing::debug!(
                player_id,
                player_name,
                mission_id,
                mission_name = cimmeria_names::book().mission(mission_id),
                rows = done.rows_affected(),
                "Abandoned mission row deleted"
            ),
            Err(e) => tracing::error!(
                player_id,
                player_name,
                mission_id,
                mission_name = cimmeria_names::book().mission(mission_id),
                current_step_id,
                current_step_name = current_step_name.as_deref(),
                status,
                reason = "db_write_failed",
                error = %e,
                "Failed to delete abandoned mission"
            ),
        }
        return;
    }

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
            player_name,
            mission_id,
            mission_name = cimmeria_names::book().mission(mission_id),
            current_step_id,
            current_step_name = current_step_name.as_deref(),
            status,
            repeats,
            "Mission state persisted"
        ),
        Err(e) => tracing::error!(
            player_id,
            player_name,
            mission_id,
            mission_name = cimmeria_names::book().mission(mission_id),
            current_step_id,
            current_step_name = current_step_name.as_deref(),
            status,
            reason = "db_write_failed",
            error = %e,
            "Failed to persist mission"
        ),
    }
}

// Split out of a single missions.rs (was 691 lines, over the repo's 700-line
// hard cap) when C04 added a persistence round-trip test for mission 1360 —
// see docs/analysis/castle-cellblock-rebuild/work-packets.md#c04. Production
// code stays here; every existing and new test moved to tests.rs verbatim
// (no behavior change, no public-surface change — `pub mod missions;` in
// ../mod.rs resolves identically whether this is missions.rs or
// missions/mod.rs).
#[cfg(test)]
mod tests;

#[cfg(test)]
mod letter_turn_in_tests;

#[cfg(test)]
mod letter_turn_in_concurrency_tests;
