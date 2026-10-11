//! The fixture's checks against the seed (lab-fixtures FX-04): the same
//! lookups the base precheck runs before its first write, run at stage time.

use sqlx::PgPool;

use cimmeria_services::base::lab_fixture::{StagedMission, StagedMissionState};

use super::{Refusal, StageFixtureArgs};

/// The checks against the seed, the same ones the base precheck runs
/// before its first write (`base-world-entry`'s `lab_fixture::precheck`
/// and `plan_missions`), plus the item design ids FX-03 checks.
pub(super) async fn check_seed(
    pool: &PgPool,
    args: &StageFixtureArgs,
    missions: &[StagedMission],
) -> Result<(), Refusal> {
    if let Some(world) = args.world.as_deref() {
        // The base precheck's lookup: `sgw_player.world_location` is a
        // foreign key into `resources.worlds`. (`.gotolocation` resolves
        // through the cell's in-memory world catalog, loaded from the same
        // table; this endpoint has no cell round trip for it.)
        let known: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM resources.worlds WHERE world = $1)")
                .bind(world)
                .fetch_one(pool)
                .await
                .map_err(|e| Refusal::lookup_failed(format_args!("world {world}"), e))?;
        if !known {
            return Err(Refusal::caller(
                "world",
                format!("world {world} is not in resources.worlds"),
            ));
        }
    }
    for (i, m) in missions.iter().enumerate() {
        let known = exists_id(
            pool,
            "SELECT EXISTS (SELECT 1 FROM resources.missions WHERE mission_id = $1)",
            m.mission_id,
        )
        .await?;
        if !known {
            return Err(Refusal::caller(
                "mission",
                format!(
                    "missions[{i}]: mission {} is not in resources.missions",
                    m.mission_id
                ),
            ));
        }
        match m.state {
            StagedMissionState::Active { step_id } => {
                let owner: Option<i32> = sqlx::query_scalar(
                    "SELECT mission_id FROM resources.mission_steps WHERE step_id = $1",
                )
                .bind(step_id)
                .fetch_optional(pool)
                .await
                .map_err(|e| {
                    Refusal::lookup_failed(format_args!("missions[{i}]: step {step_id}"), e)
                })?;
                match owner {
                    None => {
                        return Err(Refusal::caller(
                            "step",
                            format!(
                                "missions[{i}].step: step {step_id} is not in resources.mission_steps"
                            ),
                        ))
                    }
                    Some(owner) if owner != m.mission_id => {
                        return Err(Refusal::caller(
                            "step",
                            format!(
                                "missions[{i}].step: step {step_id} belongs to mission {owner}, not mission {}",
                                m.mission_id
                            ),
                        ))
                    }
                    Some(_) => {}
                }
            }
            StagedMissionState::Completed => {
                let has_steps = exists_id(
                    pool,
                    "SELECT EXISTS (SELECT 1 FROM resources.mission_steps WHERE mission_id = $1)",
                    m.mission_id,
                )
                .await?;
                if !has_steps {
                    return Err(Refusal::caller(
                        "mission",
                        format!(
                            "missions[{i}]: mission {} has no steps to complete",
                            m.mission_id
                        ),
                    ));
                }
            }
            StagedMissionState::NotActive => {}
        }
    }
    for (i, it) in args.items.iter().enumerate() {
        let known = exists_id(
            pool,
            "SELECT EXISTS (SELECT 1 FROM resources.items WHERE item_id = $1)",
            it.id,
        )
        .await?;
        if !known {
            return Err(Refusal::caller(
                "item",
                format!("items[{i}]: item {} is not in resources.items", it.id),
            ));
        }
    }
    Ok(())
}

async fn exists_id(pool: &PgPool, sql: &'static str, id: i32) -> Result<bool, Refusal> {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .map_err(|e| Refusal::lookup_failed(format_args!("seed id {id}"), e))
}
