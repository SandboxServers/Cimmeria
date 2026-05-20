//! `handle_use_inventory_item` — fire the cell-side `OnItemUse` event for
//! an inventory instance the player owns. Does not consume the stack;
//! per-item consumption is the chain's responsibility via `Action::RemoveItem`.
//!
//! As of PR #338's right-click-to-equip work: weapon instances (rows with
//! `clip_size IS NOT NULL` in `resources.items`) bypass the `OnItemUse`
//! event and route to the move path instead — `useItem` on a weapon means
//! "equip" (main bag → bandolier) or "unequip" (bandolier → main bag)
//! depending on the source container.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use super::super::move_::handle_move_inventory_item;
use crate::base::outbox::{self, CellOutboxPayload};
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;

/// Container ids that this auto-equip router understands.
const CONTAINER_MAIN: i32 = 1;
const CONTAINER_BANDOLIER: i32 = 3;
/// Bandolier capacity. Mirrors `bag_max_slots(3)` (resources.rs:32) so a
/// future bandolier resize stays consistent if it's bumped here too.
const BANDOLIER_SLOTS: i32 = 4;

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
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "UseInventoryItem: no DB pool");
            return;
        }
    };

    // Resolve the inventory instance — type id, current location, and
    // whether the resource row marks it as a weapon (clip_size IS NOT
    // NULL). The single query keeps us at one DB round-trip for the
    // common "is this a weapon?" check that gates the equip/unequip
    // routing below.
    #[derive(sqlx::FromRow)]
    struct InstanceRow {
        type_id: i32,
        container_id: i32,
        is_weapon: bool,
    }
    let row: InstanceRow = match sqlx::query_as::<_, InstanceRow>(
        "SELECT inv.type_id, inv.container_id, \
                (ri.clip_size IS NOT NULL) AS is_weapon \
         FROM sgw_inventory inv \
         JOIN resources.items ri ON ri.item_id = inv.type_id \
         WHERE inv.character_id = $1 AND inv.item_id = $2 \
         LIMIT 1",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(r)) => r,
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
    let type_id = row.type_id;

    // Right-click auto-equip / auto-unequip (PR #338): weapons bypass
    // the `OnItemUse` event entirely and route to the move path
    // instead. Direction is keyed off `container_id`:
    //
    // - Bandolier (3) → Main (1): unequip back to first empty bag slot
    // - Main (1)     → Bandolier (3): equip to first empty bandolier
    //   slot, or replace the active slot if all 4 are full
    //
    // Other source containers (equipment slots, mission bag, etc.)
    // fall through to the normal `OnItemUse` event — auto-equip only
    // makes sense between the inventory↔bandolier pair.
    if row.is_weapon
        && (row.container_id == CONTAINER_BANDOLIER || row.container_id == CONTAINER_MAIN)
    {
        let target =
            match resolve_auto_equip_target(pool.as_ref(), player_id, row.container_id).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!(
                        player_id,
                        item_id,
                        type_id,
                        source_container = row.container_id,
                        "UseInventoryItem: auto-equip target resolve failed: {e}"
                    );
                    return;
                }
            };
        let (target_container, target_slot) = match target {
            Some(t) => t,
            None => {
                // Main-bag full: nowhere to put the unequipped weapon.
                // The user-visible failure mode is silent rejection —
                // we don't have a wire channel for "bag full" feedback
                // beyond the move handler's existing reject paths, so
                // log loudly server-side and let the player notice the
                // weapon is still equipped.
                tracing::warn!(
                    player_id, item_id, type_id,
                    source_container = row.container_id,
                    "UseInventoryItem: no destination slot for auto-equip/unequip — request ignored"
                );
                return;
            }
        };
        tracing::info!(
            entity_id,
            player_id,
            item_id,
            type_id,
            source_container = row.container_id,
            target_container,
            target_slot,
            "UseInventoryItem: routing weapon to move handler (auto-equip/unequip)"
        );
        handle_move_inventory_item(
            entity_id,
            player_id,
            item_id,
            target_container,
            target_slot,
            1, // quantity — weapons are stack=1
            db_pool,
            cell_tx,
            socket,
            connected,
            entity_to_addr,
        )
        .await;
        return;
    }

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

/// Pick the destination slot for an auto-equip / auto-unequip.
///
/// - Source = Bandolier (3) → returns `(1, first_empty_main_slot)`.
///   Returns `None` only when the main bag is full (40 slots — rare in
///   practice but possible).
/// - Source = Main (1) → returns `(3, first_empty_bandolier_slot)`.
///   When all 4 bandolier slots are full, falls back to the player's
///   currently-active `bandolier_slot` — that's the intuitive
///   "replace what I'm holding" behavior the user asked for.
/// - Any other source returns `None` (caller skips the auto-equip
///   route and falls back to the normal `OnItemUse` event).
async fn resolve_auto_equip_target(
    pool: &PgPool,
    player_id: i32,
    source_container: i32,
) -> Result<Option<(i32, i32)>, sqlx::Error> {
    match source_container {
        CONTAINER_BANDOLIER => {
            // Unequip → first empty Main slot.
            // We scan up to `bag_max_slots(1)` (=40). The query
            // returns the lowest unoccupied slot in container 1.
            let slot: Option<i32> = sqlx::query_scalar(
                "SELECT s.slot \
                 FROM generate_series(0, 39) AS s(slot) \
                 LEFT JOIN sgw_inventory inv \
                   ON inv.character_id = $1 \
                  AND inv.container_id = 1 \
                  AND inv.slot_id = s.slot \
                 WHERE inv.item_id IS NULL \
                 ORDER BY s.slot ASC \
                 LIMIT 1",
            )
            .bind(player_id)
            .fetch_optional(pool)
            .await?;
            Ok(slot.map(|s| (CONTAINER_MAIN, s)))
        }
        CONTAINER_MAIN => {
            // Equip → first empty Bandolier slot. If full, fall back
            // to the player's active bandolier slot for a replace.
            let empty: Option<i32> = sqlx::query_scalar(
                "SELECT s.slot \
                 FROM generate_series(0, $1::int - 1) AS s(slot) \
                 LEFT JOIN sgw_inventory inv \
                   ON inv.character_id = $2 \
                  AND inv.container_id = 3 \
                  AND inv.slot_id = s.slot \
                 WHERE inv.item_id IS NULL \
                 ORDER BY s.slot ASC \
                 LIMIT 1",
            )
            .bind(BANDOLIER_SLOTS)
            .bind(player_id)
            .fetch_optional(pool)
            .await?;
            if let Some(s) = empty {
                return Ok(Some((CONTAINER_BANDOLIER, s)));
            }
            // All 4 slots full — replace the active one (swap).
            let active: Option<i32> =
                sqlx::query_scalar("SELECT bandolier_slot FROM sgw_player WHERE player_id = $1")
                    .bind(player_id)
                    .fetch_optional(pool)
                    .await?;
            Ok(active.map(|s| (CONTAINER_BANDOLIER, s)))
        }
        _ => Ok(None),
    }
}
