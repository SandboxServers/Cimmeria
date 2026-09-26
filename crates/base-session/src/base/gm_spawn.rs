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
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
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
        match load_spawn_record_for_template(pool, template_id, &world_name, position, heading)
            .await
        {
            Ok(Some(r)) => r,
            Ok(None) => {
                tracing::warn!(
                    entity_id,
                    template_id,
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
                send_gm_feedback_to_client(
                    entity_id,
                    &format!("spawn failed: template {template_id} not found"),
                    transport,
                    connected,
                    entity_to_addr,
                )
                .await;
                return;
            }
            Err(e) => {
                tracing::error!(
                    entity_id,
                    template_id,
                    "GmSpawnNpc: entity_templates query failed: {e}"
                );
                // Distinct from the "template not found" line above: this is
                // a DB outage or row-decode failure, not evidence the
                // template id is bad. The detailed error stays server-side.
                send_gm_feedback_to_client(
                    entity_id,
                    &format!("spawn failed: could not load template {template_id}"),
                    transport,
                    connected,
                    entity_to_addr,
                )
                .await;
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
                requester_entity_id: entity_id,
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
        // Success path: do NOT feed back here. The cell sends the definitive
        // "spawned npc <id>" line from its `GmSpawnNpcReady` handler once the
        // NPC is actually placed in the space.
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
mod tests {
    //! Live-DB guard for the `gmSpawnByCmd` base half. Self-skips when
    //! `DATABASE_URL` is unset via `require_db_or_skip!`. Drives the real
    //! `handle_gm_spawn_npc` (entity_templates query → `SpawnRecord` →
    //! `GmSpawnNpcReady`) against a seeded template, and the not-found path.

    use super::*;
    use crate::test_support::{require_db_or_skip, TestTransport};
    use sqlx::Row;

    /// Empty client-IO triple: the GM-feedback / not-found push is skipped
    /// (`send_to_witness_reliable` no-ops on an empty `entity_to_addr`), but the
    /// DB query + `GmSpawnNpcReady` reply still run — which is what these tests
    /// assert.
    fn empty_io() -> (
        Arc<dyn Transport>,
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) {
        let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
        let connected = Arc::new(Mutex::new(HashMap::new()));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
        (transport, connected, entity_to_addr)
    }

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
        let (transport, connected, entity_to_addr) = empty_io();

        handle_gm_spawn_npc(
            42, // entity_id
            template_id,
            5, // space_id
            "Castle".to_string(),
            [10.0, 20.0, 30.0],
            // A distinctive non-zero yaw: the record used to hardcode
            // `heading: 0.0`, so anything that drops the command's heading on
            // the way into the `SpawnRecord` trips the assertion below.
            1.25,
            &db_pool,
            &Some(cell_tx),
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        match cell_rx.try_recv().expect("must reply GmSpawnNpcReady") {
            BaseToCellMsg::GmSpawnNpcReady {
                record,
                space_id,
                requester_entity_id,
            } => {
                assert_eq!(space_id, 5, "space_id echoes the request");
                assert_eq!(requester_entity_id, 42, "requester echoes the entity_id");
                assert_eq!(record.template_id, template_id, "template-derived id");
                assert_eq!(record.template_name, template_name, "template-derived name");
                assert_eq!(
                    [record.x, record.y, record.z],
                    [10.0, 20.0, 30.0],
                    "position from command"
                );
                assert_eq!(record.world_name, "Castle", "world from command");
                // The dot-console `.spawn` places an entity at the caller's
                // *facing*, not an arbitrary one — the heading has to survive
                // the cell→base→cell round-trip into the record that
                // `spawn_npc_from_record_into` turns into
                // `direction = (0, heading, 0)`.
                assert_eq!(
                    record.heading, 1.25,
                    "heading from command (a hardcoded 0.0 here silently faces every \
                     .spawn'd entity the same way)"
                );
                assert_eq!(record.spawn_id, -1, "GM spawns are non-DB (spawn_id -1)");
                assert!(record.tag.is_none(), "GM spawn has no spawnlist tag");
            }
            _ => panic!("expected BaseToCellMsg::GmSpawnNpcReady"),
        }
    }

    /// A template row whose non-Option column (`template_name`) is NULL must be
    /// dropped gracefully: `load_spawn_record_for_template` reads it via
    /// `try_get::<String, _>` + `?`, so a NULL decodes to an `sqlx::Error` →
    /// `Err` arm → no `GmSpawnNpcReady` reply (and crucially, no panic that
    /// would take down the base task).
    ///
    /// `template_name` is schema-NOT-NULL, so the constraint is dropped for the
    /// duration of the INSERT and restored in a `defer`-style guaranteed
    /// teardown. Live-DB tests run serialized (`--test-threads=1`), so the
    /// transient constraint relaxation can't race another test. The sentinel
    /// row is deleted on the way out.
    ///
    /// Reverting the `try_get` + `?` discipline back to `row.get` would turn
    /// this graceful drop into a panic — the handler would unwind instead of
    /// returning, and this test (which expects a clean no-reply) would fail.
    #[tokio::test]
    async fn gm_spawn_malformed_template_drops_gracefully() {
        let pool = require_db_or_skip!();
        // Sentinel template id in the 0x7000_xxxx range, well within i32.
        const SENTINEL_TEMPLATE_ID: i32 = 0x7000_4242;

        // Teardown: delete the sentinel and restore NOT NULL. The handler under
        // test returns gracefully (it does not panic — the whole point), so a
        // straight-line teardown after the run is sufficient; live-DB tests run
        // serialized so the transient constraint relaxation can't race.
        async fn teardown(pool: &sqlx::PgPool) {
            let _ = sqlx::query("DELETE FROM resources.entity_templates WHERE template_id = $1")
                .bind(SENTINEL_TEMPLATE_ID)
                .execute(pool)
                .await;
            let _ = sqlx::query(
                "ALTER TABLE resources.entity_templates ALTER COLUMN template_name SET NOT NULL",
            )
            .execute(pool)
            .await;
        }

        // Clean any leftover sentinel from a previously-aborted run before we
        // touch the constraint.
        sqlx::query("DELETE FROM resources.entity_templates WHERE template_id = $1")
            .bind(SENTINEL_TEMPLATE_ID)
            .execute(&pool)
            .await
            .expect("pre-clean sentinel");

        // Relax NOT NULL just long enough to insert the malformed row.
        sqlx::query(
            "ALTER TABLE resources.entity_templates ALTER COLUMN template_name DROP NOT NULL",
        )
        .execute(&pool)
        .await
        .expect("drop NOT NULL on template_name");

        // Insert the sentinel with template_name = NULL. body_set and class are
        // still NOT NULL so we provide them; only the column under test is NULL.
        if let Err(e) = sqlx::query(
            "INSERT INTO resources.entity_templates \
                (template_id, template_name, class, body_set) \
             VALUES ($1, NULL, 'TestClass', 'TestBodySet')",
        )
        .bind(SENTINEL_TEMPLATE_ID)
        .execute(&pool)
        .await
        {
            teardown(&pool).await;
            panic!("failed to insert malformed sentinel template: {e}");
        }

        let (cell_tx, mut cell_rx) = mpsc::channel(8);
        let db_pool = Some(Arc::new(pool.clone()));
        let (transport, connected, entity_to_addr) = empty_io();

        // Run the handler against the malformed template. `try_get` + `?` turns
        // the NULL into an Err arm → graceful drop (no reply, no panic).
        handle_gm_spawn_npc(
            42,
            SENTINEL_TEMPLATE_ID,
            5,
            "Castle".to_string(),
            [0.0; 3],
            0.0,
            &db_pool,
            &Some(cell_tx),
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        let got_reply = cell_rx.try_recv().is_ok();
        teardown(&pool).await;

        assert!(
            !got_reply,
            "a NULL non-Option column must decode-error → graceful drop, \
             not a GmSpawnNpcReady reply"
        );
    }

    /// A template id that doesn't exist must drop the spawn — no
    /// `GmSpawnNpcReady` reply (so the cell never spawns a bogus mob).
    #[tokio::test]
    async fn gm_spawn_missing_template_sends_nothing() {
        let pool = require_db_or_skip!();
        let (cell_tx, mut cell_rx) = mpsc::channel(8);
        let db_pool = Some(Arc::new(pool.clone()));
        let (transport, connected, entity_to_addr) = empty_io();

        handle_gm_spawn_npc(
            42,
            0x7FFF_FFF0, // template id that won't exist in the seed
            5,
            "Castle".to_string(),
            [0.0; 3],
            0.0,
            &db_pool,
            &Some(cell_tx),
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        assert!(
            cell_rx.try_recv().is_err(),
            "missing template must not reply GmSpawnNpcReady"
        );
    }
}
