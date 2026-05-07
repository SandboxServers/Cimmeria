//! Durable base→cell content event delivery.
//!
//! Closes the gap between "base committed a DB change" and "cell fires the
//! corresponding `OnItemUse` / inventory chain event". The previous
//! best-effort `cell_tx.send(...)` could lose events when the receiver
//! was dropped (cell task panic / shut down) — for `OnItemUse` that
//! strands mission progression with no recovery path. (Note: tokio mpsc
//! `send().await` does not error on a full channel — it backpressures —
//! so this guards specifically against a torn-down receiver, not
//! saturation.)
//!
//! Two write paths:
//!   * [`enqueue`] — INSERT a row in its own short-lived statement. Used by
//!     [`crate::base::world_entry::methods::inventory::handle_use_inventory_item`],
//!     which has no surrounding tx because it doesn't mutate inventory.
//!   * [`enqueue_in_tx`] — INSERT inside a caller-owned transaction so the
//!     event row is committed atomically with the caller's DB write. Future
//!     callers (grant/remove) will use this; not yet wired here.
//!
//! Two delivery paths:
//!   * [`try_dispatch_now`] — fired immediately after enqueue. On a healthy
//!     in-process channel this clears the row before the drainer ever
//!     looks at it.
//!   * [`drain_undelivered`] — periodic + startup sweep that resends
//!     anything still undelivered (channel was closed at the time, base
//!     restarted before the row could be marked delivered, etc.).
//!
//! Cell-side handlers must be idempotent because retries can fan out a
//! second copy after a crash. `OnItemUse` already is — chain conditions
//! self-gate on `step_status = active`, and `Action::RemoveItem` handles
//! consumption.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sqlx::{types::Json, PgPool, Postgres, Transaction};
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;

#[cfg(test)]
mod tests;

/// Event types persisted in `cell_event_outbox.event_type`. Stable strings —
/// changing one breaks in-flight rows on existing databases.
const EVENT_TYPE_ITEM_USED: &str = "item_used";
const EVENT_TYPE_INVENTORY_ITEM_GRANTED: &str = "inventory_item_granted";
const EVENT_TYPE_INVENTORY_ITEM_REMOVED: &str = "inventory_item_removed";

/// Strongly-typed payload variants. Serialized as JSON into the
/// `cell_event_outbox.payload` JSONB column. Adding a new variant requires
/// only (a) a new enum arm here, (b) a matching `event_type` constant, and
/// (c) an arm in [`row_to_message`] AND in [`try_dispatch_now`].
///
/// Variants intentionally don't carry `entity_id` — it lives in its own
/// indexed column for ordering / per-entity drainer scoping.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CellOutboxPayload {
    ItemUsed {
        /// Inventory row id the player clicked. Carried through so the
        /// `OnItemUse` chain context records exactly which stack
        /// initiated the use, letting `Action::RemoveItem` consume that
        /// instance instead of the player's first-by-type.
        ///
        /// `#[serde(default)]` so undelivered rows enqueued by older
        /// code (before this field existed) deserialize cleanly with
        /// `instance_id = 0`. The cell-side executor falls back to
        /// `RemoveInventoryItemByType` when the value is 0, so old
        /// rows reach the drainer with the same behavior they had
        /// when written. Without this attribute the drainer hard-fails
        /// on legacy rows with `ColumnDecode("missing field
        /// 'instance_id'")` and the outbox stalls.
        #[serde(default)]
        instance_id: i32,
        type_id: i32,
        target_id: i32,
    },
    InventoryItemGranted {
        item_id: i32,
        container_id: i32,
        slot_id: i32,
        quantity: i32,
    },
    InventoryItemRemoved {
        item_id: i32,
        source_container_id: i32,
    },
}

impl CellOutboxPayload {
    fn event_type(&self) -> &'static str {
        match self {
            CellOutboxPayload::ItemUsed { .. } => EVENT_TYPE_ITEM_USED,
            CellOutboxPayload::InventoryItemGranted { .. } => EVENT_TYPE_INVENTORY_ITEM_GRANTED,
            CellOutboxPayload::InventoryItemRemoved { .. } => EVENT_TYPE_INVENTORY_ITEM_REMOVED,
        }
    }
}

