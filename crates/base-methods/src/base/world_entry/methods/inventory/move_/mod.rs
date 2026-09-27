//! `moveItem`: move an inventory item between containers and slots, with a
//! split, a merge into a same-type stack, or a swap with the occupant.
//!
//! - [`mod.rs`](self): the entry points, the target checks, the move locks
//!   and the locked source read.
//! - [`finish`]: the vault rules, the choice of write, the commit.
//! - [`apply`]: the four writes.
//! - [`after_commit`]: resync, cell notification, bandolier, appearance.
//! - [`container_policy`]: the player-movable allowlist and the refusal path.
//! - [`bank_rules`]: the personal vault's rules (BV-03).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::inventory::INV_BANK;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::vault::VaultAccess;
use sqlx::PgPool;
use tokio::sync::mpsc;
use tracing::Instrument;

use super::super::super::super::resources::{bag_max_slots, bag_min_slot};
use super::super::super::super::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use container_policy::{container_refusal, refuse_move, MoveEnd};

pub(crate) use container_policy::player_accessible;

mod after_commit;
mod apply;
mod bank_rules;
mod container_policy;
mod finish;

#[derive(sqlx::FromRow)]
struct InventoryInstanceRow {
    type_id: i32,
    stack_size: i32,
    container_id: i32,
    slot_id: i32,
    bound: bool,
    durability: i32,
    charges: i32,
}

/// The row already sitting in the target slot, locked for the move.
#[derive(Debug, Clone, Copy, sqlx::FromRow)]
struct Occupant {
    item_id: i32,
    type_id: i32,
    stack_size: i32,
}

/// One `moveItem` request, as the cell forwarded it.
#[derive(Debug, Clone, Copy)]
struct MoveRequest {
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    target_container_id: i32,
    target_slot_id: i32,
    /// As sent: `<= 0` means the whole stack.
    quantity: i32,
}

/// What every step of a move needs to reach the database, the cell and the
/// client.
struct MoveCtx<'a> {
    pool: &'a Arc<PgPool>,
    db_pool: &'a Option<Arc<PgPool>>,
    cell_tx: &'a Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &'a Arc<dyn Transport>,
    connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

/// Move an item with no vault session: a move into or out of the vault (17)
/// is refused. For in-process callers that never reach the vault
/// (right-click auto-equip moves between 1 and 3).
pub async fn handle_move_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    target_container_id: i32,
    target_slot_id: i32,
    quantity: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_move_inventory_item_with_vault(
        entity_id,
        player_id,
        item_id,
        target_container_id,
        target_slot_id,
        quantity,
        VaultAccess::NO_SESSION,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// Move an inventory item between containers/slots, optionally splitting,
/// merging or swapping with the occupant. `vault` is the cell's verdict on
/// this move (BV-03): a move into or out of the vault (17) needs it open.
#[tracing::instrument(
    name = "inventory.move_item",
    level = "info",
    skip_all,
    fields(
        entity_id,
        player_id,
        item_id,
        target_container_id,
        target_slot_id,
        quantity
    )
)]
pub async fn handle_move_inventory_item_with_vault(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    target_container_id: i32,
    target_slot_id: i32,
    quantity: i32,
    vault: VaultAccess,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "MoveInventoryItem: no DB pool");
            return;
        }
    };
    let req = MoveRequest {
        entity_id,
        player_id,
        item_id,
        target_container_id,
        target_slot_id,
        quantity,
    };
    let ctx = MoveCtx {
        pool,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    };
    // The bank branch of `moveItem` gets its own info span (D-BV19). A
    // withdrawal is only known once the source row is read, so `move_item`
    // opens the span there.
    if target_container_id == INV_BANK {
        move_item(req, &vault, &ctx)
            .instrument(bank_move_span(&req, &vault))
            .await;
    } else {
        move_item(req, &vault, &ctx).await;
    }
}

/// `bank.move_item`: the info span around a move that touches the vault.
fn bank_move_span(req: &MoveRequest, vault: &VaultAccess) -> tracing::Span {
    tracing::info_span!(
        target: "bank",
        "bank.move_item",
        entity_id = req.entity_id,
        player_id = req.player_id,
        item_id = req.item_id,
        target_container_id = req.target_container_id,
        target_slot_id = req.target_slot_id,
        vault_open = vault.is_open(),
        banker_id = vault.banker_id(),
    )
}

