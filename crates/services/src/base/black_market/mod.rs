//! Black Market / Auction House (`SGWBlackMarket`) base-side support.
//!
//! `SGWBlackMarket` is a ServerOnly BASE entity. Client RPCs land at the cell
//! methods 61–66 (`cell/cell_methods/black_market.rs`), which decode and forward
//! to the base via `CellToBaseMsg::BM*` variants. The base side (here) owns all
//! DB + escrow/cash/mail work and sends the `onBM*` (client indices 90–95)
//! replies back to the player's own client.
//!
//! Module map:
//! - [`types`]   — wire deserializer for `BMSearchOptions` (Phase 1) + the
//!   `AuctionRow` model + status constants.
//! - [`wire`]    — `onBM*` serializers + the x64dbg-blocked placeholders
//!   (D.1 error codes, D.5 duration→seconds, D.6 next-min-bid).
//! - [`helpers`] — reusable persistence: `send_mail_to_player`,
//!   `adjust_player_cash`, item `escrow_item` / `return_item`.
//! - [`validate`] — pure accept/reject predicates (unit-tested).
//! - [`send`]    — `onBM*` server→client send wrappers.
//! - [`search`]  — the `BMSearch` handler: queries active listings, resolves
//!   seller names, and sends `onBMAuctions` back to the requesting entity.
//! - [`create`] / [`bid`] / [`cancel`] — the create/bid/cancel state machine.
//! - [`sweep`]   — the periodic expiry-settlement background task.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use crate::base::ConnectedClientState;

pub mod bid;
pub mod cancel;
pub mod create;
pub mod helpers;
pub mod search;
pub mod seed;
pub mod send;
pub mod sweep;
pub mod types;
pub mod validate;
pub mod wire;

pub use types::BMSearchOptions;

#[cfg(test)]
mod tests;

/// Resolve the display name of the player owning `entity_id` from the connected
/// session map. Falls back to an empty string when unknown (the name is only a
/// cosmetic field on `onBMAuctionUpdate`).
fn player_name_for_entity(
    entity_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> String {
    let addr = {
        let map = entity_to_addr.lock().unwrap_or_else(|p| p.into_inner());
        map.get(&entity_id).copied()
    };
    let Some(addr) = addr else {
        return String::new();
    };
    let clients = connected.lock().unwrap_or_else(|p| p.into_inner());
    clients
        .get(&addr)
        .and_then(|c| c.player_name.clone())
        .unwrap_or_default()
}

/// Resolve a player's display name by `player_id` (DB character id) by scanning
/// the connected session map. Returns `None` when the player is offline.
fn player_name_for_player_id(
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    _entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Option<String> {
    let clients = connected.lock().unwrap_or_else(|p| p.into_inner());
    clients
        .values()
        .find(|c| c.active_player_id == Some(player_id))
        .and_then(|c| c.player_name.clone())
}

/// Resolve the live `entity_id` for an online player by `player_id`, or `None`
/// if the player is offline.
fn entity_id_for_player_id(
    player_id: i32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
) -> Option<u32> {
    let clients = connected.lock().unwrap_or_else(|p| p.into_inner());
    clients
        .values()
        .find(|c| c.active_player_id == Some(player_id))
        .and_then(|c| c.player_entity_id)
}
