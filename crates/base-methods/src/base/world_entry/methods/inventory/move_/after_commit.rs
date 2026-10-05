//! What a committed `moveItem` sets off: the client resync, the cell's
//! `InventoryItemMoveApplied`, the bandolier sync and the appearance
//! refresh. Nothing here can undo the move; each step logs its own failure.

use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::super::player_load::core::EQUIPMENT_CONTAINERS;
use super::super::super::vendor::helpers::sync_bandolier_after_inventory_change_with_options;
use super::super::appearance::refresh_player_appearance;
use super::super::core::{send_full_inventory_update, send_on_remove_item};
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;

/// The committed move, as the side effects need it.
#[derive(Debug, Clone, Copy)]
pub(super) struct AppliedMove {
    pub entity_id: u32,
    pub player_id: i32,
    /// The request's `item_id`, for the log.
    pub item_id: i32,
    /// The row that now sits in the target slot: the request's `item_id`,
    /// or the new row a split inserted.
    pub applied_item_id: i32,
    pub type_id: i32,
    pub source_container_id: i32,
    pub target_container_id: i32,
    /// The occupant a swap moved into the source slot.
    pub swapped_item_id: Option<i32>,
    /// A whole-stack merge deleted the source row (`item_id`).
    pub source_deleted: bool,
}

/// Run every post-commit side effect of `applied`, in the order the move
/// path always has: resync, cell notification, bandolier, appearance.
pub(super) async fn after_commit(
    applied: AppliedMove,
    pool: &Arc<PgPool>,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let AppliedMove {
        entity_id,
        player_id,
        item_id,
        applied_item_id,
        type_id,
        source_container_id,
        target_container_id,
        swapped_item_id,
        source_deleted,
    } = applied;

    // The full update below only sends rows that exist, so a row the merge
    // deleted would stay on the client's screen without its own removal.
    if source_deleted {
        send_on_remove_item(entity_id, item_id, transport, connected, entity_to_addr).await;
    }

    let total_items = send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;

    let player_label = known_names::player_name(player_id);
    tracing::debug!(
        entity_id,
        entity_name = player_label,
        player_id,
        player_name = player_label,
        item_id,
        item_name = cimmeria_names::book().item(type_id),
        total_items,
        "Inventory move persisted"
    );

    if let Some(cell_tx) = cell_tx {
        let _ = cell_tx
            .send(BaseToCellMsg::InventoryItemMoveApplied {
                entity_id,
                item_id: applied_item_id,
                type_id,
                source_container_id,
                target_container_id,
                swapped_item_id,
            })
            .await;
    }

    if source_container_id == 3 || target_container_id == 3 {
        // Unequip (source=bandolier, target=elsewhere): defer the
        // base-side `refresh_player_appearance` so the cell-side
        // holster animation has time to play. The cell's
        // `SyncBandolierItems` handler fires `Item_Unequip` and
        // schedules a Phase 2 (`holster_animation_complete_at`) that
        // dispatches the eventual `RefreshAppearance` back to base
        // after `HOLSTER_ANIMATION_DURATION`. Without this defer,
        // the base yanks the weapon mesh immediately and the user
        // sees no animation — the weapon just vanishes.
        let is_unequip = source_container_id == 3 && target_container_id != 3;
        sync_bandolier_after_inventory_change_with_options(
            entity_id,
            player_id,
            db_pool,
            cell_tx,
            transport,
            connected,
            entity_to_addr,
            is_unequip,
        )
        .await;
    }

    // Equipment containers (4..=14) — armor and other slotted visuals.
    // The grant path already refreshes appearance on equipment grants;
    // the bandolier branch above handles weapons. Without this branch,
    // manually dragging armor into (or out of) a slot persists to DB but
    // the player-visible model on every client keeps the pre-move
    // components.
    //
    // Gated on `visual_component IS NOT NULL` to match the grant path's
    // shape (grant\mod.rs:425) — non-visual items (charms, ID-only
    // artifacts) can legally occupy equipment slots without contributing
    // to the appearance composite, so refreshing for them is wasted
    // wire traffic. Lookup is keyed by `source.type_id`, which is the
    // type both the equip leg (bag→slot) and the unequip leg (slot→bag)
    // are moving; for a swap, only the source item's visual matters
    // (the displaced occupant's container also changes, but that case
    // is already covered when the swap's source/target straddles
    // equipment).
    if EQUIPMENT_CONTAINERS.contains(&source_container_id)
        || EQUIPMENT_CONTAINERS.contains(&target_container_id)
    {
        let has_visual: bool = sqlx::query_scalar(
            "SELECT visual_component IS NOT NULL FROM resources.items WHERE item_id = $1",
        )
        .bind(type_id)
        .fetch_optional(pool.as_ref())
        .await
        .ok()
        .flatten()
        .unwrap_or(false);

        if has_visual {
            refresh_player_appearance(
                entity_id,
                player_id,
                db_pool,
                transport,
                connected,
                entity_to_addr,
                cell_tx,
            )
            .await;
        }
    }
}
