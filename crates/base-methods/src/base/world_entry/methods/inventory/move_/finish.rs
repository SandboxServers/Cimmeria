//! The second half of a move, under every lock: the vault rules, the choice
//! of write (whole, split, merge, swap), the commit and the side effects.

use cimmeria_entity::inventory::INV_BANK;
use cimmeria_wire::cell::vault::VaultAccess;
use sqlx::{Postgres, Transaction};

use super::super::grant::item_allows_container;
use super::after_commit::{after_commit, AppliedMove};
use super::bank_rules::{
    is_mission_item, log_move_accepted, move_kind, read_vault_owner, touches_vault,
    AcceptedVaultMove, MoveRefusal, MoveShape, VaultOwner,
};
use super::container_policy::MoveEnd;
use super::{apply, refuse, InventoryInstanceRow, MoveCtx, MoveRequest, Occupant};

type MoveTx = Transaction<'static, Postgres>;

/// Finish a move whose source row and containers are locked. `quantity` is
/// resolved (never `<= 0`) and at most the source stack.
pub(super) async fn finish_move(
    req: MoveRequest,
    quantity: i32,
    vault: &VaultAccess,
    ctx: &MoveCtx<'_>,
    mut tx: MoveTx,
    source: InventoryInstanceRow,
) {
    let MoveRequest {
        entity_id,
        player_id,
        item_id,
        target_container_id,
        target_slot_id,
        ..
    } = req;

    // The vault's own rules (BV-03). The session verdict already passed at
    // both ends; what is left needs the database.
    let owner = if touches_vault(source.container_id, target_container_id) {
        match vault_owner(&mut tx, &req).await {
            Some(owner) => Some(owner),
            None => {
                let _ = tx.rollback().await;
                return;
            }
        }
    } else {
        None
    };
    if let Some(refusal) = deposit_refusal(&mut tx, &req, &source, owner).await {
        let _ = tx.rollback().await;
        refuse(&req, refusal, vault, ctx).await;
        return;
    }

    if !item_allows_container(ctx.pool, source.type_id, target_container_id).await {
        let _ = tx.rollback().await;
        tracing::warn!(
            player_id,
            item_id,
            type_id = source.type_id,
            target_container_id,
            "MoveInventoryItem: item cannot be moved into target container"
        );
        // A vault move is refused visibly (`bank` log, line, snap-back);
        // other moves keep their quiet refusal.
        if owner.is_some() {
            let refusal = MoveRefusal::ItemNotAllowed {
                end: MoveEnd::Target,
            };
            refuse(&req, refusal, vault, ctx).await;
        }
        return;
    }

    // The occupant, locked for the rest of the tx, so the merge and swap
    // arms can rely on its cached type and stack.
    let occupied: Option<Occupant> = match sqlx::query_as::<_, Occupant>(
        "SELECT item_id, type_id, stack_size, bound, durability, charges FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2 AND slot_id = $3 AND item_id <> $4 LIMIT 1 FOR UPDATE",
    )
    .bind(player_id)
    .bind(target_container_id)
    .bind(target_slot_id)
    .bind(item_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(result) => result,
        Err(e) => {
            let _ = tx.rollback().await;
            tracing::error!(player_id, target_container_id, target_slot_id, "MoveInventoryItem: occupied slot query failed: {e}");
            return;
        }
    };

    let shape = match choose_shape(&mut tx, &req, quantity, &source, occupied.as_ref()).await {
        Ok(shape) => shape,
        Err(refusal) => {
            let _ = tx.rollback().await;
            if let (Some(refusal), Some(_)) = (refusal, owner) {
                refuse(&req, refusal, vault, ctx).await;
            }
            return;
        }
    };

    // A swap out of the vault puts the occupant into it: the occupant must
    // pass the deposit rules too (D-BV08).
    if let (MoveShape::Swap, Some(occ)) = (shape, occupied.as_ref()) {
        if source.container_id == INV_BANK && target_container_id != INV_BANK {
            match is_mission_item(&mut tx, occ.type_id, target_container_id).await {
                Ok(false) => {}
                Ok(true) => {
                    let _ = tx.rollback().await;
                    refuse(&req, MoveRefusal::MissionItem, vault, ctx).await;
                    return;
                }
                Err(e) => {
                    let _ = tx.rollback().await;
                    tracing::error!(
                        player_id,
                        item_id,
                        occupied_item_id = occ.item_id,
                        "MoveInventoryItem: occupant mission-item lookup failed: {e}"
                    );
                    return;
                }
            }
        }
        if !item_allows_container(ctx.pool, occ.type_id, source.container_id).await {
            let _ = tx.rollback().await;
            tracing::warn!(
                player_id,
                item_id,
                occupied_item_id = occ.item_id,
                occupied_item_type = occ.type_id,
                source_container_id = source.container_id,
                "MoveInventoryItem: occupied item cannot be swapped into source container"
            );
            if owner.is_some() {
                let refusal = MoveRefusal::ItemNotAllowed {
                    end: MoveEnd::Source,
                };
                refuse(&req, refusal, vault, ctx).await;
            }
            return;
        }
    }

    // The id we report on `InventoryItemMoveApplied`: the instance that
    // moved. The source row for a whole move, a swap or a merge (it pairs
    // with `source_container_id`; a whole merge has just deleted it), and the
    // new row for a split.
    let applied = match (shape, occupied.as_ref()) {
        (MoveShape::Whole, _) => apply::whole(
            &mut tx,
            player_id,
            item_id,
            target_container_id,
            target_slot_id,
        )
        .await
        .map(|()| (item_id, false)),
        (MoveShape::Split, _) => apply::split(
            &mut tx,
            player_id,
            item_id,
            quantity,
            &source,
            target_container_id,
            target_slot_id,
        )
        .await
        .map(|new_id| (new_id, false)),
        (MoveShape::Merge, Some(occ)) => {
            apply::merge(&mut tx, player_id, item_id, quantity, &source, occ)
                .await
                .map(|deleted| (item_id, deleted))
        }
        (MoveShape::Swap, Some(occ)) => apply::swap(
            &mut tx,
            player_id,
            item_id,
            &source,
            occ,
            target_container_id,
            target_slot_id,
        )
        .await
        .map(|()| (item_id, false)),
        (MoveShape::Merge | MoveShape::Swap, None) => None,
    };
    let Some((applied_item_id, source_deleted)) = applied else {
        let _ = tx.rollback().await;
        return;
    };

    if let Err(e) = tx.commit().await {
        tracing::error!(player_id, item_id, "MoveInventoryItem: commit failed: {e}");
        return;
    }

    if let Some(owner) = owner {
        log_move_accepted(
            &accepted(&req, quantity, &source, occupied.as_ref(), shape, owner),
            vault,
        );
    }

    after_commit(
        AppliedMove {
            entity_id,
            player_id,
            item_id,
            applied_item_id,
            type_id: source.type_id,
            source_container_id: source.container_id,
            target_container_id,
            swapped_item_id: match shape {
                MoveShape::Swap => occupied.map(|o| o.item_id),
                _ => None,
            },
            source_deleted,
        },
        ctx.pool,
        ctx.db_pool,
        ctx.cell_tx,
        ctx.transport,
        ctx.connected,
        ctx.entity_to_addr,
    )
    .await;
}

