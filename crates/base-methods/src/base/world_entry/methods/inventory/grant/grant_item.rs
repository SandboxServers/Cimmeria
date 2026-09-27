//! The generic item grant behind loot pickup, the content engine's
//! `grant_item` and GM `gmGiveItem`: placement → vault guard → transaction
//! → inventory resync → equipment epilogue.
//!
//! [`handle_loot_grant`] is the loot pickup's entry: the cell has already
//! taken the item off the corpse, so a grant that commits nothing is
//! answered with `BaseToCellMsg::LootGrantRefused` and the cell puts the
//! item back.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::core::send_full_inventory_update;
use super::equip_epilogue::equip_epilogue;
use super::persist::{persist_grant, PersistOutcome};
use super::placement::{format_container_sets, resolve_placement};
use crate::base::gm_feedback::send_gm_feedback_to_client;
use crate::base::outbox;
use crate::base::ConnectedClientState;
use crate::cell::messages::{BaseToCellMsg, GrantRefusal, LootGrantSource};

/// How a grant ended, for callers that must act on a refusal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GrantOutcome {
    Committed,
    /// Nothing committed. `container_id` is where the grant was going.
    Refused {
        reason: GrantRefusal,
        container_id: i32,
        account_id: Option<i32>,
    },
    /// The commit's outcome is unknown; the item may have been granted.
    CommitUnknown {
        account_id: Option<i32>,
    },
}

