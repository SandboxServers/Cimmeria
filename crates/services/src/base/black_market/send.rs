//! Shared server→client send helpers for the Black Market `onBM*` methods.
//!
//! Wraps [`send_to_witness_reliable`] with the BM method indices so the
//! create/bid/cancel handlers and the expiry sweep all emit identical packets.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;

use super::types::AuctionRow;
use super::wire;
use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// Send `onBMError(errorId)` to the entity's own client.
pub async fn send_bm_error(
    entity_id: u32,
    error_id: i32,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let args = wire::serialize_on_bm_error(error_id);
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_BM_ERROR,
                &args,
                version,
            )
        },
    )
    .await;
}

/// Send `onBMAuctionUpdate(auctionItem)` to the entity's own client.
pub async fn send_bm_auction_update(
    entity_id: u32,
    row: &AuctionRow,
    seller_name: &str,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let args = wire::serialize_on_bm_auction_update(row, seller_name);
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_BM_AUCTION_UPDATE,
                &args,
                version,
            )
        },
    )
    .await;
}

/// Send `onBMAuctionRemove(sequenceId)` to the entity's own client.
pub async fn send_bm_auction_remove(
    entity_id: u32,
    sequence_id: i32,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let args = wire::serialize_on_bm_auction_remove(sequence_id);
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, version, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_BM_AUCTION_REMOVE,
                &args,
                version,
            )
        },
    )
    .await;
}