/// Read the player's `account_id` and `bank_slots` in the move transaction.
/// `None`, logged, when the row is missing or the read fails; the caller
/// rolls back.
async fn vault_owner(tx: &mut MoveTx, req: &MoveRequest) -> Option<VaultOwner> {
    match read_vault_owner(tx, req.player_id).await {
        Ok(Some(owner)) => Some(owner),
        Ok(None) => {
            tracing::warn!(
                player_id = req.player_id,
                item_id = req.item_id,
                "MoveInventoryItem: player row missing for a vault move"
            );
            None
        }
        Err(e) => {
            tracing::error!(
                player_id = req.player_id,
                item_id = req.item_id,
                "MoveInventoryItem: vault owner read failed: {e}"
            );
            None
        }
    }
}

/// The deposit rules for a move into the vault from elsewhere, and the slot
/// bound for any move into it: the slot must be below the player's
/// `bank_slots` (not the ceiling of 100), and a mission item may not enter
/// (D-BV08). `None` when the move passes, or does not target the vault.
async fn deposit_refusal(
    tx: &mut MoveTx,
    req: &MoveRequest,
    source: &InventoryInstanceRow,
    owner: Option<VaultOwner>,
) -> Option<MoveRefusal> {
    if req.target_container_id != INV_BANK {
        return None;
    }
    let owner = owner?;
    let bank_slots = i32::from(owner.bank_slots);
    if req.target_slot_id >= bank_slots {
        return Some(MoveRefusal::BankSlotLocked { bank_slots });
    }
    if source.container_id == INV_BANK {
        return None;
    }
    match is_mission_item(tx, source.type_id, source.container_id).await {
        Ok(true) => Some(MoveRefusal::MissionItem),
        Ok(false) => None,
        Err(e) => {
            // Fail closed: a deposit whose item cannot be classified is
            // refused as if it were a mission item, and the error is logged.
            tracing::error!(
                player_id = req.player_id,
                item_id = req.item_id,
                "MoveInventoryItem: mission-item lookup failed, refusing the deposit: {e}"
            );
            Some(MoveRefusal::MissionItem)
        }
    }
}

