//! BaseApp-side handler for the `gmSpawnByCmd` template lookup.
//!
//! [`handle_gm_spawn_npc`] is the base half of the `gmSpawnByCmd` round-trip:
//! it queries `resources.entity_templates` for the requested template,
//! materializes a [`SpawnRecord`], and ships it back to the cell via
//! `BaseToCellMsg::GmSpawnNpcReady`. The base owns the DB query because it
//! owns the pool at request time.
//!
//! The row -> `SpawnRecord` mapping itself is **not** owned here: both the
//! SELECT and the field mapping come from
//! [`crate::cell::spawner::entity_template_select`] /
//! [`crate::cell::spawner::build_prototype`], shared with the cell's startup
//! template cache so the two can never drift on a schema change. Only the
//! GM-specific overrides (position, world, one-shot respawn) live in this
//! file.

use tokio::sync::mpsc;

use sqlx::PgPool;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use crate::base::gm_feedback::send_gm_feedback_to_client;
use crate::base::session_identity::entity_name_for;
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use crate::cell::spawner::{build_prototype, entity_template_select, SpawnRecord};

/// Handle `gmSpawnByCmd` from CellService — look up the requested template in
/// `resources.entity_templates`, build a [`SpawnRecord`] from it (filling the
/// spawn-specific position/world from the message), and reply to the cell with
/// `BaseToCellMsg::GmSpawnNpcReady`.
///
/// This query reads `entity_templates` ONLY (no `spawnlist`/`worlds` join — the
/// spawn instance is GM-created, not DB-seeded). It mirrors
/// `cell::spawner::npcs::load_spawns_from_db`'s column→field mapping for the
/// template-derived fields and the patrol/wander default conventions, but
/// sources the position from the command rather than a spawnlist row.
#[tracing::instrument(
    name = "gm_spawn.gm_spawn_npc",
    level = "info",
    skip_all,
    fields(entity_id, template_id, space_id)
)]
pub async fn handle_gm_spawn_npc(
    entity_id: u32,
    template_id: i32,
    space_id: u32,
    world_name: String,
    position: [f32; 3],
    heading: f32,
    batch: Option<u32>,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Where a failure line goes: a batched copy (`.spawnrandom`, CH-13)
    // reports to the cell, which folds it into the batch's one summary line;
    // a single spawn tells the GM directly, as before.
    let fail = |reason: String| async move {
        match (batch, cell_tx) {
            (Some(batch), Some(tx)) => {
                if let Err(e) = tx
                    .send(BaseToCellMsg::GmSpawnNpcFailed {
                        requester_entity_id: entity_id,
                        batch,
                        template_id,
                        reason,
                    })
                    .await
                {
                    // The batch will report this copy as timed out.
                    tracing::warn!(
                        entity_id,
                        entity_name = entity_name_for(connected, entity_to_addr, entity_id),
                        template_id,
                        template_name = cimmeria_names::book().template(template_id),
                        batch,
                        "GmSpawnNpc: GmSpawnNpcFailed send to cell failed: {e}"
                    );
                }
            }
            _ => {
                send_gm_feedback_to_client(
                    entity_id,
                    &format!("spawn failed: {reason}"),
                    transport,
                    connected,
                    entity_to_addr,
                )
                .await;
            }
        }
    };

    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(
                entity_id,
                entity_name = entity_name_for(connected, entity_to_addr, entity_id),
                template_id,
                template_name = cimmeria_names::book().template(template_id),
                "GmSpawnNpc: no DB pool, cannot resolve template"
            );
            // A single spawn stays silent here (unchanged); a batch must hear
            // about every copy or its summary waits for the timeout.
            if batch.is_some() {
                fail("no database to resolve the template".to_string()).await;
            }
            return;
        }
    };

    let record =
        match load_spawn_record_for_template(pool, template_id, &world_name, position, heading)
            .await
        {
            Ok(Some(r)) => r,
            Ok(None) => {
                tracing::warn!(
                    entity_id,
                    entity_name = entity_name_for(connected, entity_to_addr, entity_id),
                    template_id,
                    template_name = cimmeria_names::book().template(template_id),
                    "GmSpawnNpc: template not found in entity_templates — dropping spawn"
                );
                // Definitive failure feedback: the GM asked for a template the
                // DB doesn't have. The success path stays silent here — the
                // cell confirms the actual spawn once `GmSpawnNpcReady` lands.
                //
                // Command-neutral wording: this same round-trip serves the native
                // `gmSpawnByCmd` and the dot-console `.spawn` / `.spawnrandom`, so
                // naming one of them here would misreport which command the GM
                // actually typed.
                fail(format!("template {template_id} not found")).await;
                return;
            }
            Err(e) => {
                tracing::error!(
                    entity_id,
                    entity_name = entity_name_for(connected, entity_to_addr, entity_id),
                    template_id,
                    template_name = cimmeria_names::book().template(template_id),
                    "GmSpawnNpc: entity_templates query failed: {e}"
                );
                // Distinct from the "template not found" line above: this is
                // a DB outage or row-decode failure, not evidence the
                // template id is bad. The detailed error stays server-side.
                fail(format!("could not load template {template_id}")).await;
                return;
            }
        };

    tracing::info!(
        entity_id,
        entity_name = entity_name_for(connected, entity_to_addr, entity_id),
        template_id,
        space_id,
        world = %world_name,
        template_name = %record.template_name,
        "GmSpawnNpc: template resolved, replying to cell"
    );

    if let Some(tx) = cell_tx {
        if let Err(e) = tx
            .send(BaseToCellMsg::GmSpawnNpcReady {
                record: Box::new(record),
                space_id,
                requester_entity_id: entity_id,
                batch,
            })
            .await
        {
            tracing::warn!(
                entity_id,
                entity_name = entity_name_for(connected, entity_to_addr, entity_id),
                template_id,
                template_name = cimmeria_names::book().template(template_id),
                space_id,
                world = %world_name,
                "GmSpawnNpc: GmSpawnNpcReady send to cell failed: {e}"
            );
        }
        // Success path: do NOT feed back here. The cell sends the definitive
        // "spawned npc <id>" line from its `GmSpawnNpcReady` handler once the
        // NPC is actually placed in the space.
    } else {
        tracing::warn!(
            entity_id,
            entity_name = entity_name_for(connected, entity_to_addr, entity_id),
            template_id,
            template_name = cimmeria_names::book().template(template_id),
            "GmSpawnNpc: no cell channel — spawn dropped"
        );
    }
}