/// Single undelivered row pulled by the drainer.
#[derive(sqlx::FromRow)]
struct OutboxRow {
    id: i64,
    entity_id: i32,
    event_type: String,
    payload: Json<CellOutboxPayload>,
    /// Used to rate-limit poison-row warnings — first encounter logs WARN,
    /// subsequent retries log DEBUG so a single bad row doesn't spam the
    /// log every drain interval.
    attempts: i32,
}

/// INSERT a fresh outbox row using a one-shot statement (no caller tx).
/// Returns the assigned `id` so the caller can mark it delivered after a
/// successful in-process send.
pub async fn enqueue(
    pool: &PgPool,
    entity_id: u32,
    payload: &CellOutboxPayload,
) -> Result<i64, sqlx::Error> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO cell_event_outbox (entity_id, event_type, payload) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(entity_id as i32)
    .bind(payload.event_type())
    .bind(Json(payload))
    .fetch_one(pool)
    .await?;
    Ok(id)
}

/// INSERT inside a caller-owned transaction so the outbox row commits
/// atomically with the caller's DB-visible state change. Used by grant /
/// remove / vendor purchase / vendor sell — anything that already opens a
/// transaction for an inventory mutation. `handle_use_inventory_item` uses
/// [`enqueue`] instead because it has no surrounding tx.
pub async fn enqueue_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    entity_id: u32,
    payload: &CellOutboxPayload,
) -> Result<i64, sqlx::Error> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO cell_event_outbox (entity_id, event_type, payload) \
         VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(entity_id as i32)
    .bind(payload.event_type())
    .bind(Json(payload))
    .fetch_one(&mut **tx)
    .await?;
    Ok(id)
}

/// Mark a row delivered. Called after a successful `cell_tx.send(...)`.
async fn mark_delivered(pool: &PgPool, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE cell_event_outbox SET delivered_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await
        .map(|_| ())
}

/// Bump attempts + capture the failure string for operator diagnostics.
/// Best-effort: a failure here is logged but does not propagate, since the
/// drainer will simply pick the row up again on its next pass.
async fn record_failure(pool: &PgPool, id: i64, error: &str) {
    let res = sqlx::query(
        "UPDATE cell_event_outbox \
         SET attempts = attempts + 1, last_error = $2 \
         WHERE id = $1",
    )
    .bind(id)
    .bind(error)
    .execute(pool)
    .await;
    if let Err(e) = res {
        tracing::warn!(
            outbox_id = id,
            "outbox: failed to record dispatch failure: {e}"
        );
    }
}

/// Build the `BaseToCellMsg` for a stored outbox row. Returns `None` if the
/// row's `event_type` is unknown (forward-compat: a newer base wrote an event
/// type this version doesn't understand) or doesn't match the payload shape.
///
/// No logging here — the drainer is the only caller and decides whether to
/// log (first encounter) or stay quiet (already-flagged poison row), based
/// on `attempts`. Without this gate, a single bad row turns into a continuous
/// warn-spam at the 5s drain interval.
fn row_to_message(row: &OutboxRow) -> Option<BaseToCellMsg> {
    let entity_id = row.entity_id as u32;
    match (row.event_type.as_str(), &*row.payload) {
        (
            EVENT_TYPE_ITEM_USED,
            CellOutboxPayload::ItemUsed {
                instance_id,
                type_id,
                target_id,
            },
        ) => Some(BaseToCellMsg::ItemUsed {
            entity_id,
            instance_id: *instance_id,
            type_id: *type_id,
            target_id: *target_id,
        }),
        (
            EVENT_TYPE_INVENTORY_ITEM_GRANTED,
            CellOutboxPayload::InventoryItemGranted {
                item_id,
                container_id,
                slot_id,
                quantity,
            },
        ) => Some(BaseToCellMsg::InventoryItemGranted {
            entity_id,
            item_id: *item_id,
            container_id: *container_id,
            slot_id: *slot_id,
            quantity: *quantity,
        }),
        (
            EVENT_TYPE_INVENTORY_ITEM_REMOVED,
            CellOutboxPayload::InventoryItemRemoved {
                item_id,
                source_container_id,
            },
        ) => Some(BaseToCellMsg::InventoryItemRemoved {
            entity_id,
            item_id: *item_id,
            source_container_id: *source_container_id,
        }),
        _ => None,
    }
}