async fn refuse(
    req: &MoveRequest,
    refusal: bank_rules::MoveRefusal,
    vault: &VaultAccess,
    ctx: &MoveCtx<'_>,
) {
    refuse_move(
        refusal,
        vault,
        req.entity_id,
        req.player_id,
        req.item_id,
        req.quantity,
        req.target_container_id,
        req.target_slot_id,
        ctx.pool,
        ctx.transport,
        ctx.connected,
        ctx.entity_to_addr,
    )
    .await;
}

async fn move_item(req: MoveRequest, vault: &VaultAccess, ctx: &MoveCtx<'_>) {
    let MoveRequest {
        player_id,
        item_id,
        target_container_id,
        target_slot_id,
        quantity,
        ..
    } = req;

    // D-BV07 allowlist, target end. Checked before the slot range: 17-20
    // have a capacity now, so the range check alone would accept them.
    if let Some(refusal) = container_refusal(MoveEnd::Target, target_container_id, vault) {
        refuse(&req, refusal, vault, ctx).await;
        return;
    }

    let max_slots = bag_max_slots(target_container_id);
    let min_slot = bag_min_slot(target_container_id);
    // Reject out-of-range slot targets. The wire decoder is responsible for
    // translating client-side 1-indexed slot IDs into the 0-indexed values
    // this handler operates on, so a `target_slot_id < min_slot` here means
    // the client genuinely asked for a slot below the container's allowed
    // range (forged packet, off-by-one bug elsewhere). For the vault this is
    // only the ceiling (100); the player's own `bank_slots` is checked inside
    // the transaction ([`finish`]).
    //
    // Quantity validation is deferred to AFTER the source row is read.
    // The SGW client's drag-to-equip / drag-to-bag UI sends
    // `quantity = -1` to mean "move the whole stack" (legacy convention
    // from `SGWPlayer.py:moveItem`). A naive `<= 0` reject here breaks
    // every drag-to-bandolier interaction. Treat `<= 0` as the
    // whole-stack sentinel, resolved against `source.stack_size`
    // once we've read the row inside the tx.
    if target_container_id <= 0 || target_slot_id < min_slot || target_slot_id >= max_slots {
        tracing::warn!(
            player_id,
            item_id,
            target_container_id,
            target_slot_id,
            quantity,
            min_slot,
            max_slots,
            "MoveInventoryItem: invalid target slot"
        );
        return;
    }

    let mut tx = match ctx.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(player_id, item_id, "MoveInventoryItem: begin failed: {e}");
            return;
        }
    };

    // Per-player advisory lock serializes ALL inventory moves for this player.
    //
    // A per-(player, container) lock is not enough: opposite-direction swaps
    // (A→B and B→A running concurrently) each lock their own source row first,
    // then deadlock when each tries to FOR-UPDATE the other's target occupant.
    // Taking a single per-player lock before any row locks eliminates that
    // ordering problem outright. Moves are rare enough that the contention
    // cost is negligible compared to the deadlock risk.
    //
    // Sentinel arg `0` distinguishes the "all-containers" move lock from the
    // per-container slot-reservation locks taken by
    // `reserve_free_inventory_slots(player_id, container_id)`.
    if let Err(e) = sqlx::query("SELECT pg_advisory_xact_lock($1, 0)")
        .bind(player_id)
        .execute(&mut *tx)
        .await
    {
        let _ = tx.rollback().await;
        tracing::error!(
            player_id,
            item_id,
            target_container_id,
            "MoveInventoryItem: advisory lock failed: {e}"
        );
        return;
    }

    // Also take the per-container lock for the target so concurrent
    // grants/purchases that call `reserve_free_inventory_slots(player_id,
    // target_container)` block until the move commits. Without this, a grant
    // can read target-slot occupancy, see the slot free, INSERT into it, and
    // commit before this move's UPDATE relocates the source row — the unique
    // index on (character_id, container_id, slot_id) would then surface as a
    // user-visible error on a legitimate move. Source-container lock is taken
    // below once we've read the source row.
    if let Err(e) = sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind(player_id)
        .bind(target_container_id)
        .execute(&mut *tx)
        .await
    {
        let _ = tx.rollback().await;
        tracing::error!(
            player_id,
            item_id,
            target_container_id,
            "MoveInventoryItem: target container lock failed: {e}"
        );
        return;
    }

    // Read source row inside the tx with FOR UPDATE so concurrent moves observe
    // a consistent snapshot. Without this, the swap path could lose updates.
    let source = match sqlx::query_as::<_, InventoryInstanceRow>(
        "SELECT type_id, stack_size, container_id, slot_id, bound, durability, charges \
         FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 LIMIT 1 FOR UPDATE",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            let _ = tx.rollback().await;
            tracing::warn!(
                player_id,
                item_id,
                "MoveInventoryItem: source item not found"
            );
            return;
        }
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(
                player_id,
                item_id,
                "MoveInventoryItem: source query failed: {e}"
            );
            return;
        }
    };

    // A withdrawal: the rest of the move runs in the bank span. A deposit
    // already runs in it (see `handle_move_inventory_item_with_vault`).
    if source.container_id == INV_BANK && target_container_id != INV_BANK {
        locked_move(req, vault, ctx, tx, source)
            .instrument(bank_move_span(&req, vault))
            .await;
    } else {
        locked_move(req, vault, ctx, tx, source).await;
    }
}

