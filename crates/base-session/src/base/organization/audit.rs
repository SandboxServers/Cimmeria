//! Exports what the member-delete trigger did to the `org` log target.
//!
//! The trigger (`org_member_after_delete`, `db/sgw/_functions.sql`) promotes
//! a new leader, disbands, or leaves an organization memberless inside
//! Postgres, where tracing cannot see it. It writes each result to
//! `sgw_organization_events`, and the functions here log each row and then
//! stamp its `exported_at`:
//!
//! - [`export_committed`]: rows a committed transaction wrote, logged at
//!   INFO. `character_delete::delete_character` calls it right after its
//!   commit.
//! - [`export_in_transaction`]: rows the current transaction wrote, logged
//!   at DEBUG because the caller may still roll back (and a rollback takes
//!   the row and its stamp with it). `persistence::remove_member` calls it.
//! - [`sweep_unexported`]: every row still unstamped, logged at INFO. Run
//!   at base startup ([`spawn_startup_sweep`]) for rows a bare `DELETE`
//!   (psql, a test) or a crash left.
//!
//! **Delivery is at least once.** Each export locks its rows
//! (`FOR UPDATE SKIP LOCKED`, so two exporters never log the same row at
//! the same time), logs them, then stamps them and commits. A crash after
//! the log and before the commit leaves the rows unstamped, and the next
//! startup sweep logs them again. Every exported event carries the row's
//! `org_event_id` so a query can drop the duplicates.

use std::sync::Arc;

use sqlx::{PgConnection, PgPool, Postgres, Transaction};

/// One exported row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgEventRow {
    /// The row's id: the dedup key for an at-least-once export.
    pub org_event_id: i64,
    pub org_id: i32,
    /// `leader_changed`, `disbanded` or `left_memberless`.
    pub event: String,
    /// `character_deleted` or `member_removed`.
    pub reason: String,
    pub from_player_id: i32,
    pub from_account_id: i32,
    pub to_player_id: Option<i32>,
    pub to_account_id: Option<i32>,
    /// When the trigger wrote the row, in Unix milliseconds.
    pub at_unix_ms: i64,
}

/// Which exporter logged a row (the `source` log field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportSource {
    CharacterDelete,
    InTransaction,
    StartupSweep,
}

impl ExportSource {
    fn as_str(self) -> &'static str {
        match self {
            ExportSource::CharacterDelete => "character_delete",
            ExportSource::InTransaction => "in_transaction",
            ExportSource::StartupSweep => "startup_sweep",
        }
    }
}

/// Which unstamped rows an export takes.
#[derive(Debug, Clone, Copy)]
enum Scope {
    /// Rows transaction `tx_id` wrote.
    Tx(i64),
    /// Rows the current transaction wrote.
    CurrentTx,
    /// Every unstamped row.
    All,
}

type Row = (
    i64,
    i32,
    String,
    String,
    i32,
    i32,
    Option<i32>,
    Option<i32>,
    i64,
);

macro_rules! select_unexported {
    ($where:literal) => {
        concat!(
            "SELECT org_event_id, org_id, event, reason, from_player_id, from_account_id, \
             to_player_id, to_account_id, (extract(epoch FROM at) * 1000)::bigint \
             FROM sgw_organization_events WHERE exported_at IS NULL AND ",
            $where,
            " ORDER BY org_event_id FOR UPDATE SKIP LOCKED",
        )
    };
}

/// The id of the current transaction, as the trigger stamps it on its rows.
/// Call it inside the transaction whose rows [`export_committed`] should log
/// after the commit.
pub async fn current_tx_id(tx: &mut Transaction<'_, Postgres>) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("SELECT txid_current()")
        .fetch_one(&mut **tx)
        .await
}

/// Log, at INFO, every unexported row transaction `tx_id` wrote, then stamp
/// them. Call after that transaction committed.
pub async fn export_committed(pool: &PgPool, tx_id: i64) -> Result<Vec<OrgEventRow>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let rows = export(&mut tx, Scope::Tx(tx_id), ExportSource::CharacterDelete).await?;
    tx.commit().await?;
    Ok(rows)
}

