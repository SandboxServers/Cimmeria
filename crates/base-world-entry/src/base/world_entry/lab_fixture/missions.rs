//! Missions of a staged fixture, written as the saved rows the route into
//! that state leaves behind.
//!
//! The base needs no mission definitions beyond the step and objective
//! ids: hydration rebuilds the current step's objective roster from
//! `resources.mission_objectives` and re-derives the hidden and optional
//! flags (the cell's `mission_restore`), so the row carries only the
//! status, the current step, the completed steps and objectives, and
//! `repeats`. Every row goes through `handle_mission_update`, the base's
//! one mission persist, so an abandon-shaped row (`MISSION_NOT_ACTIVE`,
//! `repeats = 0`) is deleted by the same rule a real abandon uses.
//!
//! Step order is `resources.mission_steps.index` ascending, the column the
//! cell's mission cache picks a mission's first step by
//! (`load_mission_defs`); `step_id` breaks ties.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use cimmeria_entity::missions::{MISSION_ACTIVE, MISSION_COMPLETED, MISSION_NOT_ACTIVE};
use sqlx::PgPool;

use cimmeria_base_session::base::world_entry::lab_fixture::{StagedMission, StagedMissionState};

use super::super::methods::missions::handle_mission_update;

/// The row one fixture mission writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MissionRow {
    mission_id: i32,
    status: i8,
    current_step_id: Option<i32>,
    completed_step_ids: Vec<i32>,
    completed_objective_ids: Vec<i32>,
    repeats: i32,
}

/// Check every mission of the fixture against the seed and build its row,
/// writing nothing: the apply's precheck, so a bad step anywhere in the
/// fixture writes nothing at all.
pub(crate) async fn plan_missions(
    pool: &PgPool,
    missions: &[StagedMission],
) -> Result<Vec<MissionRow>, String> {
    let mut seen = HashSet::new();
    let mut rows = Vec::with_capacity(missions.len());
    for m in missions {
        if !seen.insert(m.mission_id) {
            return Err(format!("mission {} is listed twice", m.mission_id));
        }
        rows.push(build_row(pool, m).await?);
    }
    Ok(rows)
}

/// Write the rows [`plan_missions`] built; returns how many were written.
pub(crate) async fn write_missions(
    pool: &PgPool,
    player_id: i32,
    rows: &[MissionRow],
) -> Result<usize, String> {
    // `handle_mission_update` takes the session-shaped optional pool; the
    // clone is a handle to the same pool.
    let db = Some(Arc::new(pool.clone()));
    for row in rows {
        handle_mission_update(
            player_id,
            row.mission_id,
            row.status,
            row.current_step_id,
            &row.completed_step_ids,
            &row.completed_objective_ids,
            // Hydration rebuilds the current step's roster from the step.
            &[],
            &[],
            row.repeats,
            &db,
        )
        .await;
        // The persist logs a failure and returns nothing, and refuses the
        // write outright for a player whose saved missions are locked
        // (TG-DB-12 `refuse_persist_if_locked`); read the row back so either
        // fails the fixture instead of passing silently.
        confirm_written(pool, player_id, row).await?;
    }
    Ok(rows.len())
}