/// Try to deliver a freshly-enqueued event over the in-process channel.
/// On success, marks the row delivered. On failure, leaves the row for the
/// drainer to retry. Errors are logged, never propagated — the durability
/// guarantee is "row exists in DB", not "send succeeded".
pub async fn try_dispatch_now(
    pool: &PgPool,
    cell_tx: &mpsc::Sender<BaseToCellMsg>,
    id: i64,
    entity_id: u32,
    payload: CellOutboxPayload,
) {
    let msg = match payload {
        CellOutboxPayload::ItemUsed {
            instance_id,
            type_id,
            target_id,
        } => BaseToCellMsg::ItemUsed {
            entity_id,
            instance_id,
            type_id,
            target_id,
        },
        CellOutboxPayload::InventoryItemGranted {
            item_id,
            container_id,
            slot_id,
            quantity,
        } => BaseToCellMsg::InventoryItemGranted {
            entity_id,
            item_id,
            container_id,
            slot_id,
            quantity,
        },
        CellOutboxPayload::InventoryItemRemoved {
            item_id,
            source_container_id,
        } => BaseToCellMsg::InventoryItemRemoved {
            entity_id,
            item_id,
            source_container_id,
        },
    };
    match cell_tx.send(msg).await {
        Ok(()) => {
            if let Err(e) = mark_delivered(pool, id).await {
                // Row will be redelivered by the drainer — cell handler
                // must be idempotent. Logged because a chronic mark-delivered
                // failure means the drainer is doing redundant sends.
                tracing::warn!(
                    outbox_id = id,
                    "outbox: dispatch succeeded but mark_delivered failed: {e}"
                );
            }
        }
        Err(e) => {
            tracing::warn!(
                outbox_id = id,
                "outbox: cell channel send failed; left for drainer retry: {e}"
            );
            record_failure(pool, id, &format!("send: {e}")).await;
        }
    }
}

/// Default batch size for the drainer. Bounded to keep one drainer pass
/// from monopolising the channel under a backlog spike.
const DRAIN_BATCH_SIZE: i64 = 64;

/// Outcome of a single drain pass — separates "rows successfully delivered"
/// from "rows skipped without action" so the operator log isn't misleading
/// when the drainer breaks early on a closed channel. Returned via
/// [`drain_undelivered`].
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct DrainStats {
    /// Rows whose `delivered_at` was set during this pass.
    pub delivered: usize,
    /// Rows skipped because `row_to_message` rejected them (unknown
    /// event_type / payload mismatch).
    pub skipped_bad: usize,
    /// Rows that hit a send error this pass — the loop breaks on the first
    /// such row, so this is at most 1 today, but it's a count so future
    /// "keep going on transient errors" changes don't change the API shape.
    pub send_failed: usize,
}

