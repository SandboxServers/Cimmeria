//! Vendor + trade dispatch arms.
//!
//! Each function is the body of one `CellToBaseMsg` arm carved out of the
//! [`super::handle_cell_message`] match, routing vendor store open/buy/sell/
//! buyback and the atomic two-party trade swap into the existing
//! [`super::super::methods`] handlers.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};

use super::super::super::ConnectedClientState;
use super::super::methods::{
    handle_buyback_vendor_items, handle_open_vendor_store, handle_purchase_vendor_items,
    handle_sell_vendor_items,
};
use super::DispatchCtx;

/// Route the vendor + two-party-trade family of `CellToBaseMsg`.
///
/// Called by [`super::handle_cell_message`] once the outer match has narrowed
/// `msg` to this family; any other variant is `unreachable!` by construction.
pub(super) async fn route(msg: CellToBaseMsg, ctx: &DispatchCtx<'_>) {
    match msg {
        CellToBaseMsg::OpenVendorStore {
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
        } => {
            open_vendor_store(
                entity_id,
                player_id,
                vendor_entity_id,
                vendor_template_id,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::PurchaseVendorItems {
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
            items,
        } => {
            purchase_vendor_items(
                entity_id,
                player_id,
                vendor_entity_id,
                vendor_template_id,
                items,
                ctx.db_pool,
                ctx.cell_tx,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::SellVendorItems {
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
            items,
        } => {
            sell_vendor_items(
                entity_id,
                player_id,
                vendor_entity_id,
                vendor_template_id,
                items,
                ctx.db_pool,
                ctx.cell_tx,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::BuybackVendorItems {
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
            items,
        } => {
            buyback_vendor_items(
                entity_id,
                player_id,
                vendor_entity_id,
                vendor_template_id,
                items,
                ctx.db_pool,
                ctx.cell_tx,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::ExecuteTrade {
            entity_id,
            player_id,
            partner_entity_id,
            partner_player_id,
            p1_item_instance_ids,
            p1_cash,
            p2_item_instance_ids,
            p2_cash,
        } => {
            execute_trade(
                entity_id,
                player_id,
                partner_entity_id,
                partner_player_id,
                p1_item_instance_ids,
                p1_cash,
                p2_item_instance_ids,
                p2_cash,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        other => unreachable!("vendor_dispatch::route got non-vendor variant: {other:?}"),
    }
}

/// `CellToBaseMsg::OpenVendorStore`.
pub(super) async fn open_vendor_store(
    entity_id: u32,
    player_id: i32,
    vendor_entity_id: i32,
    vendor_template_id: Option<i32>,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_open_vendor_store(
        entity_id,
        player_id,
        vendor_entity_id,
        vendor_template_id,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::PurchaseVendorItems`.
pub(super) async fn purchase_vendor_items(
    entity_id: u32,
    player_id: i32,
    vendor_entity_id: i32,
    vendor_template_id: i32,
    items: Vec<(i32, i32)>,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_purchase_vendor_items(
        entity_id,
        player_id,
        vendor_entity_id,
        vendor_template_id,
        items,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::SellVendorItems`.
pub(super) async fn sell_vendor_items(
    entity_id: u32,
    player_id: i32,
    vendor_entity_id: i32,
    vendor_template_id: i32,
    items: Vec<(i32, i32)>,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_sell_vendor_items(
        entity_id,
        player_id,
        vendor_entity_id,
        vendor_template_id,
        items,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::BuybackVendorItems`.
pub(super) async fn buyback_vendor_items(
    entity_id: u32,
    player_id: i32,
    vendor_entity_id: i32,
    vendor_template_id: i32,
    items: Vec<(i32, i32)>,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_buyback_vendor_items(
        entity_id,
        player_id,
        vendor_entity_id,
        vendor_template_id,
        items,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::ExecuteTrade`.
pub(super) async fn execute_trade(
    entity_id: u32,
    player_id: i32,
    partner_entity_id: u32,
    partner_player_id: i32,
    p1_item_instance_ids: Vec<i32>,
    p1_cash: i32,
    p2_item_instance_ids: Vec<i32>,
    p2_cash: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    super::super::methods::trade::handle_execute_trade(
        entity_id,
        player_id,
        partner_entity_id,
        partner_player_id,
        p1_item_instance_ids,
        p1_cash,
        p2_item_instance_ids,
        p2_cash,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}