/// Persist an item grant to inventory and sync client appearance.
pub async fn handle_grant_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    container_id: i32,
    count: i32,
    notify_gm: bool,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    grant(
        entity_id,
        player_id,
        item_id,
        container_id,
        count,
        notify_gm,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// Grant a looted item. A refusal sends the item back to its corpse; a
/// commit whose outcome is unknown does not, because the item may already
/// be in the looter's inventory and a second copy would be a duplicate.
pub async fn handle_loot_grant(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    container_id: i32,
    count: i32,
    source: LootGrantSource,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let outcome = grant(
        entity_id,
        player_id,
        item_id,
        container_id,
        count,
        false,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
    match outcome {
        GrantOutcome::Committed => {}
        GrantOutcome::Refused {
            reason,
            container_id,
            account_id,
        } => {
            let msg = BaseToCellMsg::LootGrantRefused {
                entity_id,
                player_id,
                source,
                design_id: item_id,
                quantity: count,
                container_id,
                reason,
            };
            let sent = match cell_tx {
                Some(tx) => tx.send(msg).await.is_ok(),
                None => false,
            };
            if !sent {
                tracing::warn!(
                    target: "inventory",
                    event = "loot_restore_failed",
                    account_id,
                    player_id,
                    entity_id,
                    corpse_id = source.corpse_id,
                    index = source.index,
                    type_id = item_id,
                    qty = count,
                    refusal = reason.as_str(),
                    reason = "cell_channel_closed",
                    "loot_restore_failed: the refused item could not be handed back to the cell and is lost"
                );
            }
        }
        GrantOutcome::CommitUnknown { account_id } => {
            tracing::warn!(
                target: "inventory",
                event = "loot_restore_skipped",
                account_id,
                player_id,
                entity_id,
                corpse_id = source.corpse_id,
                index = source.index,
                type_id = item_id,
                qty = count,
                reason = "commit_outcome_unknown",
                "loot_restore_skipped: the grant's commit outcome is unknown, so the item stays off the corpse"
            );
        }
    }
}

#[tracing::instrument(
    name = "inventory.grant_item",
    level = "info",
    skip_all,
    fields(entity_id, player_id, item_id, container_id, count)
)]
async fn grant(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    container_id: i32,
    count: i32,
    notify_gm: bool,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> GrantOutcome {
    tracing::debug!(
        entity_id,
        player_id,
        item_id,
        container_id,
        count,
        cell_tx_present = cell_tx.is_some(),
        "handle_grant_item: entered"
    );
    let gm_refusal = |text: String| async move {
        if notify_gm {
            send_gm_feedback_to_client(entity_id, &text, transport, connected, entity_to_addr)
                .await;
        }
    };
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "GrantItem: no DB pool");
            return GrantOutcome::Refused {
                reason: GrantRefusal::NoDatabase,
                container_id,
                account_id: None,
            };
        }
    };

    let placement = match resolve_placement(pool, player_id, item_id, container_id).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                target: "inventory",
                event = "lookup_failed",
                phase = "placement",
                player_id,
                entity_id,
                type_id = item_id,
                requested_container_id = container_id,
                error = %e,
                "lookup_failed: could not read the item's container_sets; grant refused"
            );
            gm_refusal(format!(
                "gmGiveItem: could not give item {item_id}: the item lookup failed"
            ))
            .await;
            return GrantOutcome::Refused {
                reason: GrantRefusal::DatabaseError,
                container_id,
                account_id: None,
            };
        }
    };
    let account_id = placement.account_id;
    let target = placement.container_id;

    if super::validation::refuse_storage_grant(pool, entity_id, player_id, item_id, target, count)
        .await
    {
        gm_refusal(format!(
            "gmGiveItem: item {item_id} can only be stored in a vault, not carried"
        ))
        .await;
        return GrantOutcome::Refused {
            reason: GrantRefusal::StorageOnly,
            container_id: target,
            account_id,
        };
    }

    let committed = match persist_grant(pool, entity_id, player_id, item_id, target, count).await {
        PersistOutcome::Committed(c) => c,
        PersistOutcome::Refused(reason) => {
            tracing::info!(
                target: "inventory",
                event = "grant_refused",
                account_id,
                player_id,
                entity_id,
                type_id = item_id,
                quantity = count,
                container_id = target,
                reason = reason.as_str(),
                "grant_refused: nothing was written"
            );
            let why = match reason {
                GrantRefusal::ContainerFull => "that bag is full",
                _ => "the inventory write failed",
            };
            gm_refusal(format!("gmGiveItem: could not give item {item_id}: {why}")).await;
            return GrantOutcome::Refused {
                reason,
                container_id: target,
                account_id,
            };
        }
        PersistOutcome::CommitUnknown => {
            tracing::warn!(
                target: "inventory",
                event = "grant_outcome_unknown",
                account_id,
                player_id,
                entity_id,
                type_id = item_id,
                quantity = count,
                container_id = target,
                reason = "commit_outcome_unknown",
                "grant_outcome_unknown: the commit failed without a server answer"
            );
            return GrantOutcome::CommitUnknown { account_id };
        }
    };

    tracing::info!(
        target: "inventory",
        event = "grant_container_chosen",
        account_id,
        player_id,
        entity_id,
        type_id = item_id,
        quantity = count,
        container_sets = %format_container_sets(&placement.container_sets),
        requested_container_id = placement.requested,
        skipped_storage = placement.skipped_storage,
        container_id = committed.container_id,
        slot_id = committed.slot_id,
        qty_before = committed.qty_before,
        qty_after = committed.qty_after,
        "grant_container_chosen"
    );

    let total_items = send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
    tracing::debug!(
        entity_id,
        player_id,
        item_id,
        total_items,
        "Sent full onUpdateItem to client"
    );

    // Definitive GM feedback: the grant committed above, so every later
    // step is after the write has landed.
    if notify_gm {
        send_gm_feedback_to_client(
            entity_id,
            &format!("gmGiveItem: gave {count}x item {item_id}"),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }

    if let Some(tx) = cell_tx {
        outbox::try_dispatch_now(
            pool.as_ref(),
            tx,
            committed.outbox_id,
            entity_id,
            committed.outbox_payload,
        )
        .await;
    }

    // A merge never lands in an equipment slot (weapons do not stack).
    if let Some(instance_id) = committed.instance_id {
        equip_epilogue(
            entity_id,
            player_id,
            item_id,
            committed.container_id,
            committed.slot_id,
            instance_id,
            committed.bandolier_became_active,
            db_pool,
            cell_tx,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }
    GrantOutcome::Committed
}
