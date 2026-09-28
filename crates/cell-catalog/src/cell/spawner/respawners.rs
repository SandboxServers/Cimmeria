//! Respawner definitions.
//!
//! Defeat-window respawn locations loaded from `resources.respawners`.
//! The client picks one and sends back the chosen `respawner_id` via
//! `callForAid`.

use sqlx::PgPool;

/// A respawner location loaded from the database.
///
/// Players see a list of these in the Defeat Window (onBeginAidWait).
/// The client sends back the chosen `respawner_id` via `callForAid`.
#[derive(Debug, Clone)]
pub struct RespawnerDef {
    pub respawner_id: i32,
    pub world_name: String,
    pub name: String,
    pub pos: [f32; 3],
}

/// Respawners the Defeat Window offers a player in `world`: today every
/// respawner registered for that world. A per-player known-respawner filter
/// would narrow this one function, so the offer and the check narrow together.
///
/// Two callers must agree: `send_begin_aid_wait` builds the Defeat Window
/// list from it, and the `callForAid` dispatch arm accepts a positive
/// client-supplied `respawner_id` only if it is in this set. An id from
/// another world is never offered, so the arm refuses it before
/// `handle_respawn` can take its cross-world GateTravel branch.
pub fn offered_in_world<'a>(
    all: &'a [RespawnerDef],
    world: &'a str,
) -> impl Iterator<Item = &'a RespawnerDef> + 'a {
    all.iter().filter(move |r| r.world_name == world)
}

/// Mirror of the SQL projection for `query_as`. The `pos` array on
/// `RespawnerDef` doesn't have a direct column equivalent, so we map the
/// three position columns onto separate fields here and assemble the array
/// in the conversion below. Using `FromRow` rather than manual `Row::get`
/// turns column-name and type mismatches into compile-/load-time errors
/// with proper diagnostics instead of mid-query runtime panics.
#[derive(sqlx::FromRow)]
struct RespawnerRow {
    respawner_id: i32,
    world_name: String,
    name: String,
    pos_x: f32,
    pos_y: f32,
    pos_z: f32,
}

impl From<RespawnerRow> for RespawnerDef {
    fn from(r: RespawnerRow) -> Self {
        RespawnerDef {
            respawner_id: r.respawner_id,
            world_name: r.world_name,
            name: r.name,
            pos: [r.pos_x, r.pos_y, r.pos_z],
        }
    }
}

/// Load respawner definitions from the database.
///
/// Joins `resources.respawners` with `resources.worlds` to get world names.
pub async fn load_respawners(pool: &PgPool) -> Result<Vec<RespawnerDef>, sqlx::Error> {
    let rows = sqlx::query_as::<_, RespawnerRow>(
        "SELECT r.respawner_id, w.world AS world_name, r.name, \
                r.pos_x, r.pos_y, r.pos_z \
         FROM resources.respawners r \
         JOIN resources.worlds w ON w.world_id = r.world_id \
         ORDER BY r.respawner_id",
    )
    .fetch_all(pool)
    .await?;

    let respawners: Vec<RespawnerDef> = rows.into_iter().map(RespawnerDef::from).collect();

    tracing::info!(count = respawners.len(), "Loaded respawner definitions");
    Ok(respawners)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(respawner_id: i32, world_name: &str) -> RespawnerDef {
        RespawnerDef {
            respawner_id,
            world_name: world_name.to_string(),
            name: format!("r{respawner_id}"),
            pos: [1.0, 2.0, 3.0],
        }
    }

    /// The Defeat Window offer and the `callForAid` check both read this set,
    /// so a respawner registered for another world must never be in it.
    #[test]
    fn offered_in_world_keeps_only_that_worlds_respawners() {
        let all = [def(1, "Castle"), def(2, "Agnos"), def(3, "Castle")];
        let ids: Vec<i32> = offered_in_world(&all, "Castle")
            .map(|r| r.respawner_id)
            .collect();
        assert_eq!(ids, vec![1, 3]);
        assert_eq!(offered_in_world(&all, "Harset").count(), 0);
    }
}
