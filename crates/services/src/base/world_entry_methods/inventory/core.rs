use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_entity_method_packet, method_idx};
use super::super::super::helpers::send_to_witness;
use super::super::super::ConnectedClientState;
use super::super::vendor::helpers::sync_bandolier_after_inventory_change;

const INVENTORY_ITEM_SELECT: &str = r#"
SELECT inv.item_id, inv.type_id, inv.stack_size, inv.slot_id, inv.container_id,
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
WHERE inv.character_id = $1
ORDER BY inv.container_id, inv.slot_id
"#;

#[derive(sqlx::FromRow)]
struct InventoryRow {
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

#[derive(sqlx::FromRow)]
struct InventoryInstanceRow {
    type_id: i32,
    stack_size: i32,
    container_id: i32,
    slot_id: i32,
    bound: bool,
    durability: i32,
    charges: i32,
}

/// Send full inventory update to player, refreshing all items on the client.
pub async fn send_full_inventory_update(
    entity_id: u32,
    player_id: i32,
    pool: &Arc<PgPool>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> usize {
    let all_items: Vec<InventoryRow> = sqlx::query_as::<_, InventoryRow>(INVENTORY_ITEM_SELECT)
        .bind(player_id)
        .fetch_all(pool.as_ref())
        .await
        .unwrap_or_default();

    let mut args = Vec::with_capacity(4 + all_items.len() * 48);
    args.extend_from_slice(&(all_items.len() as u32).to_le_bytes());
    for row in all_items.iter() {
        let item = cimmeria_entity::inventory::InvItem {
            id: row.item_id,
            dbid: row.type_id,
            stack_size: row.stack_size,
            slot_id: row.slot_id + 1,
            container_id: row.container_id,
            is_bound: row.bound,
            durability: row.durability,
            ammo_types: row.ammo_type_ids.clone(),
            cur_ammo_type: row.cur_ammo_type_id,
            charges: row.charges,
        };
        item.serialize(&mut args);
    }

    send_to_witness(
        socket,
        connected,
        entity_to_addr,
        entity_id,
        |key, seq, acks| {
            build_entity_method_packet(key, seq, acks, entity_id, method_idx::ON_UPDATE_ITEM, &args)
        },
    )
    .await;

    all_items.len()
}

/// Remove an inventory item from player inventory and sync client.
pub async fn handle_remove_inventory_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    quantity: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "RemoveInventoryItem: no DB pool");
            return;
        }
    };

    if quantity <= 0 {
        tracing::warn!(
            player_id,
            item_id,
            quantity,
            "RemoveInventoryItem: invalid quantity"
        );
        return;
    }

    let source = sqlx::query_as::<_, InventoryInstanceRow>(
        "SELECT type_id, stack_size, container_id, slot_id, bound, durability, charges \
         FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 LIMIT 1",
    )
    .bind(player_id)
    .bind(item_id)
    .fetch_optional(pool.as_ref())
    .await
    .ok()
    .flatten();

    let Some(source) = source else {
        tracing::warn!(
            player_id,
            item_id,
            "RemoveInventoryItem: source item not found"
        );
        return;
    };

    let removed_all = quantity >= source.stack_size;
    let result = if removed_all {
        sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1 AND item_id = $2")
            .bind(player_id)
            .bind(item_id)
            .execute(pool.as_ref())
            .await
    } else {
        sqlx::query(
            "UPDATE sgw_inventory SET stack_size = stack_size - $1 \
             WHERE character_id = $2 AND item_id = $3 AND stack_size > $1",
        )
        .bind(quantity)
        .bind(player_id)
        .bind(item_id)
        .execute(pool.as_ref())
        .await
    };

    match result {
        Ok(r) if r.rows_affected() == 1 => {}
        Ok(_) => {
            tracing::warn!(player_id, item_id, "RemoveInventoryItem: no rows changed");
            return;
        }
        Err(e) => {
            tracing::error!(
                player_id,
                item_id,
                "RemoveInventoryItem: update failed: {e}"
            );
            return;
        }
    }

    let total_items = send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        socket,
        connected,
        entity_to_addr,
    )
    .await;

    tracing::debug!(
        entity_id,
        player_id,
        item_id,
        quantity,
        total_items,
        "Inventory remove persisted"
    );

    if removed_all {
        if let Some(cell_tx) = cell_tx {
            let _ = cell_tx
                .send(BaseToCellMsg::InventoryItemRemoved {
                    entity_id,
                    item_id,
                    source_container_id: source.container_id,
                })
                .await;
        }
    }

    if source.container_id == 3 {
        sync_bandolier_after_inventory_change(
            entity_id,
            player_id,
            db_pool,
            cell_tx,
            socket,
            connected,
            entity_to_addr,
        )
        .await;
    }
}