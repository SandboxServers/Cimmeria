//! `handle_use_inventory_item` — fire the cell-side `OnItemUse` event for
//! an inventory instance the player owns. Does not consume the stack;
//! per-item consumption is the chain's responsibility via `Action::RemoveItem`.

use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::base::outbox::{self, CellOutboxPayload};
use crate::cell::messages::BaseToCellMsg;

/// Resolve an inventory instance's design id and fire the cell-side
/// content-engine `OnItemUse` event. **Does not consume the item.**
///
/// `item_id` here is the inventory **instance id** from the wire (per
/// `SGWInventoryManager.def`'s `useItem` arg). The cell never knows the
/// design id at use-time — base resolves it from `sgw_inventory` and sends
/// it back via `BaseToCellMsg::ItemUsed`. Per-item consumption (if any) is
/// the chain's responsibility via `Action::RemoveItem`, which routes through
/// [`super::handle_remove_inventory_item_by_type`] — mirrors python's
/// `Inventory.useItem` (`python/cell/Inventory.py:419-432`) which fires
/// `item.use::<typeId>` as a pure event and lets per-mission handlers decide
/// whether to remove the stack (e.g., `FindAmbernol.py:115` calls
/// `removeItemByDesign(19, 1, False)` while radios just don't remove).
///
/// Refusing to fire when the instance isn't owned by this character keeps
/// content events tied to actual inventory state — a malicious client
/// can't spam `useItem(any id)` to trigger arbitrary chain actions.
///
/// Delivery durability: the `ItemUsed` event is enqueued in
/// `cell_event_outbox` first, then dispatched on the in-process channel.
/// `tokio::sync::mpsc::Sender::send().await` only fails when the receiver
/// is dropped (e.g., cell task panicked / shut down) — full channels
/// backpressure rather than error — so this guards specifically against
/// a torn-down receiver. The undelivered row is replayed by the
/// background drainer ([`crate::base::outbox::spawn_drainer`]) once a
/// healthy cell channel is back. The cell-side `BaseToCellMsg::ItemUsed`
/// handler is idempotent — chain conditions self-gate on
/// `step_status = active` so a duplicate fire from a drainer retry is
/// harmless.
pub async fn handle_use_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    target_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "UseInventoryItem: no DB pool");
            return;
        }
    };

    // Read-only ownership lookup. No FOR UPDATE — we don't mutate. The
    // simple SELECT also avoids the per-call transaction overhead the
    // pre-#95 consume-on-use code paid.
    let type_id: i32 = match sqlx::query_scalar::<_, i32>(
        "SELECT type_id FROM sgw_inventory \
         WHERE character_id = $1 AND item_id = $2 LIMIT 1",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(tid)) => tid,
        Ok(None) => {
            tracing::warn!(
                player_id, item_id,
                "UseInventoryItem: instance not found for this character — refusing to fire ItemUsed"
            );
            return;
        }
        Err(e) => {
            tracing::error!(player_id, item_id, "UseInventoryItem: lookup failed: {e}");
            return;
        }
    };

    tracing::info!(
        entity_id,
        player_id,
        item_id,
        type_id,
        target_id,
        "UseInventoryItem: firing ItemUsed (no consumption — chain decides)"
    );

    let payload = CellOutboxPayload::ItemUsed {
        instance_id: item_id,
        type_id,
        target_id,
    };
    let outbox_id = match outbox::enqueue(pool.as_ref(), entity_id, &payload).await {
        Ok(id) => id,
        Err(e) => {
            // Outbox INSERT failed — cannot guarantee delivery, so do NOT
            // attempt the in-process send (a successful send without an
            // outbox row would lose its retry safety net on next failure).
            // The player can re-use the item; ownership lookup above is
            // idempotent.
            tracing::error!(
                entity_id,
                player_id,
                item_id,
                type_id,
                "UseInventoryItem: outbox enqueue failed; ItemUsed not dispatched: {e}"
            );
            return;
        }
    };

    if let Some(cell_tx) = cell_tx {
        outbox::try_dispatch_now(pool.as_ref(), cell_tx, outbox_id, entity_id, payload).await;
    } else {
        tracing::debug!(
            entity_id,
            outbox_id,
            "UseInventoryItem: no cell channel; row left for drainer"
        );
    }
}
