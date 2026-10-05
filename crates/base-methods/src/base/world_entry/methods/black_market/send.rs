//! Server→client sends for the Black Market: the `onBM*` methods, and the
//! `onRemoveItem` a listing owes the seller's client. Settlements move items
//! by mail, whose own notice tells the recipient (`payout_mail`).
//!
//! Every send logs one DEBUG `event = "bm.send"` row with the method, the
//! auction id or row count, the payload size and whether it reached the
//! wire (plan §5.1). The underlying `send_to_witness_reliable` logs its own
//! drop reasons.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use cimmeria_wire::cell::client_methods::black_market::{
    ON_BM_AUCTIONS, ON_BM_AUCTION_REMOVE, ON_BM_AUCTION_UPDATE, ON_BM_ERROR,
};

use super::types::AuctionRow;
use super::wire::{self, BMError};
use crate::base::feedback::FeedbackCtx;
use crate::base::helpers::{send_to_witness_reliable, WitnessSendOutcome};
use crate::base::{session_identity, ConnectedClientState};
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// The transport and session maps every send needs.
#[derive(Clone, Copy)]
pub struct BmNet<'a> {
    pub transport: &'a Arc<dyn Transport>,
    pub connected: &'a Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    pub entity_to_addr: &'a Arc<Mutex<HashMap<u32, SocketAddr>>>,
}

impl BmNet<'_> {
    /// The feedback context the mail notices go through.
    pub fn feedback(&self) -> FeedbackCtx<'_> {
        FeedbackCtx {
            transport: self.transport,
            connected: self.connected,
        }
    }

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
            entity_name = session_identity::identity_for_entity(
                self.connected,
                self.entity_to_addr,
                entity_id
            )
            .player_name,
            method_name = cimmeria_wire::names::player_client_method(method),
            method_index = method,
            auction_id, // nt:id-only auctions have no name column; the send carries no item
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
    net.send(entity_id, ON_BM_ERROR, &args, None, None).await;
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
        &args,
        Some(row.sequence_id),
        None,
    )
    .await;
}

/// `onBMAuctions(items, totalResults, clientKey)`; the payload was paged by
/// the caller, `rows` is how many it carries.
pub async fn send_bm_auctions(net: BmNet<'_>, entity_id: u32, args: &[u8], rows: usize) {
    net.send(entity_id, ON_BM_AUCTIONS, args, None, Some(rows))
        .await;
}

/// `onBMAuctionRemove(sequenceId)`.
pub async fn send_bm_auction_remove(net: BmNet<'_>, entity_id: u32, sequence_id: i32) {
    let args = wire::serialize_on_bm_auction_remove(sequence_id);
    net.send(
        entity_id,
        ON_BM_AUCTION_REMOVE,
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
    net.send(entity_id, method_idx::ON_REMOVE_ITEM, &args, None, Some(1))
        .await;
}
