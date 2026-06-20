//! Inventory / bandolier / persisted-options dispatch arms.
//!
//! Each function is the body of one `CellToBaseMsg` arm carved out of the
//! [`super::handle_cell_message`] match. These route inventory mutations and
//! per-player option/state persistence into the existing
//! [`super::super::methods`] handlers and the sibling
//! [`super::bandolier`] / [`super::system_options`] / [`super::state_field`]
//! modules.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::cell::messages::CellToBaseMsg;

use super::super::super::ConnectedClientState;
use super::super::methods::{
    handle_move_inventory_item, handle_recharge_inventory_items, handle_remove_inventory_item,
    handle_remove_inventory_item_by_type, handle_repair_inventory_item,
    handle_repair_inventory_items, handle_use_inventory_item, send_full_inventory_resync,
};
use super::{bandolier, state_field, system_options, DispatchCtx};

/// Route the inventory / bandolier / persisted-options family of
/// `CellToBaseMsg`.
///
/// Called by [`super::handle_cell_message`] once the outer match has narrowed
/// `msg` to this family; any other variant is `unreachable!` by construction.
pub(super) async fn route(msg: CellToBaseMsg, ctx: &DispatchCtx<'_>) {
    match msg {
        CellToBaseMsg::ListInventoryItems {
            entity_id,
            player_id,
        } => {
            list_inventory_items(
                entity_id,
                player_id,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::MoveInventoryItem {
            entity_id,
            player_id,
            item_id,
            target_container_id,
            target_slot_id,
            quantity,
        } => {
            move_inventory_item(
                entity_id,
                player_id,
                item_id,
                target_container_id,
                target_slot_id,
                quantity,
                ctx.db_pool,
                ctx.cell_tx,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::RemoveInventoryItem {
            entity_id,
            player_id,
            item_id,
            quantity,
            notify_gm,
        } => {
            remove_inventory_item(
                entity_id,
                player_id,
                item_id,
                quantity,
                notify_gm,
                ctx.db_pool,
                ctx.cell_tx,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::UseInventoryItem {
            entity_id,
            player_id,
            item_id,
            target_id,
        } => {
            use_inventory_item(
                entity_id,
                player_id,
                item_id,
                target_id,
                ctx.db_pool,
                ctx.cell_tx,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::RemoveInventoryItemByType {
            entity_id,
            player_id,
            type_id,
            count,
        } => {
            remove_inventory_item_by_type(
                entity_id,
                player_id,
                type_id,
                count,
                ctx.db_pool,
                ctx.cell_tx,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::RepairInventoryItem {
            entity_id,
            player_id,
            item_id,
            repair_ratio,
        } => {
            repair_inventory_item(
                entity_id,
                player_id,
                item_id,
                repair_ratio,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::RepairInventoryItems {
            entity_id,
            player_id,
            item_ids,
            vendor_template_id,
        } => {
            repair_inventory_items(
                entity_id,
                player_id,
                item_ids,
                vendor_template_id,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::RechargeInventoryItems {
            entity_id,
            player_id,
            item_ids,
            vendor_template_id,
        } => {
            recharge_inventory_items(
                entity_id,
                player_id,
                item_ids,
                vendor_template_id,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::ActiveSlotUpdate {
            entity_id,
            player_id,
            slot_id,
        } => {
            active_slot_update(
                entity_id,
                player_id,
                slot_id,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::SystemOptionsUpdate {
            player_id,
            auto_reload,
            reload_on_activate,
        } => system_options_update(player_id, auto_reload, reload_on_activate, ctx.db_pool).await,
        CellToBaseMsg::StateFieldUpdate {
            player_id,
            state_field,
        } => state_field_update(player_id, state_field, ctx.db_pool).await,
        CellToBaseMsg::RefreshAppearance {
            entity_id,
            player_id,
            holstered,
        } => {
            refresh_appearance(
                entity_id,
                player_id,
                holstered,
                ctx.db_pool,
                ctx.transport,
                ctx.connected,
                ctx.entity_to_addr,
            )
            .await
        }
        CellToBaseMsg::BandolierAmmoUpdate {
            player_id,
            slot_id,
            expected_instance_id,
            current_ammo,
            cur_ammo_type,
        } => {
            bandolier_ammo_update(
                player_id,
                slot_id,
                expected_instance_id,
                current_ammo,
                cur_ammo_type,
                ctx.db_pool,
            )
            .await
        }
        other => unreachable!("inventory_dispatch::route got non-inventory variant: {other:?}"),
    }
}

/// `CellToBaseMsg::ListInventoryItems` — post-`ReanchorPlayer` resync.
pub(super) async fn list_inventory_items(
    entity_id: u32,
    player_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // `ListInventoryItems` is the post-`ReanchorPlayer` resync
    // path. Run the FULL re-init bundle (bag info + active slot
    // + cash + items), not just the item snapshot — the new
    // pawn's `InventoryComponent` is empty after pawn-recreate
    // and needs the bag list registered before `onUpdateItem`
    // entries can slot. See `send_full_inventory_resync` doc
    // for the bug shape this guards against.
    if let Some(pool) = &db_pool {
        send_full_inventory_resync(
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

/// `CellToBaseMsg::MoveInventoryItem`.
pub(super) async fn move_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    target_container_id: i32,
    target_slot_id: i32,
    quantity: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<tokio::sync::mpsc::Sender<crate::cell::messages::BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_move_inventory_item(
        entity_id,
        player_id,
        item_id,
        target_container_id,
        target_slot_id,
        quantity,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::RemoveInventoryItem`.
pub(super) async fn remove_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    quantity: i32,
    notify_gm: bool,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<tokio::sync::mpsc::Sender<crate::cell::messages::BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_remove_inventory_item(
        entity_id,
        player_id,
        item_id,
        quantity,
        notify_gm,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::UseInventoryItem`.
pub(super) async fn use_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    target_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<tokio::sync::mpsc::Sender<crate::cell::messages::BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_use_inventory_item(
        entity_id,
        player_id,
        item_id,
        target_id,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::RemoveInventoryItemByType`.
pub(super) async fn remove_inventory_item_by_type(
    entity_id: u32,
    player_id: i32,
    type_id: i32,
    count: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<tokio::sync::mpsc::Sender<crate::cell::messages::BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_remove_inventory_item_by_type(
        entity_id,
        player_id,
        type_id,
        count,
        db_pool,
        cell_tx,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::RepairInventoryItem`.
pub(super) async fn repair_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    repair_ratio: f32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    handle_repair_inventory_item(
        entity_id,
        player_id,
        item_id,
        repair_ratio,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::RepairInventoryItems`.
pub(super) async fn repair_inventory_items(
    entity_id: u32,
    player_id: i32,
    item_ids: Vec<i32>,
    vendor_template_id: Option<i32>,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Route through the wrapper so the `None` (free repair) path is
    // reachable. The wrapper dispatches to handle_paid_repair when a
    // template id is supplied and to the free-repair UPDATE otherwise.
    handle_repair_inventory_items(
        entity_id,
        player_id,
        item_ids,
        vendor_template_id,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::RechargeInventoryItems`.
pub(super) async fn recharge_inventory_items(
    entity_id: u32,
    player_id: i32,
    item_ids: Vec<i32>,
    vendor_template_id: Option<i32>,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Route through the wrapper so the `None` (free recharge) path is
    // reachable. The wrapper dispatches to handle_paid_recharge when a
    // template id is supplied and to the free-recharge UPDATE otherwise.
    handle_recharge_inventory_items(
        entity_id,
        player_id,
        item_ids,
        vendor_template_id,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::ActiveSlotUpdate`.
pub(super) async fn active_slot_update(
    entity_id: u32,
    player_id: i32,
    slot_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    bandolier::active_slot_update(
        entity_id,
        player_id,
        slot_id,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::SystemOptionsUpdate`.
pub(super) async fn system_options_update(
    player_id: i32,
    auto_reload: bool,
    reload_on_activate: bool,
    db_pool: &Option<Arc<PgPool>>,
) {
    system_options::persist_system_options(player_id, auto_reload, reload_on_activate, db_pool)
        .await;
}

/// `CellToBaseMsg::StateFieldUpdate`.
pub(super) async fn state_field_update(
    player_id: i32,
    state_field: u32,
    db_pool: &Option<Arc<PgPool>>,
) {
    state_field::persist_state_field(player_id, state_field, db_pool).await;
}

/// `CellToBaseMsg::RefreshAppearance`.
pub(super) async fn refresh_appearance(
    entity_id: u32,
    player_id: i32,
    holstered: bool,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    bandolier::refresh_appearance(
        entity_id,
        player_id,
        holstered,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// `CellToBaseMsg::BandolierAmmoUpdate`.
pub(super) async fn bandolier_ammo_update(
    player_id: i32,
    slot_id: i32,
    expected_instance_id: i32,
    current_ammo: i32,
    cur_ammo_type: i32,
    db_pool: &Option<Arc<PgPool>>,
) {
    bandolier::bandolier_ammo_update(
        player_id,
        slot_id,
        expected_instance_id,
        current_ammo,
        cur_ammo_type,
        db_pool,
    )
    .await;
}
