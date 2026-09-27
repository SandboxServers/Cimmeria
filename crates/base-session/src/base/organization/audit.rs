//! Exports what the member-delete trigger did to the `org` log target.
//!
//! The trigger (`org_member_after_delete`, `db/sgw/_functions.sql`) promotes
//! a new leader, disbands, or leaves an organization memberless inside
//! Postgres, where tracing cannot see it. It writes each result to
//! `sgw_organization_events`, and the functions here log each row exactly
//! once and stamp its `exported_at`:
//!
//! - [`export_committed`]: rows a committed transaction wrote, logged at
//!   INFO. `character_delete::delete_character` calls it right after its
//!   commit.
//! - [`export_in_transaction`]: rows the current transaction wrote, logged
//!   at DEBUG because the caller may still roll back (and a rollback takes
//!   the stamp with it). `persistence::remove_member` calls it.
//! - [`sweep_unexported`]: every row still unstamped, logged at INFO. Run
//!   once at base startup ([`spawn_startup_sweep`]) for rows a bare
//!   `DELETE` (psql, a test, a crash between commit and export) left.
//!
//! Each export is an `UPDATE … WHERE exported_at IS NULL RETURNING`, so two
//! exporters racing for a row log it once between them.

use std::sync::Arc;

use sqlx::{PgExecutor, PgPool, Postgres, Transaction};

/// One exported row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrgEventRow {
    pub event_id: i64,
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
pub enum ExportSource {
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

macro_rules! stamp_returning {
    ($where:literal) => {
        concat!(
            "UPDATE sgw_organization_events SET exported_at = now() WHERE exported_at IS NULL AND ",
            $where,
            " RETURNING event_id, org_id, event, reason, from_player_id, from_account_id, \
             to_player_id, to_account_id, (extract(epoch FROM at) * 1000)::bigint",
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

/// Log, at INFO, every unexported row transaction `tx_id` wrote, and stamp
/// them. Call after that transaction committed.
pub async fn export_committed(pool: &PgPool, tx_id: i64) -> Result<Vec<OrgEventRow>, sqlx::Error> {
    let rows = stamp(pool, Some(tx_id), stamp_returning!("tx_id = $1")).await?;
    Ok(log_rows(rows, ExportSource::CharacterDelete))
}

/// Log, at DEBUG, every unexported row the current transaction wrote, and
/// stamp them in that transaction.
pub async fn export_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Vec<OrgEventRow>, sqlx::Error> {
    let rows = stamp(&mut **tx, None, stamp_returning!("tx_id = txid_current()")).await?;
    Ok(log_rows(rows, ExportSource::InTransaction))
}

/// Log, at INFO, every row still unstamped, and stamp them.
pub async fn sweep_unexported(pool: &PgPool) -> Result<Vec<OrgEventRow>, sqlx::Error> {
    let rows = stamp(pool, None, stamp_returning!("true")).await?;
    Ok(log_rows(rows, ExportSource::StartupSweep))
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

async fn stamp<'e>(
    executor: impl PgExecutor<'e>,
    tx_id: Option<i64>,
    sql: &'static str,
) -> Result<Vec<OrgEventRow>, sqlx::Error> {
    let query = sqlx::query_as(sql);
    let query = match tx_id {
        Some(id) => query.bind(id),
        None => query,
    };
    let rows: Vec<Row> = query.fetch_all(executor).await?;
    let mut rows: Vec<OrgEventRow> = rows
        .into_iter()
        .map(
            |(event_id, org_id, event, reason, fp, fa, tp, ta, at_unix_ms)| OrgEventRow {
                event_id,
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
    rows.sort_by_key(|r| r.event_id);
    Ok(rows)
}

fn log_rows(rows: Vec<OrgEventRow>, source: ExportSource) -> Vec<OrgEventRow> {
    for r in &rows {
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
                    event_id = r.event_id,
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
    rows
}