/// Query `resources.entity_templates` by `template_id` and build a
/// `SpawnRecord` whose spawn-specific fields (position, world, spawn_id, tag)
/// come from the GM command rather than a `spawnlist` row.
///
/// Returns `Ok(None)` when the template doesn't exist. The ability-id bucket
/// is loaded via the same correlated-subquery shape as the spawnlist loader so
/// a GM-spawned mob is armed identically to a seeded one.
pub(crate) async fn load_spawn_record_for_template(
    pool: &PgPool,
    template_id: i32,
    world_name: &str,
    position: [f32; 3],
    heading: f32,
) -> Result<Option<SpawnRecord>, sqlx::Error> {
    use sqlx::Row;

    let row_opt = sqlx::query(entity_template_select!(" WHERE t.template_id = $1"))
        .bind(template_id)
        .fetch_optional(pool)
        .await?;

    let row = match row_opt {
        Some(r) => r,
        None => return Ok(None),
    };

    // Patrol points, if any, follow the same `point_set_points` lookup the
    // spawnlist loader uses. For a GM spawn we resolve the single template's
    // patrol_path_id (NULL → empty path). The resolved map is handed to the
    // shared mapper, which does the id → path lookup itself.
    let patrol_path_id: Option<i32> = row.try_get("patrol_path_id")?;
    let patrol_paths = match patrol_path_id {
        Some(path_id) => crate::cell::spawner::load_patrol_points(pool, &[path_id]).await?,
        None => std::collections::HashMap::new(),
    };

    // Template-derived fields come from the shared mapper so this handler
    // and the cell's startup template cache can never drift apart on a
    // schema change (PR #662 review, finding 3). Everything below the
    // mapper call is what makes a GM spawn a GM spawn.
    let mut record = build_prototype(&row, &patrol_paths)?;

    // Spawn-instance fields sourced from the GM command, not a spawnlist
    // row. `spawn_id = -1` and `tag = None` are already the prototype's
    // placeholders; `heading` comes from the command (the dot-console sends
    // the caller's own facing, matching legacy `Resource.spawnEntity`'s
    // `player.rotation`; the native `gmSpawnByCmd` has no rotation argument
    // and sends 0.0).
    record.world_name = world_name.to_string();
    record.x = position[0];
    record.y = position[1];
    record.z = position[2];
    record.heading = heading;
    // GM spawns are one-shot: force `respawn_secs = None` so a `spawn_id = -1`
    // (non-DB) instance is never handed to the respawner. The prototype
    // carries the template's value verbatim, so this override is load-bearing.
    record.respawn_secs = None;

    Ok(Some(record))
}

#[cfg(test)]
#[path = "gm_spawn_tests.rs"]
mod tests;
