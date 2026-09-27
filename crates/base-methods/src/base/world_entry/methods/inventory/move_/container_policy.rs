//! Which containers a client-driven `moveItem` may read from or write to.
//!
//! An explicit allowlist, checked for both the source and the target
//! container (D-BV07). Before it, the move path checked only the target, and
//! only through `bag_max_slots`, so any container with a capacity was a legal
//! target and every container was a legal source. That let a sold item be
//! dragged back out of the vendor buyback bag (16) without paying.
//!
//! Every container a player could move into or out of before BV-01 is still
//! `Yes`: 1 (main), 2 (mission), 3 (bandolier), 4-14 (equipment) and 15
//! (crafting, which the crafting flow needs in both directions with 1).
//! Everything else is refused, and unknown ids are refused by default, so a
//! future container never becomes movable by accident.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::inventory::{
    INV_ARTIFACT2, INV_AUCTION, INV_BANK, INV_BUYBACK, INV_COMMAND_BANK, INV_CRAFTING, INV_MAIN,
    INV_TEAM_BANK,
};
use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::core::send_inventory_item_update_via;
use crate::base::ConnectedClientState;

/// Whether a player may move an item into or out of a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Movable {
    /// Always movable.
    Yes,
    /// Never movable by the player. The container is owned by a service
    /// (vendor buyback, auction escrow, org vaults) that moves items itself.
    No,
    /// Movable only while the player has a vault session open. BV-01 has no
    /// session, so the move path treats this as [`Movable::No`]; BV-03
    /// wires it to the session.
    VaultSession,
}

/// The allowlist. See the module docs.
pub(crate) fn player_movable(container_id: i32) -> Movable {
    match container_id {
        // 1 main, 2 mission, 3 bandolier, 4-14 equipment, 15 crafting.
        INV_MAIN..=INV_ARTIFACT2 | INV_CRAFTING => Movable::Yes,
        // Buyback leaves only through `buybackItems`, which charges.
        INV_BUYBACK => Movable::No,
        INV_BANK => Movable::VaultSession,
        // Auction escrow and the Team and Command vaults: refused until
        // their own waves.
        INV_AUCTION | INV_TEAM_BANK | INV_COMMAND_BANK => Movable::No,
        _ => Movable::No,
    }
}

/// Which end of the move failed the allowlist.
#[derive(Debug, Clone, Copy)]
pub(super) enum MoveEnd {
    Source,
    Target,
}

/// The `reason` field of a refused move. Stable: tests and log queries pin
/// these strings.
fn refusal_reason(end: MoveEnd, movable: Movable) -> &'static str {
    match (end, movable) {
        (MoveEnd::Source, Movable::VaultSession) => "source_container_needs_vault_session",
        (MoveEnd::Target, Movable::VaultSession) => "target_container_needs_vault_session",
        (MoveEnd::Source, _) => "source_container_not_player_movable",
        (MoveEnd::Target, _) => "target_container_not_player_movable",
    }
}

/// `Some(verdict)` when `container_id` is not movable right now. BV-01 has
/// no vault session, so `VaultSession` refuses too.
pub(super) fn refusal(container_id: i32) -> Option<Movable> {
    match player_movable(container_id) {
        Movable::Yes => None,
        verdict @ (Movable::No | Movable::VaultSession) => Some(verdict),
    }
}

/// Who and what a refused move was about, read at refusal time.
#[derive(Debug, Default, sqlx::FromRow)]
struct RefusalContext {
    account_id: Option<i32>,
    type_id: Option<i32>,
    stack_size: Option<i32>,
    source_container_id: Option<i32>,
    source_slot_id: Option<i32>,
    /// The lookup itself failed (logged); ownership is unknown.
    #[sqlx(skip)]
    lookup_failed: bool,
}

