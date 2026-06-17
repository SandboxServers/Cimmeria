//! BaseApp-side handler for the `gmSpawnByCmd` template lookup.
//!
//! [`handle_gm_spawn_npc`] is the base half of the `gmSpawnByCmd` round-trip:
//! it queries `resources.entity_templates` for the requested template,
//! materializes a [`SpawnRecord`], and ships it back to the cell via
//! `BaseToCellMsg::GmSpawnNpcReady`. The cell can't build a `SpawnRecord` for
//! an arbitrary `template_id` (it has no template cache), so the base owns the
//! DB query and the record construction.

use tokio::sync::mpsc;

use sqlx::PgPool;
use std::sync::Arc;

use crate::cell::messages::BaseToCellMsg;
use crate::cell::spawner::SpawnRecord;

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
#[allow(clippy::too_many_arguments)]
pub async fn handle_gm_spawn_npc(
    entity_id: u32,
    template_id: i32,
    space_id: u32,
    world_name: String,
    position: [f32; 3],
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(
                entity_id,
                template_id,
                "GmSpawnNpc: no DB pool, cannot resolve template"
            );
            return;
        }
    };

    let record =
        match load_spawn_record_for_template(pool, template_id, &world_name, position).await {
            Ok(Some(r)) => r,
            Ok(None) => {
                tracing::warn!(
                    entity_id,
                    template_id,
                    "GmSpawnNpc: template not found in entity_templates — dropping spawn"
                );
                return;
            }
            Err(e) => {
                tracing::error!(
                    entity_id,
                    template_id,
                    "GmSpawnNpc: entity_templates query failed: {e}"
                );
                return;
            }
        };

    tracing::info!(
        entity_id,
        template_id,
        space_id,
        template_name = %record.template_name,
        "GmSpawnNpc: template resolved, replying to cell"
    );

    if let Some(tx) = cell_tx {
        if let Err(e) = tx
            .send(BaseToCellMsg::GmSpawnNpcReady {
                record: Box::new(record),
                space_id,
            })
            .await
        {
            tracing::warn!(
                entity_id,
                template_id,
                space_id,
                "GmSpawnNpc: GmSpawnNpcReady send to cell failed: {e}"
            );
        }
    } else {
        tracing::warn!(
            entity_id,
            template_id,
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
async fn load_spawn_record_for_template(
    pool: &PgPool,
    template_id: i32,
    world_name: &str,
    position: [f32; 3],
) -> Result<Option<SpawnRecord>, sqlx::Error> {
    use sqlx::Row;

    let row_opt = sqlx::query(
        "SELECT t.template_id, t.template_name, t.class, t.static_mesh, t.body_set, \
                t.components, t.flags, t.interaction_type, t.event_set_id, t.level, \
                t.alignment, t.faction, t.name_id, t.speaker_id, \
                t.static_interaction_sets, t.has_dynamic_properties, \
                t.loot_table_id, \
                t.patrol_path_id, \
                COALESCE(t.patrol_point_delay, 2.0) AS patrol_point_delay, \
                COALESCE(t.wander_radius, 0.0) AS wander_radius, \
                COALESCE(t.wander_min_dwell_secs, 3.0) AS wander_min_dwell_secs, \
                COALESCE(t.wander_max_dwell_secs, 8.0) AS wander_max_dwell_secs, \
                COALESCE(t.follow_min_distance, 2.0) AS follow_min_distance, \
                COALESCE(t.follow_max_distance, 5.0) AS follow_max_distance, \
                t.respawn_secs, \
                COALESCE( \
                  (SELECT array_agg(asa.ability_id ORDER BY asa.ability_id) \
                   FROM resources.ability_set_abilities asa \
                   WHERE asa.ability_set_id = t.ability_set_id), \
                  ARRAY[]::int[] \
                ) AS ability_ids \
         FROM resources.entity_templates t \
         WHERE t.template_id = $1",
    )
    .bind(template_id)
    .fetch_optional(pool)
    .await?;

    let row = match row_opt {
        Some(r) => r,
        None => return Ok(None),
    };

    // Patrol points, if any, follow the same `point_set_points` lookup the
    // spawnlist loader uses. For a GM spawn we resolve the single template's
    // patrol_path_id (NULL → empty path).
    let patrol_path = match row
        .try_get::<Option<i32>, _>("patrol_path_id")
        .ok()
        .flatten()
    {
        Some(path_id) => crate::cell::spawner::load_patrol_points(pool, &[path_id])
            .await?
            .remove(&path_id)
            .unwrap_or_default(),
        None => Vec::new(),
    };

    let record = SpawnRecord {
        // Spawn-instance fields sourced from the GM command, not a spawnlist
        // row. spawn_id = -1 marks this as a non-DB (GM) spawn; tag = None and
        // heading = 0 match the "drop it here facing forward" semantics.
        spawn_id: -1,
        world_name: world_name.to_string(),
        x: position[0],
        y: position[1],
        z: position[2],
        heading: 0.0,
        tag: None,
        // Template-derived fields.
        template_id: row.get("template_id"),
        template_name: row.get("template_name"),
        class: row.get("class"),
        static_mesh: row.get("static_mesh"),
        body_set: row.get("body_set"),
        components: row.get("components"),
        flags: row.get("flags"),
        interaction_type: row.get("interaction_type"),
        event_set_id: row.get("event_set_id"),
        level: row.get("level"),
        alignment: row.get("alignment"),
        faction: row.get("faction"),
        name_id: row.get("name_id"),
        speaker_id: row.get("speaker_id"),
        static_interaction_sets: row.get("static_interaction_sets"),
        has_dynamic_properties: row.get("has_dynamic_properties"),
        loot_table_id: row.get("loot_table_id"),
        // GM spawns are placeable mobs, not stationary props.
        is_stationary: false,
        ability_ids: row.get::<Vec<i32>, _>("ability_ids"),
        respawn_secs: crate::cell::spawner::normalize_respawn_secs(
            row.try_get::<Option<i32>, _>("respawn_secs").ok().flatten(),
        ),
        patrol_path,
        patrol_point_delay_secs: row.get::<f32, _>("patrol_point_delay"),
        wander_radius: row.get::<f32, _>("wander_radius"),
        wander_min_dwell_secs: row.get::<f32, _>("wander_min_dwell_secs"),
        wander_max_dwell_secs: row.get::<f32, _>("wander_max_dwell_secs"),
        follow_min_distance: row.get::<f32, _>("follow_min_distance"),
        follow_max_distance: row.get::<f32, _>("follow_max_distance"),
    };

    Ok(Some(record))
}

#[cfg(test)]
mod tests {
    //! Live-DB guard for the `gmSpawnByCmd` base half. Self-skips when
    //! `DATABASE_URL` is unset via `require_db_or_skip!`. Drives the real
    //! `handle_gm_spawn_npc` (entity_templates query → `SpawnRecord` →
    //! `GmSpawnNpcReady`) against a seeded template, and the not-found path.

    use super::*;
    use crate::test_support::require_db_or_skip;
    use sqlx::Row;

    /// A real template (resolved from the seed, not hard-coded) must produce a
    /// `GmSpawnNpcReady` whose template-derived fields match the row and whose
    /// spawn-instance fields (position, world, spawn_id) come from the command.
    /// Reverting the query or the record construction trips this.
    #[tokio::test]
    async fn gm_spawn_resolves_real_template_and_replies() {
        let pool = require_db_or_skip!();
        // Pick any fully-populated template — the handler reads template_name /
        // class / body_set as NOT NULL, so filter to a row it can materialize.
        let row = sqlx::query(
            "SELECT template_id, template_name FROM resources.entity_templates \
             WHERE template_name IS NOT NULL AND class IS NOT NULL AND body_set IS NOT NULL \
             ORDER BY template_id LIMIT 1",
        )
        .fetch_one(&pool)
        .await
        .expect("seed must contain at least one fully-populated entity_template");
        let template_id: i32 = row.get("template_id");
        let template_name: String = row.get("template_name");

        let (cell_tx, mut cell_rx) = mpsc::channel(8);
        let db_pool = Some(Arc::new(pool.clone()));

        handle_gm_spawn_npc(
            42, // entity_id
            template_id,
            5, // space_id
            "Castle".to_string(),
            [10.0, 20.0, 30.0],
            &db_pool,
            &Some(cell_tx),
        )
        .await;

        match cell_rx.try_recv().expect("must reply GmSpawnNpcReady") {
            BaseToCellMsg::GmSpawnNpcReady { record, space_id } => {
                assert_eq!(space_id, 5, "space_id echoes the request");
                assert_eq!(record.template_id, template_id, "template-derived id");
                assert_eq!(record.template_name, template_name, "template-derived name");
                assert_eq!(
                    [record.x, record.y, record.z],
                    [10.0, 20.0, 30.0],
                    "position from command"
                );
                assert_eq!(record.world_name, "Castle", "world from command");
                assert_eq!(record.spawn_id, -1, "GM spawns are non-DB (spawn_id -1)");
                assert!(record.tag.is_none(), "GM spawn has no spawnlist tag");
            }
            _ => panic!("expected BaseToCellMsg::GmSpawnNpcReady"),
        }
    }

    /// A template id that doesn't exist must drop the spawn — no
    /// `GmSpawnNpcReady` reply (so the cell never spawns a bogus mob).
    #[tokio::test]
    async fn gm_spawn_missing_template_sends_nothing() {
        let pool = require_db_or_skip!();
        let (cell_tx, mut cell_rx) = mpsc::channel(8);
        let db_pool = Some(Arc::new(pool.clone()));

        handle_gm_spawn_npc(
            42,
            0x7FFF_FFF0, // template id that won't exist in the seed
            5,
            "Castle".to_string(),
            [0.0; 3],
            &db_pool,
            &Some(cell_tx),
        )
        .await;

        assert!(
            cell_rx.try_recv().is_err(),
            "missing template must not reply GmSpawnNpcReady"
        );
    }
}
