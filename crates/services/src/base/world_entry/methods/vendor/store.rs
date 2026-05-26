use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::super::helpers::send_to_witness_reliable;
use super::super::super::super::ConnectedClientState;
use super::data::{
    load_store_buy_items, load_vendor_buyback_prices, load_vendor_recharge_prices,
    load_vendor_repair_prices, load_vendor_sell_prices,
};
use super::serializers::{
    serialize_empty_store_open, serialize_store_open, serialize_store_update, StoreItemCostUpdate,
};
use crate::mercury::{build_player_entity_method_packet, method_idx};

#[derive(sqlx::FromRow)]
pub struct VendorTemplateLists {
    pub buy_item_list: Option<i32>,
    pub sell_item_list: Option<i32>,
    pub repair_item_list: Option<i32>,
    pub recharge_item_list: Option<i32>,
}

/// Open a vendor store for a player, loading all item lists and pricing.
#[tracing::instrument(
    name = "vendor.open_store",
    level = "info",
    skip_all,
    fields(entity_id, player_id, vendor_entity_id, ?vendor_template_id),
)]
pub async fn handle_open_vendor_store(
    entity_id: u32,
    player_id: i32,
    vendor_entity_id: i32,
    vendor_template_id: Option<i32>,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let Some(pool) = db_pool else {
        send_store_open_to_client(
            entity_id,
            vendor_entity_id,
            serialize_empty_store_open(vendor_entity_id),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        tracing::debug!(entity_id, vendor_entity_id, "OpenVendorStore: no DB pool");
        return;
    };

    let Some(template_id) = vendor_template_id else {
        send_store_open_to_client(
            entity_id,
            vendor_entity_id,
            serialize_empty_store_open(vendor_entity_id),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        tracing::debug!(
            entity_id,
            vendor_entity_id,
            "OpenVendorStore: vendor template missing"
        );
        return;
    };

    let template = match sqlx::query_as::<_, VendorTemplateLists>(
        "SELECT buy_item_list, sell_item_list, repair_item_list, recharge_item_list \
         FROM resources.entity_templates WHERE template_id = $1",
    )
    .bind(template_id)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => {
            tracing::warn!(template_id, "OpenVendorStore: template not found");
            VendorTemplateLists {
                buy_item_list: None,
                sell_item_list: None,
                repair_item_list: None,
                recharge_item_list: None,
            }
        }
        Err(e) => {
            tracing::error!(template_id, "OpenVendorStore: template query failed: {e}");
            VendorTemplateLists {
                buy_item_list: None,
                sell_item_list: None,
                repair_item_list: None,
                recharge_item_list: None,
            }
        }
    };

    let buy_items = load_store_buy_items(pool, template.buy_item_list).await;
    let sell_prices = load_vendor_sell_prices(pool, player_id, template.sell_item_list).await;
    let buyback_prices = load_vendor_buyback_prices(pool, player_id).await;
    let repair_prices = load_vendor_repair_prices(pool, player_id, template.repair_item_list).await;
    let recharge_prices =
        load_vendor_recharge_prices(pool, player_id, template.recharge_item_list).await;

    let args = serialize_store_open(
        vendor_entity_id,
        &buy_items,
        &sell_prices,
        &buyback_prices,
        &repair_prices,
        &recharge_prices,
    );

    let buy_count = buy_items.len();
    let sell_count = sell_prices.len();
    let buyback_count = buyback_prices.len();
    let repair_count = repair_prices.len();
    let recharge_count = recharge_prices.len();

    send_store_open_to_client(
        entity_id,
        vendor_entity_id,
        args,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
    tracing::debug!(
        entity_id,
        player_id,
        vendor_entity_id,
        template_id,
        buy_count,
        sell_count,
        buyback_count,
        repair_count,
        recharge_count,
        "OpenVendorStore: sent"
    );
}

/// Send vendor store open to client.
pub async fn send_store_open_to_client(
    entity_id: u32,
    vendor_entity_id: i32,
    args: Vec<u8>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_STORE_OPEN,
                &args,
            )
        },
    )
    .await;
    tracing::trace!(entity_id, vendor_entity_id, "Sent onStoreOpen");
}

/// Send vendor store price updates to client.
pub async fn send_store_update_to_client(
    entity_id: u32,
    updates: &[StoreItemCostUpdate],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    if updates.is_empty() {
        return;
    }

    let args = serialize_store_update(updates);
    send_to_witness_reliable(
        transport,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_player_entity_method_packet(
                key,
                seq,
                acks,
                entity_id,
                method_idx::ON_STORE_UPDATE,
                &args,
            )
        },
    )
    .await;
}