/// Refuse a move: log it under the `bank` target, then resend the dragged
/// item so the client snaps it back to where the server still has it. The
/// legacy server did the same (`Inventory.py:377-382`: mark the item dirty,
/// then flush). No `onErrorCode` exists for "cannot move item", so the
/// snap-back is the whole of the feedback.
///
/// Both happen under two locks, held until the packet has gone:
///
/// - the per-player move lock, the `(player_id, 0)` advisory lock every move
///   takes before it touches a row, so a move of the same item that is
///   committing at the same moment finishes first;
/// - a `FOR UPDATE` lock on the refused item's row. The other inventory
///   writers do not take the move lock: a grant that merges into an existing
///   stack locks `(player_id, container_id)`, and removes, uses, vendor sales
///   and trades lock only the row. Every one of them row-locks the row before
///   changing it (an `UPDATE` or `DELETE` does so itself), so a write already
///   in flight commits before the read, and a later one waits until the
///   packet has gone and sends its own update after it.
///
/// So the log and the packet both show the committed row, and no write to
/// that row can overtake the packet. Only the named item is resent, never
/// the whole inventory: see [`send_inventory_item_update_via`].
///
/// If either lock cannot be taken (a `lock_timeout`, or the transaction
/// cannot begin), the refusal is still logged but nothing is resent: an
/// unlocked read could be overtaken by a concurrent write's own update and
/// undo it on the client. The client keeps its optimistic position until
/// the next authoritative update of that item, and `move_resync_skipped
/// reason=lock_timeout` records the skip.
///
/// `quantity` is the requested quantity as sent (`<= 0` means the whole
/// stack). The caller has already rolled back any transaction it opened.
pub(super) async fn refuse_move(
    end: MoveEnd,
    verdict: Movable,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    quantity: i32,
    target_container_id: i32,
    target_slot_id: i32,
    pool: &Arc<PgPool>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let refusal = Refusal {
        reason: refusal_reason(end, verdict),
        entity_id,
        player_id,
        item_id,
        quantity,
        target_container_id,
        target_slot_id,
    };
    match take_move_lock(pool, entity_id, player_id, item_id).await {
        Some(mut tx) => {
            let context = refusal_context(&mut *tx, entity_id, player_id, item_id).await;
            if refusal.log(&context) {
                refusal
                    .resend(&mut *tx, &context, transport, connected, entity_to_addr)
                    .await;
            }
            if let Err(e) = tx.rollback().await {
                tracing::warn!(
                    target: "bank",
                    event = "move_rejected",
                    account_id = context.account_id,
                    player_id,
                    entity_id,
                    item_id,
                    reason = "move_lock_release_failed",
                    "move_rejected: rolling back the read-only lock transaction failed: {e}"
                );
            }
        }
        None => {
            // The context read is only for the log, so it needs no lock.
            let context = refusal_context(pool.as_ref(), entity_id, player_id, item_id).await;
            if refusal.log(&context) {
                refusal.skip_unlocked_resend(&context);
            }
        }
    }
}

/// The fields of one refused move that the caller knows.
struct Refusal {
    reason: &'static str,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    quantity: i32,
    target_container_id: i32,
    target_slot_id: i32,
}

impl Refusal {
    /// Emit `move_rejected`. Returns `true` when the player owns the item,
    /// so there is something to resend; `false` after logging
    /// `move_resync_skipped` when there is not.
    fn log(&self, context: &RefusalContext) -> bool {
        tracing::warn!(
            target: "bank",
            event = "move_rejected",
            account_id = context.account_id,
            player_id = self.player_id,
            entity_id = self.entity_id,
            item_id = self.item_id,
            type_id = context.type_id,
            quantity = self.quantity,
            stack_size = context.stack_size,
            source_container_id = context.source_container_id,
            source_slot_id = context.source_slot_id,
            target_container_id = self.target_container_id,
            target_slot_id = self.target_slot_id,
            reason = self.reason,
            "move_rejected: container is not player-movable; item stays put (snap-back follows unless move_resync_skipped)"
        );
        if context.type_id.is_some() || context.lookup_failed {
            // Ownership unknown after a failed lookup: try the resend, which
            // logs its own failure.
            return true;
        }
        tracing::warn!(
            target: "bank",
            event = "move_resync_skipped",
            account_id = context.account_id,
            player_id = self.player_id,
            entity_id = self.entity_id,
            item_id = self.item_id,
            reason = "refused_item_not_owned",
            "move_resync_skipped: the refused move named an item this player does not own; \
             nothing to snap back"
        );
        false
    }

    /// The locks could not be taken, so the item is not resent (see
    /// [`refuse_move`]).
    fn skip_unlocked_resend(&self, context: &RefusalContext) {
        tracing::warn!(
            target: "bank",
            event = "move_resync_skipped",
            account_id = context.account_id,
            player_id = self.player_id,
            entity_id = self.entity_id,
            item_id = self.item_id,
            reason = "lock_timeout",
            "move_resync_skipped: the move lock or the item's row lock could not be taken; \
             not resending an unlocked read that a concurrent write could overtake"
        );
    }

    async fn resend<'c, E>(
        &self,
        executor: E,
        context: &RefusalContext,
        transport: &Arc<dyn Transport>,
        connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) where
        E: sqlx::PgExecutor<'c>,
    {
        let sent = send_inventory_item_update_via(
            self.entity_id,
            self.player_id,
            self.item_id,
            executor,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        if !sent {
            // The row was read a moment ago under the same lock, so this is
            // a failed read, which `send_inventory_item_update_via` has
            // already logged with its error.
            tracing::warn!(
                target: "bank",
                event = "move_resync_skipped",
                account_id = context.account_id,
                player_id = self.player_id,
                entity_id = self.entity_id,
                item_id = self.item_id,
                reason = "resync_read_failed",
                "move_resync_skipped: could not read the refused item back; client not resynced"
            );
        }
    }
}

