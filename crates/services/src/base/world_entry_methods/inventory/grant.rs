use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::base::{ConnectedClientState, helpers, resources, world_entry_appearance};
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_entity_method_packet, method_idx};
use super::core::send_full_inventory_update;
use super::super::player_load::core::query_player_load_data;

/// Normalize item ID array: remove dupes, sort, filter invalid IDs.
pub fn normalize_item_ids(mut item_ids: Vec<i32>) -> Vec<i32> {
    item_ids.retain(|id| *id > 0);
    item_ids.sort_unstable();
    item_ids.dedup();
    item_ids
}

/// Check if an item type can be placed in a container.
pub async fn item_allows_container(pool: &Arc<PgPool>, type_id: i32, container_id: i32) -> bool {
    let container_sets: Vec<i32> = sqlx::query_scalar(
        "SELECT container_sets FROM resources.items WHERE item_id = $1"
    )
    .bind(type_id)
    .fetch_optional(pool.as_ref())
    .await
    .ok()
    .flatten()
    .unwrap_or_default();

    if container_sets.is_empty() {
        container_id == 1
    } else {
        container_sets.contains(&container_id)
    }
}

/// Persist an item grant to inventory and sync client appearance.
pub async fn handle_grant_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    container_id: i32,
    count: i32,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "GrantItem: no DB pool");
            return;
        }
    };

    let next_slot: i32 = sqlx::query_scalar(
        "SELECT COALESCE(MAX(slot_id), -1) + 1 FROM sgw_inventory \
         WHERE character_id = $1 AND container_id = $2",
    )
    .bind(player_id)
    .bind(container_id)
    .fetch_one(pool.as_ref())
    .await
    .unwrap_or(0);

    let result = sqlx::query(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id, \
         bound, durability, charges) VALUES ($1, $2, $3, $4, $5, false, 100, 0)",
    )
    .bind(player_id)
    .bind(item_id)
    .bind(count)
    .bind(next_slot)
    .bind(container_id)
    .execute(pool.as_ref())
    .await;

    match result {
        Ok(_) => tracing::debug!(
            player_id,
            item_id,
            container_id,
            slot = next_slot,
            "Item persisted to inventory"
        ),
        Err(e) => {
            tracing::error!(player_id, item_id, "Failed to persist item: {e}");
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
        total_items,
        "Sent full onUpdateItem to client"
    );

    if let Some(tx) = cell_tx {
        let _ = tx
            .send(BaseToCellMsg::InventoryItemGranted { entity_id, item_id })
            .await;
    }

    let is_equipped = (3..=14).contains(&container_id);
    if !is_equipped {
        return;
    }

    if container_id == 3 {
        let mut args = Vec::with_capacity(8);
        args.extend_from_slice(&container_id.to_le_bytes());
        args.extend_from_slice(&(next_slot + 1).to_le_bytes());
        helpers::send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_ACTIVE_SLOT_UPDATE,
                    &args,
                )
            },
        )
        .await;

        if let Some(tx) = cell_tx {
            #[derive(sqlx::FromRow)]
            struct BandolierRow {
                item_id: i32,
                clip_size: i32,
                default_ammo_type_id: i32,
            }

            let item = sqlx::query_as::<_, BandolierRow>(
                r#"
                SELECT item_id, COALESCE(clip_size, 0) AS clip_size,
                       CASE WHEN default_ammo_type IS NULL THEN 0
                            ELSE array_position(enum_range(NULL::resources."EAmmoType"), default_ammo_type) - 1
                       END AS default_ammo_type_id
                FROM resources.items
                WHERE item_id = $1
                "#,
            )
            .bind(item_id)
            .fetch_optional(pool.as_ref())
            .await
            .ok()
            .flatten()
            .map(|row| cimmeria_entity::cell_entity::BandolierItem {
                item_id: row.item_id,
                clip_size: row.clip_size,
                default_ammo_type: row.default_ammo_type_id,
            });

            if let Some(item) = item {
                let _ = tx
                    .send(BaseToCellMsg::UpdateBandolierItem {
                        entity_id,
                        slot_id: next_slot,
                        item,
                        make_active: true,
                    })
                    .await;

                let result =
                    sqlx::query("UPDATE sgw_player SET bandolier_slot = $1 WHERE player_id = $2")
                        .bind(next_slot)
                        .bind(player_id)
                        .execute(pool.as_ref())
                        .await;
                match result {
                    Ok(_) => tracing::debug!(
                        player_id,
                        slot_id = next_slot,
                        "Persisted granted bandolier slot"
                    ),
                    Err(e) => tracing::error!(
                        player_id,
                        slot_id = next_slot,
                        "Failed to persist granted bandolier slot: {e}"
                    ),
                }
            }
        }
    }

    let visual: Option<String> = sqlx::query_scalar(
        "SELECT visual_component FROM resources.items WHERE item_id = $1 AND visual_component IS NOT NULL",
    )
    .bind(item_id)
    .fetch_optional(pool.as_ref())
    .await
    .unwrap_or(None);

    if visual.is_some() {
        tracing::info!(
            entity_id, player_id, item_id, container_id,
            "Equipped item has visual — resending BeingAppearance"
        );

        let account_id = {
            let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
                Some(a) => a,
                None => return,
            };
            let clients = connected.lock().unwrap();
            match clients.get(&addr) {
                Some(c) => c.account_id,
                None => return,
            }
        };

        let player_data = query_player_load_data(db_pool, account_id, player_id).await;
        let appearance_args = world_entry_appearance::build_appearance_args(
            &player_data.bodyset,
            &player_data.components,
        );

        {
            let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
                Some(a) => a,
                None => return,
            };
            let mut clients = connected.lock().unwrap();
            if let Some(c) = clients.get_mut(&addr) {
                c.cached_appearance_args = Some(appearance_args.clone());
            }
        }

        helpers::send_to_witness(
            socket,
            connected,
            entity_to_addr,
            entity_id,
            |key, seq, acks| {
                build_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::BEING_APPEARANCE,
                    &appearance_args,
                )
            },
        )
        .await;
    }
}