/// Pick the write. `Err` (logged) refuses the move: with the refusal a
/// vault move reports, or `None` for an infrastructure failure.
///
/// - Empty target: the whole stack, or a split.
/// - A same-type occupant with the same `bound`, `durability` and
///   `charges`, and room for `quantity`: a merge (legacy
///   `Inventory.py:391-395`; a partial merge too, which the legacy server
///   left unimplemented). A bound stack never merges into an unbound one,
///   which would make its count sellable, tradable and mailable.
/// - Any other occupant: a swap of the whole stack; a split onto it is
///   refused.
async fn choose_shape(
    tx: &mut MoveTx,
    req: &MoveRequest,
    quantity: i32,
    source: &InventoryInstanceRow,
    occupied: Option<&Occupant>,
) -> Result<MoveShape, Option<MoveRefusal>> {
    let Some(occ) = occupied else {
        return Ok(if quantity < source.stack_size {
            MoveShape::Split
        } else {
            MoveShape::Whole
        });
    };
    let same_instance_state = occ.bound == source.bound
        && occ.durability == source.durability
        && occ.charges == source.charges;
    if occ.type_id == source.type_id && same_instance_state {
        let max_stack: Option<i32> = match sqlx::query_scalar(
            "SELECT max_stack_size FROM resources.items WHERE item_id = $1",
        )
        .bind(source.type_id)
        .fetch_optional(&mut **tx)
        .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(
                    player_id = req.player_id,
                    item_id = req.item_id,
                    "MoveInventoryItem: max_stack_size lookup failed: {e}"
                );
                return Err(None);
            }
        };
        let merged = occ.stack_size.checked_add(quantity);
        if max_stack.zip(merged).is_some_and(|(max, n)| n <= max) {
            return Ok(MoveShape::Merge);
        }
    }
    if quantity < source.stack_size {
        tracing::warn!(
            player_id = req.player_id,
            item_id = req.item_id,
            target_container_id = req.target_container_id,
            target_slot_id = req.target_slot_id,
            "MoveInventoryItem: cannot split onto occupied slot"
        );
        return Err(Some(MoveRefusal::SplitOntoOccupied));
    }
    Ok(MoveShape::Swap)
}

/// The `move_accepted` record of a committed vault move: the stacks at both
/// ends before and after the write.
fn accepted(
    req: &MoveRequest,
    quantity: i32,
    source: &InventoryInstanceRow,
    occupied: Option<&Occupant>,
    shape: MoveShape,
    owner: VaultOwner,
) -> AcceptedVaultMove {
    let occupant_stack = occupied.map_or(0, |o| o.stack_size);
    let (source_after, target_after) = match shape {
        MoveShape::Whole => (0, source.stack_size),
        MoveShape::Split => (source.stack_size - quantity, quantity),
        MoveShape::Merge => (source.stack_size - quantity, occupant_stack + quantity),
        // The occupant takes the source slot.
        MoveShape::Swap => (occupant_stack, source.stack_size),
    };
    AcceptedVaultMove {
        owner,
        entity_id: req.entity_id,
        player_id: req.player_id,
        item_id: req.item_id,
        type_id: source.type_id,
        quantity,
        source_container_id: source.container_id,
        source_slot_id: source.slot_id,
        target_container_id: req.target_container_id,
        target_slot_id: req.target_slot_id,
        kind: move_kind(source.container_id, req.target_container_id, shape),
        source_stack_before: source.stack_size,
        source_stack_after: source_after,
        target_stack_before: occupant_stack,
        target_stack_after: target_after,
    }
}