/// Log, at DEBUG, every unexported row the current transaction wrote, and
/// stamp them in that transaction.
pub async fn export_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Vec<OrgEventRow>, sqlx::Error> {
    export(tx, Scope::CurrentTx, ExportSource::InTransaction).await
}

/// Log, at INFO, every row still unstamped, then stamp them.
pub async fn sweep_unexported(pool: &PgPool) -> Result<Vec<OrgEventRow>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    let rows = export(&mut tx, Scope::All, ExportSource::StartupSweep).await?;
    tx.commit().await?;
    Ok(rows)
}

/// Run [`sweep_unexported`] once in the background (base startup).
pub fn spawn_startup_sweep(pool: Arc<PgPool>) {
    tokio::spawn(async move {
        match sweep_unexported(&pool).await {
            Ok(rows) => tracing::debug!(
                target: "org",
                event = "org_events_swept",
                rows = rows.len(),
                "Exported unstamped organization trigger events"
            ),
            Err(e) => tracing::warn!(
                target: "org",
                event = "org_events_swept",
                reason = "db_error",
                error = %e,
                "Organization trigger-event sweep failed; the rows stay unstamped for the next start"
            ),
        }
    });
}

/// Lock the rows in `scope`, log them, stamp them. The caller commits.
async fn export(
    conn: &mut PgConnection,
    scope: Scope,
    source: ExportSource,
) -> Result<Vec<OrgEventRow>, sqlx::Error> {
    let rows: Vec<Row> = match scope {
        Scope::Tx(tx_id) => {
            sqlx::query_as(select_unexported!("tx_id = $1"))
                .bind(tx_id)
                .fetch_all(&mut *conn)
                .await?
        }
        Scope::CurrentTx => {
            sqlx::query_as(select_unexported!("tx_id = txid_current()"))
                .fetch_all(&mut *conn)
                .await?
        }
        Scope::All => {
            sqlx::query_as(select_unexported!("true"))
                .fetch_all(&mut *conn)
                .await?
        }
    };
    let rows: Vec<OrgEventRow> = rows
        .into_iter()
        .map(
            |(org_event_id, org_id, event, reason, fp, fa, tp, ta, at_unix_ms)| OrgEventRow {
                org_event_id,
                org_id,
                event,
                reason,
                from_player_id: fp,
                from_account_id: fa,
                to_player_id: tp,
                to_account_id: ta,
                at_unix_ms,
            },
        )
        .collect();
    if rows.is_empty() {
        return Ok(rows);
    }
    // Log first, stamp second: a failure in between re-sends, never drops.
    log_rows(&rows, source);
    let ids: Vec<i64> = rows.iter().map(|r| r.org_event_id).collect();
    sqlx::query(
        "UPDATE sgw_organization_events SET exported_at = now() WHERE org_event_id = ANY($1)",
    )
    .bind(&ids)
    .execute(&mut *conn)
    .await?;
    Ok(rows)
}

fn log_rows(rows: &[OrgEventRow], source: ExportSource) {
    for r in rows {
        macro_rules! emit {
            ($level:ident) => {
                tracing::$level!(
                    target: "org",
                    event = r.event.as_str(),
                    reason = r.reason.as_str(),
                    org_id = r.org_id,
                    from_player_id = r.from_player_id,
                    from_account_id = r.from_account_id,
                    to_player_id = r.to_player_id,
                    to_account_id = r.to_account_id,
                    org_event_id = r.org_event_id,
                    at_unix_ms = r.at_unix_ms,
                    source = source.as_str(),
                    "Organization changed by the member-delete trigger"
                )
            };
        }
        match source {
            ExportSource::InTransaction => emit!(debug),
            ExportSource::CharacterDelete | ExportSource::StartupSweep => emit!(info),
        }
    }
}
