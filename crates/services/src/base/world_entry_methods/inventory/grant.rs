use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::base::{ConnectedClientState, helpers, world_entry_appearance};
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_entity_method_packet, method_idx};
use super::core::send_full_inventory_update;
use super::super::player_load::core::query_player_load_data;
use super::super::vendor::serializers::reserve_free_inventory_slots;

/// Normalize item ID array: remove dupes, sort, filter invalid IDs.
pub fn normalize_item_ids(mut item_ids: Vec<i32>) -> Vec<i32> {
    item_ids.retain(|id| *id > 0);
    item_ids.sort_unstable();
    item_ids.dedup();
    item_ids
}

/// Check if an item type can be placed in a container.
///
/// Returns `false` on DB error rather than silently defaulting to "main bag" —
/// the caller can decide whether to abort the operation or try a fallback.
///
/// Default rule (no `container_sets` configured for the item type): only the
/// main inventory bag (container 1) is allowed.
pub async fn item_allows_container(pool: &Arc<PgPool>, type_id: i32, container_id: i32) -> bool {
    let result = sqlx::query_scalar::<_, Option<Vec<i32>>>(
        "SELECT container_sets FROM resources.items WHERE item_id = $1",
    )
    .bind(type_id)
    .fetch_optional(pool.as_ref())
    .await;

    let container_sets: Option<Vec<i32>> = match result {
        Ok(row) => row.flatten(),
        Err(e) => {
            tracing::error!(type_id, container_id, "item_allows_container query failed: {e}");
            return false;
        }
    };

    match container_sets {
        Some(sets) if !sets.is_empty() => sets.contains(&container_id),
        // Either the item type has no row, or `container_sets` is NULL/empty —
        // fall back to allowing only the main bag.
        _ => container_id == 1,
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

    let mut db_tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(player_id, item_id, "GrantItem: begin tx failed: {e}");
            return;
        }
    };

    // Reserve a free slot via the same hole-filling helper used by vendor purchase.
    // (reserve_free_inventory_slots takes a per-(player, container) advisory lock.)
    let next_slot: i32 = match reserve_free_inventory_slots(&mut db_tx, player_id, container_id, 1).await {
        Ok(Some(slots)) => match slots.into_iter().next() {
            Some(s) => s,
            None => {
                let _ = db_tx.rollback().await;
                tracing::warn!(player_id, item_id, container_id, "GrantItem: reserve returned empty");
                return;
            }
        },
        Ok(None) => {
            let _ = db_tx.rollback().await;
            tracing::warn!(player_id, item_id, container_id, "GrantItem: container full");
            return;
        }
        Err(e) => {
            let _ = db_tx.rollback().await;
            tracing::error!(player_id, item_id, container_id, "GrantItem: slot reserve failed: {e}");
            return;
        }
    };

    // Default charges to the item's full charge capacity (consumables/abilities ammo)
    // rather than always inserting `charges = 0`. A DB error here aborts the grant
    // — we don't want a transient timeout to silently produce a depleted item.
    let default_charges: i32 = match sqlx::query_scalar::<_, Option<i32>>(
        "SELECT charges FROM resources.items WHERE item_id = $1",
    )
    .bind(item_id)
    .fetch_optional(&mut *db_tx)
    .await
    {
        Ok(Some(Some(c))) => c,
        Ok(Some(None)) | Ok(None) => 0,
        Err(e) => {
            let _ = db_tx.rollback().await;
            tracing::error!(player_id, item_id, "GrantItem: charges lookup failed: {e}");
            return;
        }
    };

    let result = sqlx::query(
        "INSERT INTO sgw_inventory (character_id, type_id, stack_size, slot_id, container_id, \
         bound, durability, charges) VALUES ($1, $2, $3, $4, $5, false, 100, $6)",
    )
    .bind(player_id)
    .bind(item_id)
    .bind(count)
    .bind(next_slot)
    .bind(container_id)
    .bind(default_charges)
    .execute(&mut *db_tx)
    .await;

    match result {
        Ok(_) => tracing::debug!(
            player_id,
            item_id,
            container_id,
            slot = next_slot,
            charges = default_charges,
            "Item persisted to inventory"
        ),
        Err(e) => {
            let _ = db_tx.rollback().await;
            tracing::error!(player_id, item_id, "Failed to persist item: {e}");
            return;
        }
    }

    // For bandolier grants, persist `bandolier_slot` in the SAME transaction so
    // the inventory insert and the active-slot move are atomic — a separate
    // post-commit UPDATE could silently leave the player with the new item
    // visible but the active slot still pointing at the old one.
    if container_id == 3 {
        if let Err(e) = sqlx::query("UPDATE sgw_player SET bandolier_slot = $1 WHERE player_id = $2")
            .bind(next_slot)
            .bind(player_id)
            .execute(&mut *db_tx)
            .await
        {
            let _ = db_tx.rollback().await;
            tracing::error!(
                player_id,
                slot_id = next_slot,
                "GrantItem: bandolier_slot UPDATE failed inside tx, aborting grant: {e}"
            );
            return;
        }
    }

    if let Err(e) = db_tx.commit().await {
        tracing::error!(player_id, item_id, "GrantItem: commit failed: {e}");
        return;
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
        if let Err(e) = tx
            .send(BaseToCellMsg::InventoryItemGranted {
                entity_id,
                item_id,
                container_id,
                slot_id: next_slot,
                quantity: count,
            })
            .await
        {
            tracing::warn!(
                entity_id,
                player_id,
                item_id,
                "GrantItem: cell channel closed while emitting InventoryItemGranted: {e}"
            );
        }
    }

    let is_equipped = (3..=14).contains(&container_id);
    if !is_equipped {
        return;
    }

    if container_id == 3 {
        // bandolier_slot was already committed inside the inventory tx above —
        // it is safe to broadcast the witness packet now.
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

            // On lookup error or missing row, send the UpdateBandolierItem with
            // degraded clip/ammo values rather than silently dropping the cell
            // sync — the item is committed to DB and the slot/active flag must
            // reach the cell. Combat code will refresh clip_size on next reload.
            let item = match sqlx::query_as::<_, BandolierRow>(
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
            {
                Ok(Some(row)) => cimmeria_entity::cell_entity::BandolierItem {
                    item_id: row.item_id,
                    clip_size: row.clip_size,
                    default_ammo_type: row.default_ammo_type_id,
                },
                Ok(None) => {
                    tracing::warn!(
                        item_id,
                        "GrantItem: no resources.items row for bandolier item, sending UpdateBandolierItem with placeholder clip/ammo"
                    );
                    cimmeria_entity::cell_entity::BandolierItem {
                        item_id,
                        clip_size: 0,
                        default_ammo_type: 0,
                    }
                }
                Err(e) => {
                    tracing::error!(
                        item_id,
                        "GrantItem: bandolier item lookup failed ({e}); sending UpdateBandolierItem with placeholder clip/ammo so cell stays in sync"
                    );
                    cimmeria_entity::cell_entity::BandolierItem {
                        item_id,
                        clip_size: 0,
                        default_ammo_type: 0,
                    }
                }
            };

            if let Err(e) = tx
                .send(BaseToCellMsg::UpdateBandolierItem {
                    entity_id,
                    slot_id: next_slot,
                    item,
                    make_active: true,
                })
                .await
            {
                tracing::warn!(
                    entity_id, player_id, item_id,
                    "GrantItem: cell channel closed sending UpdateBandolierItem: {e}"
                );
            }
        }
    }

    let visual: Option<String> = match sqlx::query_scalar(
        "SELECT visual_component FROM resources.items WHERE item_id = $1 AND visual_component IS NOT NULL",
    )
    .bind(item_id)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(player_id, item_id, "GrantItem: visual_component lookup failed (skipping appearance refresh): {e}");
            None
        }
    };

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