/// The move from the locked source row on: the source-end allowlist, the
/// quantity, the source container lock, then [`finish::finish_move`].
async fn locked_move(
    req: MoveRequest,
    vault: &VaultAccess,
    ctx: &MoveCtx<'_>,
    mut tx: sqlx::Transaction<'static, sqlx::Postgres>,
    source: InventoryInstanceRow,
) {
    let MoveRequest {
        player_id,
        item_id,
        target_container_id,
        target_slot_id,
        ..
    } = req;

    // D-BV07 allowlist, source end. The source container is only
    // known from the locked row, so this sits after the FOR UPDATE read.
    if let Some(refusal) = container_refusal(MoveEnd::Source, source.container_id, vault) {
        let _ = tx.rollback().await;
        refuse(&req, refusal, vault, ctx).await;
        return;
    }

    // Resolve the whole-stack sentinel (client sends `quantity = -1`
    // for drag-to-equip / drag-to-bag — see the deferred-validation
    // note in `move_item`). Any non-positive value is treated as
    // "move the whole stack."
    let quantity = if req.quantity <= 0 {
        source.stack_size
    } else {
        req.quantity
    };

    if quantity > source.stack_size {
        let _ = tx.rollback().await;
        tracing::warn!(
            player_id,
            item_id,
            quantity,
            stack_size = source.stack_size,
            "MoveInventoryItem: requested quantity exceeds stack — rejecting"
        );
        return;
    }

    if source.container_id == target_container_id && source.slot_id == target_slot_id {
        let _ = tx.rollback().await;
        return;
    }

    // Source-container lock matches the target-container lock taken above —
    // moves where source ≠ target also need to serialize against grants into
    // the source bag (the swap path moves the displaced occupant into the
    // source's old slot and would race the same way).
    if source.container_id != target_container_id {
        if let Err(e) = sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
            .bind(player_id)
            .bind(source.container_id)
            .execute(&mut *tx)
            .await
        {
            let _ = tx.rollback().await;
            tracing::error!(
                player_id,
                item_id,
                source_container_id = source.container_id,
                "MoveInventoryItem: source container lock failed: {e}"
            );
            return;
        }
    }

    finish::finish_move(req, quantity, vault, ctx, tx, source).await;
}

#[cfg(test)]
mod allowlist_tests;
#[cfg(test)]
mod concurrency_tests;
#[cfg(test)]
mod refusal_infra_tests;
#[cfg(test)]
mod refusal_resync_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod vault_concurrency_tests;
#[cfg(test)]
mod vault_move_shape_tests;
#[cfg(test)]
mod vault_move_tests;
