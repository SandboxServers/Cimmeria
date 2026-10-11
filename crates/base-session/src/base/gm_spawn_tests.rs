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
async fn live_db_gm_spawn_resolves_real_template_and_replies() {
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
        // A `.spawnrandom` batch id (CH-13): the cell matches the reply
        // to its batch by this, so it has to come back unchanged.
        Some(7),
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
            batch,
        } => {
            assert_eq!(batch, Some(7), "batch id echoes the request");
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

/// A GM `.spawn` of a Visual NPC Lineup template (DA-10) keeps its
/// literal nameplate: the record carries `entity_templates.display_name`,
/// so the AoI introduction sends `onBeingNameUpdate` exactly as for the
/// seeded actor. Revert proof: drop `t.display_name` from
/// `entity_template_select!` or from `build_prototype` and the record
/// comes back without it (or not at all).
#[tokio::test]
async fn live_db_gm_spawn_keeps_the_lineup_display_name() {
    let pool = require_db_or_skip!();
    let row = sqlx::query(
        "SELECT template_id, display_name FROM resources.entity_templates \
         WHERE display_name IS NOT NULL ORDER BY template_id LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .expect("the DA-10 lineup seeds templates with a display_name");
    let template_id: i32 = row.get("template_id");
    let display_name: String = row.get("display_name");

    let record = load_spawn_record_for_template(&pool, template_id, "DebugArea", [0.0; 3], 0.0)
        .await
        .expect("query")
        .expect("template exists");
    assert_eq!(
        record.display_name.as_deref(),
        Some(display_name.as_str()),
        "a GM-spawned lineup actor keeps its nameplate"
    );
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
async fn live_db_gm_spawn_malformed_template_drops_gracefully() {
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
    sqlx::query("ALTER TABLE resources.entity_templates ALTER COLUMN template_name DROP NOT NULL")
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
        None,
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
async fn live_db_gm_spawn_missing_template_sends_nothing() {
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
        None,
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

/// CH-13: a batched copy (`.spawnrandom`) of a missing template reports
/// the failure to the cell, so the batch summary can say so at once
/// instead of waiting out its timeout. Revert proof: send the GM line
/// directly for a batch too and no `GmSpawnNpcFailed` arrives.
#[tokio::test]
async fn live_db_gm_spawn_missing_template_in_a_batch_reports_to_the_cell() {
    let pool = require_db_or_skip!();
    let (cell_tx, mut cell_rx) = mpsc::channel(8);
    let db_pool = Some(Arc::new(pool.clone()));
    let (transport, connected, entity_to_addr) = empty_io();

    handle_gm_spawn_npc(
        42,
        0x7FFF_FFF0,
        5,
        "Castle".to_string(),
        [0.0; 3],
        0.0,
        Some(9),
        &db_pool,
        &Some(cell_tx),
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    match cell_rx
        .try_recv()
        .expect("a batched failure must reach the cell")
    {
        BaseToCellMsg::GmSpawnNpcFailed {
            requester_entity_id,
            batch,
            template_id,
            reason,
        } => {
            assert_eq!((requester_entity_id, batch), (42, 9));
            assert_eq!(template_id, 0x7FFF_FFF0);
            assert_eq!(reason, format!("template {} not found", 0x7FFF_FFF0));
        }
        _ => panic!("expected GmSpawnNpcFailed"),
    }
}

/// CH-13: with no database a single spawn stays silent (unchanged), but a
/// batched copy still reports, or the batch would sit until its timeout.
#[tokio::test]
async fn gm_spawn_without_a_pool_reports_only_a_batched_copy() {
    let (transport, connected, entity_to_addr) = empty_io();

    let (cell_tx, mut cell_rx) = mpsc::channel(8);
    handle_gm_spawn_npc(
        42,
        77,
        5,
        "Castle".to_string(),
        [0.0; 3],
        0.0,
        None,
        &None,
        &Some(cell_tx),
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    assert!(cell_rx.try_recv().is_err(), "a single spawn sends nothing");

    let (cell_tx, mut cell_rx) = mpsc::channel(8);
    handle_gm_spawn_npc(
        42,
        77,
        5,
        "Castle".to_string(),
        [0.0; 3],
        0.0,
        Some(3),
        &None,
        &Some(cell_tx),
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;
    match cell_rx.try_recv().expect("a batched copy must report") {
        BaseToCellMsg::GmSpawnNpcFailed { batch, reason, .. } => {
            assert_eq!(batch, 3);
            assert_eq!(reason, "no database to resolve the template");
        }
        _ => panic!("expected GmSpawnNpcFailed"),
    }
}

/// CH-13: a batched copy whose template query fails (a database outage, not
/// a missing row) reports to the cell too. No live database needed: the pool
/// points at a closed port, so the query errors. Revert proof: send the GM
/// line for a batch too (or drop the `fail` call in the `Err` arm) and no
/// `GmSpawnNpcFailed` arrives.
#[tokio::test]
async fn gm_spawn_query_failure_in_a_batch_reports_to_the_cell() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy("postgres://nobody:nothing@[::1]:9/none")
        .expect("a lazy pool only parses the URL");
    let db_pool = Some(Arc::new(pool));
    let (transport, connected, entity_to_addr) = empty_io();
    let (cell_tx, mut cell_rx) = mpsc::channel(8);

    handle_gm_spawn_npc(
        42,
        77,
        5,
        "Castle".to_string(),
        [0.0; 3],
        0.0,
        Some(4),
        &db_pool,
        &Some(cell_tx),
        &transport,
        &connected,
        &entity_to_addr,
    )
    .await;

    match cell_rx
        .try_recv()
        .expect("a batched failure must reach the cell")
    {
        BaseToCellMsg::GmSpawnNpcFailed {
            requester_entity_id,
            batch,
            template_id,
            reason,
        } => {
            assert_eq!((requester_entity_id, batch, template_id), (42, 4, 77));
            assert_eq!(reason, "could not load template 77");
        }
        _ => panic!("expected GmSpawnNpcFailed"),
    }
}
