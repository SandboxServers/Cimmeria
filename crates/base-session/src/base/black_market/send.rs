//! Server→client sends for the Black Market: the `onBM*` methods, and the
//! inventory updates a listing, a return or a delivery owes the client.
//!
//! Every send logs one DEBUG `event = "bm.send"` row with the method, the
//! auction id or row count, the payload size and whether it reached the
//! wire (plan §5.1). The underlying `send_to_witness_reliable` logs its own
//! drop reasons.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::inventory::InvItem;
use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::client_methods::black_market::{
    ON_BM_AUCTIONS, ON_BM_AUCTION_REMOVE, ON_BM_AUCTION_UPDATE, ON_BM_ERROR,
};
use sqlx::PgPool;

use super::types::AuctionRow;
use super::wire::{self, BMError};
use crate::base::helpers::{send_to_witness_reliable, WitnessSendOutcome};
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// The transport and session maps every send needs.
#[derive(Clone, Copy)]
pub struct BmNet<'a> {
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl BmNet<'_> {
    /// The live entity of an online player, `None` when offline.
    pub fn entity_of(&self, player_id: i32) -> Option<u32> {
        let clients = self.connected.lock().unwrap_or_else(|p| p.into_inner());
        clients
            .values()
            .find(|c| c.active_player_id == Some(player_id))
            .and_then(|c| c.player_entity_id)
    }

    /// Send one entity method to `entity_id`'s own client and log it.
    async fn send(
        &self,
        entity_id: u32,
        method: u16,
        name: &'static str,
        args: &[u8],
        auction_id: Option<i32>,
        rows: Option<usize>,
    ) -> bool {
        let outcome = send_to_witness_reliable(
            self.transport,
            self.connected,
            self.entity_to_addr,
            entity_id,
            |key, version, seq, acks| {
                build_player_entity_method_packet(key, seq, acks, entity_id, method, args, version)
            },
        )
        .await;
        let sent = matches!(outcome, WitnessSendOutcome::Sent { .. });
        tracing::debug!(
            event = "bm.send",
            entity_id,
            method = name,
            method_index = method,
            auction_id,
            rows,
            payload_bytes = args.len(),
            sent,
            "Black Market client send"
        );
        sent
    }
}

/// `onBMError(errorId)`.
pub async fn send_bm_error(net: BmNet<'_>, entity_id: u32, error: BMError) {
    let args = wire::serialize_on_bm_error(error);
    net.send(entity_id, ON_BM_ERROR, "onBMError", &args, None, None)
        .await;
}

/// `onBMAuctionUpdate(auctionItem)`.
pub async fn send_bm_auction_update(
    net: BmNet<'_>,
    entity_id: u32,
    row: &AuctionRow,
    seller_name: &str,
    now: i32,
) {
    let args = wire::serialize_on_bm_auction_update(row, seller_name, now);
    net.send(
        entity_id,
        ON_BM_AUCTION_UPDATE,
        "onBMAuctionUpdate",
        &args,
        Some(row.sequence_id),
        None,
    )
    .await;
}

/// `onBMAuctions(items, totalResults, clientKey)`; the payload was paged by
/// the caller, `rows` is how many it carries.
pub async fn send_bm_auctions(net: BmNet<'_>, entity_id: u32, args: &[u8], rows: usize) {
    net.send(
        entity_id,
        ON_BM_AUCTIONS,
        "onBMAuctions",
        args,
        None,
        Some(rows),
    )
    .await;
}

/// `onBMAuctionRemove(sequenceId)`.
pub async fn send_bm_auction_remove(net: BmNet<'_>, entity_id: u32, sequence_id: i32) {
    let args = wire::serialize_on_bm_auction_remove(sequence_id);
    net.send(
        entity_id,
        ON_BM_AUCTION_REMOVE,
        "onBMAuctionRemove",
        &args,
        Some(sequence_id),
        None,
    )
    .await;
}

/// `onRemoveItem([item_id])`: a listed item left the seller's bags. Moving
/// a row into container 18 sends nothing on its own, and the client would
/// keep showing the item.
pub async fn send_item_removed(net: BmNet<'_>, entity_id: u32, item_id: i32) {
    let mut args = 1u32.to_le_bytes().to_vec();
    args.extend_from_slice(&item_id.to_le_bytes());
    net.send(
        entity_id,
        method_idx::ON_REMOVE_ITEM,
        "onRemoveItem",
        &args,
        None,
        Some(1),
    )
    .await;
}

#[derive(sqlx::FromRow)]
struct ItemRow {
    item_id: i32,
    type_id: i32,
    stack_size: i32,
    slot_id: i32,
    container_id: i32,
    bound: bool,
    durability: i32,
    charges: i32,
    ammo_type_ids: Vec<i32>,
    cur_ammo_type_id: i32,
}

/// `onUpdateItem` for one returned or delivered item, read after the
/// commit. Container 18 is never sent: the filter keeps a row that is back
/// in escrow off the client.
pub async fn send_item_placed(
    net: BmNet<'_>,
    pool: &PgPool,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
) {
    let row: Option<ItemRow> = match sqlx::query_as(
        r#"SELECT inv.item_id, inv.type_id, inv.stack_size, inv.slot_id, inv.container_id,
                  inv.bound, inv.durability, inv.charges,
                  COALESCE((
                      SELECT array_agg(array_position(enum_range(NULL::resources."EAmmoType"), ammo) - 1 ORDER BY ord)
                      FROM unnest(ri.ammo_types) WITH ORDINALITY AS ammo_values(ammo, ord)
                  ), ARRAY[]::integer[]) AS ammo_type_ids,
                  CASE WHEN ri.default_ammo_type IS NULL THEN 0
                       ELSE array_position(enum_range(NULL::resources."EAmmoType"), ri.default_ammo_type) - 1
                  END AS cur_ammo_type_id
           FROM sgw_inventory inv
           LEFT JOIN resources.items ri ON ri.item_id = inv.type_id
           WHERE inv.character_id = $1 AND inv.item_id = $2 AND inv.container_id <> 18"#,
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(pool)
    .await
    {
        Ok(row) => row,
        Err(e) => {
            tracing::warn!(
                event = "bm.item_sync_failed",
                entity_id,
                player_id,
                item_id,
                reason = "inventory_read_failed",
                error = %e,
                "Black Market item update not sent; the client shows it after the next resync"
            );
            return;
        }
    };
    let Some(row) = row else {
        return;
    };
    let mut args = 1u32.to_le_bytes().to_vec();
    InvItem {
        id: row.item_id,
        dbid: row.type_id,
        stack_size: row.stack_size,
        // The wire slot is 1-based.
        slot_id: row.slot_id + 1,
        container_id: row.container_id,
        is_bound: row.bound,
        durability: row.durability,
        ammo_types: row.ammo_type_ids,
        cur_ammo_type: row.cur_ammo_type_id,
        charges: row.charges,
    }
    .serialize(&mut args);
    net.send(
        entity_id,
        method_idx::ON_UPDATE_ITEM,
        "onUpdateItem",
        &args,
        None,
        Some(1),
    )
    .await;
}