/// Drain a single batch of undelivered rows. Returns per-outcome counts
/// rather than just `rows.len()` — the loop breaks early on a closed cell
/// channel, so a raw row count would overstate progress. Exposed
/// `pub(crate)` so the periodic loop and tests can both call it.
pub(crate) async fn drain_undelivered(
    pool: &PgPool,
    cell_tx: &mpsc::Sender<BaseToCellMsg>,
) -> Result<DrainStats, sqlx::Error> {
    // ORDER BY id keeps per-entity FIFO (id is BIGSERIAL — strictly
    // increasing per insert across all entities, so per-entity ordering is
    // implied by global ordering). LIMIT bounds the batch so we don't
    // hold the channel under a backlog. No FOR UPDATE SKIP LOCKED yet —
    // single-writer base assumption; multi-instance HA is out of scope here.
    let rows: Vec<OutboxRow> = sqlx::query_as::<_, OutboxRow>(
        "SELECT id, entity_id, event_type, payload, attempts \
         FROM cell_event_outbox \
         WHERE delivered_at IS NULL \
         ORDER BY id \
         LIMIT $1",
    )
    .bind(DRAIN_BATCH_SIZE)
    .fetch_all(pool)
    .await?;

    let mut stats = DrainStats::default();
    for row in rows {
        let id = row.id;
        let Some(msg) = row_to_message(&row) else {
            // Poison row — log loud once (first encounter) then drop to
            // debug for subsequent retries. Without the rate-limit, the
            // 5s drainer interval turns one bad row into permanent warn
            // spam.
            if row.attempts == 0 {
                tracing::warn!(
                    outbox_id = id, event_type = %row.event_type, attempts = row.attempts,
                    "outbox: row event_type/payload mismatch — left undelivered for redeploy/rollback"
                );
            } else {
                tracing::debug!(
                    outbox_id = id, event_type = %row.event_type, attempts = row.attempts,
                    "outbox: row event_type/payload mismatch (already flagged)"
                );
            }
            // Bump attempts so the next pass can stay quiet, and so
            // operators have a count for dead-letter triage when that
            // ships.
            record_failure(pool, id, &format!("unknown event_type: {}", row.event_type)).await;
            stats.skipped_bad += 1;
            continue;
        };
        match cell_tx.send(msg).await {
            Ok(()) => {
                if let Err(e) = mark_delivered(pool, id).await {
                    tracing::warn!(
                        outbox_id = id,
                        "outbox: drainer dispatch succeeded but mark_delivered failed: {e}"
                    );
                }
                stats.delivered += 1;
            }
            Err(e) => {
                tracing::warn!(
                    outbox_id = id,
                    "outbox: drainer cell channel send failed (receiver dropped); will retry next pass: {e}"
                );
                record_failure(pool, id, &format!("drainer send: {e}")).await;
                stats.send_failed += 1;
                // Receiver is gone — stop the batch; subsequent rows would
                // hit the same error. The next periodic pass will retry
                // if (when) the channel is rebuilt.
                break;
            }
        }
    }
    Ok(stats)
}

/// Default drainer interval. Fast enough that a transient channel hiccup
/// doesn't strand a player for long; slow enough that a quiet outbox costs
/// nothing.
const DRAIN_INTERVAL: Duration = Duration::from_secs(5);

/// Spawn the background drainer task. Runs immediately on startup
/// (delivers any rows left over from a previous process), then every
/// [`DRAIN_INTERVAL`]. The task lives for the life of the process — there's
/// no shutdown signal because base service shutdown drops the cell_tx
/// which makes the next send fail and the loop continues retrying until
/// the runtime is itself dropped.
pub fn spawn_drainer(pool: Arc<PgPool>, cell_tx: mpsc::Sender<BaseToCellMsg>) {
    tokio::spawn(async move {
        tracing::debug!("cell_event_outbox drainer started");
        // Startup pass — replay any rows orphaned by a previous shutdown.
        if let Err(e) = drain_undelivered(&pool, &cell_tx).await {
            tracing::warn!("outbox: startup drain failed: {e}");
        }
        let mut ticker = tokio::time::interval(DRAIN_INTERVAL);
        // Skip the immediate first tick — we just drained above.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            match drain_undelivered(&pool, &cell_tx).await {
                Ok(s) if s.delivered > 0 || s.skipped_bad > 0 || s.send_failed > 0 => {
                    tracing::debug!(
                        delivered = s.delivered,
                        skipped_bad = s.skipped_bad,
                        send_failed = s.send_failed,
                        "outbox: periodic drain"
                    );
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("outbox: periodic drain failed: {e}"),
            }
        }
    });
}