/// Begin a read-only transaction holding the per-player move lock and a
/// row lock on the refused item (see [`refuse_move`]). `None`, logged, if
/// any step fails; the caller then logs the refusal without resending.
async fn take_move_lock(
    pool: &Arc<PgPool>,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
) -> Option<sqlx::Transaction<'static, sqlx::Postgres>> {
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::warn!(
                target: "bank",
                event = "move_rejected",
                player_id,
                entity_id,
                item_id,
                reason = "move_lock_begin_failed",
                "move_rejected: begin failed, not resyncing without the move lock: {e}"
            );
            return None;
        }
    };
    // The advisory lock first, then the row: the move path's order. No
    // writer waits on `(player_id, 0)` while holding a row lock, so this
    // cannot deadlock. An item the player does not own locks nothing, and
    // the refusal logs that next.
    let locked = match sqlx::query("SELECT pg_advisory_xact_lock($1, 0)")
        .bind(player_id)
        .execute(&mut *tx)
        .await
    {
        Ok(_) => {
            sqlx::query(
                "SELECT 1 FROM sgw_inventory \
                 WHERE character_id = $1 AND item_id = $2 FOR UPDATE",
            )
            .bind(player_id)
            .bind(item_id)
            .fetch_optional(&mut *tx)
            .await
        }
        Err(e) => Err(e),
    };
    match locked {
        Ok(_) => Some(tx),
        Err(e) => {
            let _ = tx.rollback().await; // Defensible silent: the lock query already failed and is logged next.
            tracing::warn!(
                target: "bank",
                event = "move_rejected",
                player_id,
                entity_id,
                item_id,
                reason = "move_lock_failed",
                "move_rejected: move or item row lock failed, not resyncing without it: {e}"
            );
            None
        }
    }
}

/// The account and the refused item's current row. `type_id` is `None`
/// when the player does not own `item_id`; every field is `None` and
/// `lookup_failed` is set when the query fails (logged).
async fn refusal_context<'c, E>(
    executor: E,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
) -> RefusalContext
where
    E: sqlx::PgExecutor<'c>,
{
    match sqlx::query_as::<_, RefusalContext>(
        "SELECT p.account_id, inv.type_id, inv.stack_size, \
                inv.container_id AS source_container_id, inv.slot_id AS source_slot_id \
         FROM sgw_player p \
         LEFT JOIN sgw_inventory inv ON inv.character_id = p.player_id AND inv.item_id = $2 \
         WHERE p.player_id = $1",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(executor)
    .await
    {
        Ok(row) => row.unwrap_or_default(),
        Err(e) => {
            tracing::warn!(
                target: "bank",
                event = "move_rejected",
                player_id,
                entity_id,
                item_id,
                reason = "refusal_context_query_failed",
                "move_rejected: could not read the refused item's context: {e}"
            );
            RefusalContext {
                lookup_failed: true,
                ..RefusalContext::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Pins the allowlist per container. A change here changes what a
    /// player can drag where, so it must be deliberate.
    #[test]
    fn player_movable_per_container() {
        for container_id in 1..=15 {
            assert_eq!(
                player_movable(container_id),
                Movable::Yes,
                "container {container_id} was movable before BV-01 and must stay movable"
            );
        }
        assert_eq!(player_movable(16), Movable::No, "buyback");
        assert_eq!(player_movable(17), Movable::VaultSession, "personal vault");
        for container_id in [18, 19, 20] {
            assert_eq!(
                player_movable(container_id),
                Movable::No,
                "container {container_id}"
            );
        }
        for unknown in [-1, 0, 21, 100] {
            assert_eq!(player_movable(unknown), Movable::No, "container {unknown}");
        }
    }

    /// Crafting depends on 1 <-> 15 in both directions (D-BV04).
    #[test]
    fn crafting_and_main_stay_movable() {
        assert_eq!(player_movable(INV_MAIN), Movable::Yes);
        assert_eq!(player_movable(INV_CRAFTING), Movable::Yes);
    }

    /// BV-01 has no vault session, so the vault refuses like any other
    /// non-movable container.
    #[test]
    fn vault_session_refuses_without_a_session() {
        assert_eq!(refusal(INV_BANK), Some(Movable::VaultSession));
        assert_eq!(refusal(INV_BUYBACK), Some(Movable::No));
        assert_eq!(refusal(INV_MAIN), None);
        assert_eq!(refusal(INV_CRAFTING), None);
    }

    #[test]
    fn refusal_reasons_are_stable() {
        assert_eq!(
            refusal_reason(MoveEnd::Source, Movable::No),
            "source_container_not_player_movable"
        );
        assert_eq!(
            refusal_reason(MoveEnd::Target, Movable::No),
            "target_container_not_player_movable"
        );
        assert_eq!(
            refusal_reason(MoveEnd::Target, Movable::VaultSession),
            "target_container_needs_vault_session"
        );
        assert_eq!(
            refusal_reason(MoveEnd::Source, Movable::VaultSession),
            "source_container_needs_vault_session"
        );
    }
}
