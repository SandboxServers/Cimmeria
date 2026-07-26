//! BaseApp-side executor for `.`-console authoring SQL.
//!
//! The `.`-console spawn/patrol authoring commands (`savespawn`, `path_add`,
//! …) run on the CellService, which has no DB pool. They hand a
//! **server-generated** `INSERT`/`UPDATE`/`DELETE` to the base via
//! [`CellToBaseMsg::ExecuteAuthoringSql`](crate::cell::messages::CellToBaseMsg::ExecuteAuthoringSql);
//! this handler runs it against the live pool and reports the row count back to
//! the GM on the feedback channel.
//!
//! The write is intentionally **transient**: it lets the developer see the
//! change hold across reconnects within the current deploy, but the next deploy
//! rebuilds the DB from the `db/resources/` seeds and wipes it. The durable
//! artifact is the seed SQL the cell records to the per-session authoring log
//! and (later) Discord — see `crate::cell::console::seed`.
//!
//! **Trust model:** the `.`-channel is GM-gated server-side (the cell only
//! forwards a command from an `access_level >= GameMaster` caller), and `sql`
//! is server-generated — numeric values come from cell-parsed `i32`/`f32` and
//! strings are escaped through `crate::cell::console::seed::sql_str`, so there
//! is no raw client text concatenated into the statement. This mirrors the
//! legacy `Atrea.dbQuery` authoring path.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::{AssertSqlSafe, PgPool};

use crate::base::gm_feedback::send_gm_feedback_to_client;
use crate::base::ConnectedClientState;

/// Max search hits reported per `.search*` command — keeps a broad query from
/// flooding the feedback channel.
const SEARCH_LIMIT: i64 = 25;

/// Execute one authoring statement and report the outcome to the GM.
#[tracing::instrument(
    name = "console.authoring_sql",
    level = "info",
    skip_all,
    fields(entity_id, label)
)]
pub(crate) async fn handle_execute_authoring_sql(
    entity_id: u32,
    label: &str,
    sql: &str,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let Some(pool) = db_pool else {
        tracing::warn!(
            entity_id,
            label,
            "authoring SQL: no DB pool — live write skipped (seed SQL still \
             recorded cell-side for commit)"
        );
        send_gm_feedback_to_client(
            entity_id,
            &format!("{label}: no live DB — change applied in-memory only"),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        return;
    };

    // `AssertSqlSafe` is required by sqlx 0.9: `query()` now only accepts
    // `&'static str` unless the caller asserts the string was audited for
    // injection. The audit is the "Trust model" note in this module's header —
    // `sql` is server-generated on the GM-gated `.`-channel, numerics come from
    // cell-parsed `i32`/`f32`, and strings are escaped through
    // `crate::cell::console::seed::sql_str`. No raw client text is concatenated.
    match sqlx::query(AssertSqlSafe(sql)).execute(pool.as_ref()).await {
        Ok(res) => {
            let rows = res.rows_affected();
            if rows == 0 {
                // A zero-row result means the statement ran but matched
                // nothing (e.g. an UPDATE/DELETE whose WHERE found no row, or
                // an idempotent re-run). The seed SQL is still recorded, but
                // the live DB is unchanged — report that distinctly rather
                // than as a successful write, or the GM thinks their authoring
                // edit took effect when it didn't.
                tracing::warn!(
                    entity_id,
                    label,
                    "authoring SQL affected 0 rows — live DB unchanged"
                );
                send_gm_feedback_to_client(
                    entity_id,
                    &format!(
                        "{label}: live DB UNCHANGED (0 rows matched) — seed SQL recorded; \
                         check the target exists"
                    ),
                    transport,
                    connected,
                    entity_to_addr,
                )
                .await;
            } else {
                tracing::info!(entity_id, label, rows, "authoring SQL executed");
                send_gm_feedback_to_client(
                    entity_id,
                    &format!("{label}: live DB write ok ({rows} row(s); wiped on next deploy)"),
                    transport,
                    connected,
                    entity_to_addr,
                )
                .await;
            }
        }
        Err(e) => {
            tracing::warn!(entity_id, label, error = %e, "authoring SQL failed");
            send_gm_feedback_to_client(
                entity_id,
                &format!("{label}: live DB write FAILED ({e})"),
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
    }
}

/// Run a read-only resource-name search and report `id: name` matches to the GM.
///
/// `kind` selects the table: `0` items, `1` missions, `2` entity_templates. The
/// query string is bound as a parameterized `ILIKE` pattern — never
/// concatenated — so this is injection-safe regardless of the channel gate.
#[tracing::instrument(
    name = "console.search",
    level = "info",
    skip_all,
    fields(entity_id, kind)
)]
pub(crate) async fn handle_console_search(
    entity_id: u32,
    kind: u8,
    query: &str,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let (label, sql) = match kind {
        0 => (
            "searchitem",
            "SELECT item_id AS id, name FROM resources.items \
             WHERE name ILIKE $1 ORDER BY item_id LIMIT $2",
        ),
        1 => (
            "searchmission",
            "SELECT mission_id AS id, mission_defn AS name FROM resources.missions \
             WHERE mission_defn ILIKE $1 ORDER BY mission_id LIMIT $2",
        ),
        2 => (
            "searchtemplate",
            "SELECT template_id AS id, template_name AS name FROM resources.entity_templates \
             WHERE template_name ILIKE $1 ORDER BY template_id LIMIT $2",
        ),
        _ => return,
    };

    let Some(pool) = db_pool else {
        send_gm_feedback_to_client(
            entity_id,
            &format!("{label}: no live DB connection"),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        return;
    };

    let pattern = format!("%{query}%");
    let rows = sqlx::query_as::<_, (i32, String)>(sql)
        .bind(&pattern)
        .bind(SEARCH_LIMIT)
        .fetch_all(pool.as_ref())
        .await;

    match rows {
        Ok(hits) if hits.is_empty() => {
            send_gm_feedback_to_client(
                entity_id,
                &format!("{label}: no matches for '{query}'"),
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        Ok(hits) => {
            send_gm_feedback_to_client(
                entity_id,
                &format!("{label} '{query}': {} match(es)", hits.len()),
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            for (id, name) in hits {
                send_gm_feedback_to_client(
                    entity_id,
                    &format!("    {id}: {name}"),
                    transport,
                    connected,
                    entity_to_addr,
                )
                .await;
            }
        }
        Err(e) => {
            tracing::warn!(entity_id, label, error = %e, "console search query failed");
            send_gm_feedback_to_client(
                entity_id,
                &format!("{label}: query failed ({e})"),
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
    }
}