/// Check `m` against the seed and build its row.
async fn build_row(pool: &PgPool, m: &StagedMission) -> Result<MissionRow, String> {
    match m.state {
        StagedMissionState::NotActive => Ok(MissionRow {
            mission_id: m.mission_id,
            status: MISSION_NOT_ACTIVE,
            current_step_id: None,
            completed_step_ids: Vec::new(),
            completed_objective_ids: Vec::new(),
            // With no completed run, the persist deletes the row: the
            // mission loads as never taken.
            repeats: 0,
        }),
        StagedMissionState::Active { step_id } => {
            let owner: Option<i32> = sqlx::query_scalar(
                "SELECT mission_id FROM resources.mission_steps WHERE step_id = $1",
            )
            .bind(step_id)
            .fetch_optional(pool)
            .await
            .map_err(|e| format!("step {step_id} lookup failed: {e}"))?;
            match owner {
                None => return Err(format!("step {step_id} is not in resources.mission_steps")),
                Some(owner) if owner != m.mission_id => {
                    return Err(format!(
                        "step {step_id} belongs to mission {owner}, not mission {}",
                        m.mission_id
                    ))
                }
                Some(_) => {}
            }
            let steps = ordered_steps(pool, m.mission_id).await?;
            let before: Vec<i32> = steps
                .iter()
                .copied()
                .take_while(|&s| s != step_id)
                .collect();
            Ok(MissionRow {
                mission_id: m.mission_id,
                status: MISSION_ACTIVE,
                current_step_id: Some(step_id),
                completed_objective_ids: objectives_of(pool, &before).await?,
                completed_step_ids: before,
                // A fixture puts the mission back to a first run, whatever
                // the character did before (the route forward refuses a
                // completed mission at its repeat cap).
                repeats: 0,
            })
        }
        StagedMissionState::Completed => {
            let steps = ordered_steps(pool, m.mission_id).await?;
            if steps.is_empty() {
                return Err(format!(
                    "mission {} has no steps in resources.mission_steps",
                    m.mission_id
                ));
            }
            Ok(MissionRow {
                mission_id: m.mission_id,
                status: MISSION_COMPLETED,
                // `MissionInstance::complete` takes the step.
                current_step_id: None,
                completed_objective_ids: objectives_of(pool, &steps).await?,
                completed_step_ids: steps,
                // What `MissionInstance::complete` leaves after a first
                // completion: it bumps `repeats` from 0 to 1.
                repeats: 1,
            })
        }
    }
}

/// The mission's steps in play order.
async fn ordered_steps(pool: &PgPool, mission_id: i32) -> Result<Vec<i32>, String> {
    sqlx::query_scalar(
        "SELECT step_id FROM resources.mission_steps WHERE mission_id = $1 \
         ORDER BY index ASC NULLS LAST, step_id ASC",
    )
    .bind(mission_id)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("mission {mission_id} steps lookup failed: {e}"))
}

/// Every objective of `steps`, in step order and then objective id.
async fn objectives_of(pool: &PgPool, steps: &[i32]) -> Result<Vec<i32>, String> {
    if steps.is_empty() {
        return Ok(Vec::new());
    }
    let rows: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT step_id, objective_id FROM resources.mission_objectives \
         WHERE step_id = ANY($1) ORDER BY objective_id",
    )
    .bind(steps)
    .fetch_all(pool)
    .await
    .map_err(|e| format!("objectives lookup failed: {e}"))?;
    let mut by_step: HashMap<i32, Vec<i32>> = HashMap::new();
    for (step, objective) in rows {
        by_step.entry(step).or_default().push(objective);
    }
    Ok(steps
        .iter()
        .flat_map(|s| by_step.remove(s).unwrap_or_default())
        .collect())
}

/// What [`confirm_written`] reads back: status, current step, completed
/// steps, completed objectives, repeats.
type StoredRow = (i32, Option<i32>, Vec<i32>, Vec<i32>, i32);

/// The row is as built (or gone, for a not-active one), arrays included: a
/// stale row whose status happens to match must not pass.
async fn confirm_written(pool: &PgPool, player_id: i32, row: &MissionRow) -> Result<(), String> {
    let stored: Option<StoredRow> = sqlx::query_as(
        "SELECT status, current_step_id, completed_step_ids, completed_objective_ids, repeats \
         FROM sgw_mission WHERE player_id = $1 AND mission_id = $2",
    )
    .bind(player_id)
    .bind(row.mission_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| format!("mission {} read-back failed: {e}", row.mission_id))?;
    let want = (row.status != MISSION_NOT_ACTIVE).then(|| {
        (
            i32::from(row.status),
            row.current_step_id,
            row.completed_step_ids.clone(),
            row.completed_objective_ids.clone(),
            row.repeats,
        )
    });
    if stored != want {
        return Err(format!(
            "mission {} was not written: stored {stored:?}, wanted {want:?}",
            row.mission_id
        ));
    }
    Ok(())
}
