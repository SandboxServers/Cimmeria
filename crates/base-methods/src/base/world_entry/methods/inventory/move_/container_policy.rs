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

use super::super::core::{send_full_inventory_update, send_full_inventory_update_via};
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

/// Log a refused move under the `bank` target and resync the player's
/// inventory, so the client snaps the dragged item back to where the server
/// still has it. The legacy server did the same (`Inventory.py:377-382`:
/// mark the item dirty, then flush). No `onErrorCode` exists for "cannot
/// move item", so the snap-back is the whole of the feedback.
///
/// The caller has already rolled back any transaction it opened.
pub(super) async fn refuse_move(
    end: MoveEnd,
    verdict: Movable,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    source_container_id: Option<i32>,
    target_container_id: i32,
    target_slot_id: i32,
    pool: &Arc<PgPool>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::warn!(
        target: "bank",
        entity_id,
        player_id,
        item_id,
        source_container_id,
        target_container_id,
        target_slot_id,
        reason = refusal_reason(end, verdict),
        "move_rejected: container is not player-movable; item stays put, client resynced"
    );
    resync_under_move_lock(
        entity_id,
        player_id,
        pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// Send the refusal's snapshot while holding the per-player move lock, the
/// `(player_id, 0)` advisory lock every move takes before it touches a row.
///
/// Reading the snapshot outside the lock let it go stale against a move that
/// was committing at the same moment: that move's own update could reach the
/// client first, and the older refusal snapshot then undid it on screen.
/// Under the lock the snapshot includes every move committed before it, and
/// no later move can commit until the snapshot has been sent. The
/// transaction only reads, so it is rolled back to release the lock.
///
/// If the lock cannot be taken, the resync is still sent without it: the
/// player must see the snap-back.
async fn resync_under_move_lock(
    entity_id: u32,
    player_id: i32,
    pool: &Arc<PgPool>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let locked = match pool.begin().await {
        Ok(mut tx) => match sqlx::query("SELECT pg_advisory_xact_lock($1, 0)")
            .bind(player_id)
            .execute(&mut *tx)
            .await
        {
            Ok(_) => Some(tx),
            Err(e) => {
                let _ = tx.rollback().await;
                tracing::warn!(
                    target: "bank",
                    player_id,
                    "move_rejected: move lock failed, resyncing without it: {e}"
                );
                None
            }
        },
        Err(e) => {
            tracing::warn!(
                target: "bank",
                player_id,
                "move_rejected: begin failed, resyncing without the move lock: {e}"
            );
            None
        }
    };

    match locked {
        Some(mut tx) => {
            send_full_inventory_update_via(
                entity_id,
                player_id,
                &mut *tx,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
            let _ = tx.rollback().await;
        }
        None => {
            send_full_inventory_update(
                entity_id,
                player_id,
                pool,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
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
