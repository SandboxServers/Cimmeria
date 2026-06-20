//! `handle_use_inventory_item` — fire the cell-side `OnItemUse` event for
//! an inventory instance the player owns. Does not consume the stack;
//! per-item consumption is the chain's responsibility via `Action::RemoveItem`.
//!
//! Right-click auto-equip routing: instances whose item type lists the
//! bandolier (container 3) in `container_sets` bypass the `OnItemUse`
//! event and route to the move handler — "equip" (main bag → bandolier)
//! or "unequip" (bandolier → main bag) depending on the source container.
//! Items NOT in `container_sets` for the bandolier (consumables like
//! slappacks, mission items, etc.) fall through to `OnItemUse` so per-item
//! chains in `content_chains` can match.
//!
//! The bandolier-eligibility check intentionally reads `container_sets`
//! directly rather than inferring from `clip_size` or any other proxy.
//! Earlier versions inferred "is weapon" from `clip_size IS NOT NULL`,
//! but seed data uses `clip_size = 0` for non-weapons (the slappack row
//! at `db/resources/Items/Seed/items.sql` is a representative example),
//! so the inference misclassified every consumable as a weapon and
//! suppressed its `OnItemUse` event. `container_sets` is the
//! authoritative signal the move handler already uses
//! (`item_allows_container`), so reading it here keeps both sides
//! consistent.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::move_::handle_move_inventory_item;
use crate::base::outbox::{self, CellOutboxPayload};
use crate::base::resources::bag_max_slots;
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;

// Re-export the canonical container ids the auto-equip router cares
// about. Defined once in `cimmeria_entity::inventory`; re-named locally
// so the route conditions read naturally without the `INV_` prefix.
use cimmeria_entity::inventory::{
    INV_BANDOLIER as CONTAINER_BANDOLIER, INV_MAIN as CONTAINER_MAIN,
};

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
#[tracing::instrument(
    name = "inventory.use_item",
    level = "info",
    skip_all,
    fields(entity_id, player_id, item_id, target_id)
)]
pub async fn handle_use_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    target_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
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
    // whether the resource row marks the type as bandolier-eligible
    // (`3 = ANY(container_sets)`). Bandolier-eligible items are
    // auto-equip/unequip targets; everything else falls through to
    // the `OnItemUse` event. One round-trip per useItem dispatch.
    //
    // Why not `clip_size IS NOT NULL`: see the module doc. Seed data
    // uses `clip_size = 0` for non-weapons, so the null-check
    // misclassifies every consumable as a weapon.
    #[derive(sqlx::FromRow)]
    struct InstanceRow {
        type_id: i32,
        container_id: i32,
        is_bandolier_eligible: bool,
    }
    // `$3` is the canonical bandolier container id, bound from the
    // `CONTAINER_BANDOLIER` re-export rather than inlined in the SQL
    // so the discriminator stays pegged to the same constant the move
    // handler (`item_allows_container`) consults — if the constant
    // ever moves, both sides update together rather than silently
    // disagreeing.
    let row: InstanceRow = match sqlx::query_as::<_, InstanceRow>(
        "SELECT inv.type_id, inv.container_id, \
                ($3 = ANY(ri.container_sets)) AS is_bandolier_eligible \
         FROM sgw_inventory inv \
         JOIN resources.items ri ON ri.item_id = inv.type_id \
         WHERE inv.character_id = $1 AND inv.item_id = $2 \
         LIMIT 1",
    )
    .bind(player_id)
    .bind(item_id)
    .bind(CONTAINER_BANDOLIER)
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

    // Right-click auto-equip / auto-unequip: bandolier-eligible items
    // bypass the `OnItemUse` event entirely and route to the move path
    // instead. Direction is keyed off `container_id`:
    //
    // - Bandolier (3) → Main (1): unequip back to first empty bag slot
    // - Main (1)     → Bandolier (3): equip to first empty bandolier
    //   slot, or replace the active slot if all 4 are full
    //
    // Other source containers (equipment slots, mission bag, etc.)
    // fall through to the normal `OnItemUse` event — auto-equip only
    // makes sense between the inventory↔bandolier pair.
    if row.is_bandolier_eligible
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
            transport,
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

    // Discord gameplay-channel (off by default — high volume). Resolve the
    // player's name through entity_to_addr → connected; skip the emit if the
    // name isn't cached. `target` carries the numeric target id when one was
    // supplied (cell-side has the name; base only has the id).
    if let Some(character_name) = entity_to_addr
        .lock()
        .ok()
        .and_then(|m| m.get(&entity_id).copied())
        .and_then(|a| {
            connected
                .lock()
                .ok()
                .and_then(|c| c.get(&a).and_then(|s| s.player_name.clone()))
        })
    {
        let target = (target_id != 0).then(|| format!("entity:{target_id}"));
        cimmeria_discord::emit_item_used(character_name, type_id, target);
    }

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
    // Find the lowest empty slot in `dest_container` by joining
    // generate_series(0, capacity-1) against sgw_inventory. The
    // capacity comes from `bag_max_slots` so a future bandolier or
    // main-bag resize lands here automatically — no hard-coded
    // ranges to drift.
    async fn first_empty_slot(
        pool: &PgPool,
        player_id: i32,
        dest_container: i32,
    ) -> Result<Option<i32>, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT s.slot \
             FROM generate_series(0, $1::int - 1) AS s(slot) \
             LEFT JOIN sgw_inventory inv \
               ON inv.character_id = $2 \
              AND inv.container_id = $3 \
              AND inv.slot_id = s.slot \
             WHERE inv.item_id IS NULL \
             ORDER BY s.slot ASC \
             LIMIT 1",
        )
        .bind(bag_max_slots(dest_container))
        .bind(player_id)
        .bind(dest_container)
        .fetch_optional(pool)
        .await
    }

    match source_container {
        CONTAINER_BANDOLIER => Ok(first_empty_slot(pool, player_id, CONTAINER_MAIN)
            .await?
            .map(|s| (CONTAINER_MAIN, s))),
        CONTAINER_MAIN => {
            if let Some(s) = first_empty_slot(pool, player_id, CONTAINER_BANDOLIER).await? {
                return Ok(Some((CONTAINER_BANDOLIER, s)));
            }
            // All bandolier slots full — replace the active one.
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
