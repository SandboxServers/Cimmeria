use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use super::super::vendor::serializers::reserve_free_inventory_slots;
use super::appearance::refresh_player_appearance;
use super::core::send_full_inventory_update;
use crate::base::outbox::{self, CellOutboxPayload};
use crate::base::{helpers, ConnectedClientState};
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_entity_method_packet, method_idx};

#[cfg(test)]
mod tests;

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
            tracing::error!(
                type_id,
                container_id,
                "item_allows_container query failed: {e}"
            );
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
    let next_slot: i32 =
        match reserve_free_inventory_slots(&mut db_tx, player_id, container_id, 1).await {
            Ok(Some(slots)) => match slots.into_iter().next() {
                Some(s) => s,
                None => {
                    let _ = db_tx.rollback().await;
                    tracing::warn!(
                        player_id,
                        item_id,
                        container_id,
                        "GrantItem: reserve returned empty"
                    );
                    return;
                }
            },
            Ok(None) => {
                let _ = db_tx.rollback().await;
                tracing::warn!(
                    player_id,
                    item_id,
                    container_id,
                    "GrantItem: container full"
                );
                return;
            }
            Err(e) => {
                let _ = db_tx.rollback().await;
                tracing::error!(
                    player_id,
                    item_id,
                    container_id,
                    "GrantItem: slot reserve failed: {e}"
                );
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
            tracing::error!(
                player_id,
                item_id,
                "GrantItem: grant aborted after rollback"
            );
            return;
        }
    };

    // Pull ammo_type / ammo_types / charges from resources.items so a granted
    // weapon arrives with its real ammo configuration. Without this, the
    // defaults are AMMO_NONE / [] / 0, which makes ranged grants unusable
    // until the player manually changes ammo. INSERT…SELECT keeps this in
    // a single round-trip and gives us COALESCE for ammo_type so items with
    // a NULL default still get a sane sentinel rather than NULL (the column
    // is NOT NULL in the schema).
    let result = sqlx::query(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges, \
             ammo_type, ammo_types, ammo, flags) \
         SELECT $1, ri.item_id, $2, $3, $4, false, 100, $5, \
                COALESCE(ri.default_ammo_type, 'AMMO_NONE'::resources.\"EAmmoType\"), \
                ri.ammo_types, ri.charges, 0 \
         FROM resources.items ri WHERE ri.item_id = $6",
    )
    .bind(player_id)
    .bind(count)
    .bind(next_slot)
    .bind(container_id)
    .bind(default_charges)
    .bind(item_id)
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
    //
    // Only adopt the new slot when the player's *previous* selection no longer
    // points at a real item (e.g., empty bandolier, or the previously-selected
    // slot is now also gone). This matches the slot-preservation behavior of
    // `sync_bandolier_after_inventory_change` so a loot drop or quest reward
    // doesn't hot-swap a player's preferred weapon mid-combat.
    // Track whether the bandolier_slot UPDATE actually adopted the new slot
    // (i.e., rows_affected == 1). If the WHERE-NOT-EXISTS guard preserved the
    // player's existing selection, downstream messages must NOT advertise the
    // new slot as active — otherwise the cell mirrors the new active slot
    // even though the DB still points at the old one (desync).
    let mut bandolier_became_active = false;
    if container_id == 3 {
        let res = sqlx::query(
            "UPDATE sgw_player p \
                SET bandolier_slot = $1 \
              WHERE p.player_id = $2 \
                AND NOT EXISTS ( \
                  SELECT 1 FROM sgw_inventory inv \
                  WHERE inv.character_id = p.player_id \
                    AND inv.container_id = 3 \
                    AND inv.slot_id = p.bandolier_slot \
                )",
        )
        .bind(next_slot)
        .bind(player_id)
        .execute(&mut *db_tx)
        .await;

        match res {
            Ok(r) => {
                bandolier_became_active = r.rows_affected() == 1;
                tracing::debug!(
                    player_id,
                    slot_id = next_slot,
                    swapped = r.rows_affected() == 1,
                    "GrantItem: bandolier_slot reconciled (swapped only if previous selection vacant)"
                );
            }
            Err(e) => {
                let _ = db_tx.rollback().await;
                tracing::error!(
                    player_id,
                    slot_id = next_slot,
                    expected_swap = true,
                    "GrantItem: bandolier_slot UPDATE failed mid-tx -- rollback: {e}"
                );
                return;
            }
        }
    }

    // Enqueue the cell-notification BEFORE commit so the outbox row and the
    // inventory mutation become visible atomically. After commit, try the
    // in-process dispatch; if the cell receiver is gone, the row stays
    // undelivered for the background drainer to retry.
    let outbox_payload = CellOutboxPayload::InventoryItemGranted {
        item_id,
        container_id,
        slot_id: next_slot,
        quantity: count,
    };
    let outbox_id = match outbox::enqueue_in_tx(&mut db_tx, entity_id, &outbox_payload).await {
        Ok(id) => id,
        Err(e) => {
            // Outbox INSERT failed — abort the grant rather than commit
            // an inventory mutation we can't durably notify the cell about.
            // The player retries the grant trigger, which is idempotent at
            // the chain level.
            let _ = db_tx.rollback().await;
            tracing::error!(
                player_id,
                item_id,
                "GrantItem: outbox enqueue failed, aborting: {e}"
            );
            return;
        }
    };

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
        outbox::try_dispatch_now(pool.as_ref(), tx, outbox_id, entity_id, outbox_payload).await;
    }

    let is_equipped = (3..=14).contains(&container_id);
    if !is_equipped {
        return;
    }

    if container_id == 3 {
        // Only broadcast the active-slot witness packet if the DB UPDATE above
        // actually adopted the new slot. If the WHERE-NOT-EXISTS guard kept
        // the player's existing selection, the cell/client must continue to
        // see the previous active slot — broadcasting `next_slot` here would
        // desync the client UI and the cell's `active_bandolier_slot` from
        // the persisted DB value.
        if bandolier_became_active {
            let mut args = Vec::with_capacity(8);
            args.extend_from_slice(&container_id.to_le_bytes());
            args.extend_from_slice(&(next_slot + 1).to_le_bytes());
            let _ = helpers::send_to_witness(
                socket,
                connected,
                entity_to_addr,
                entity_id,
                entity_id,
                "METHOD",
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
        }

        if let Some(tx) = cell_tx {
            #[derive(sqlx::FromRow)]
            struct BandolierRow {
                item_id: i32,
                clip_size: i32,
                default_ammo_type_id: i32,
            }

            // Resolve the item's clip/ammo metadata. On Ok(None) — the granted
            // item type is not in resources.items, which is data corruption —
            // fall back to a full bandolier resync so combat doesn't see
            // clip_size=0. On Err — transient DB failure — also resync rather
            // than ship known-bad clip/ammo to the cell.
            let row = sqlx::query_as::<_, BandolierRow>(
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
            .await;

            match row {
                Ok(Some(row)) => {
                    let item = cimmeria_entity::cell_entity::BandolierItem {
                        item_id: row.item_id,
                        clip_size: row.clip_size,
                        default_ammo_type: row.default_ammo_type_id,
                        // Stage A: a freshly-granted bandolier item starts
                        // with an empty mag and the default ammo subtype.
                        // Stages B/C will pick up these defaults; today the
                        // shadow scalars on CellEntity still drive fire/reload.
                        current_ammo: 0,
                        cur_ammo_type: row.default_ammo_type_id,
                    };
                    if let Err(e) = tx
                        .send(BaseToCellMsg::UpdateBandolierItem {
                            entity_id,
                            slot_id: next_slot,
                            item,
                            // Only flip the cell's active slot when the DB
                            // UPDATE actually adopted next_slot. Otherwise
                            // the cell would mirror an active slot the DB
                            // disagrees with.
                            make_active: bandolier_became_active,
                        })
                        .await
                    {
                        tracing::warn!(
                            entity_id,
                            player_id,
                            item_id,
                            "GrantItem: cell channel closed sending UpdateBandolierItem: {e}"
                        );
                    }
                }
                Ok(None) | Err(_) => {
                    // Either the item type is missing from resources.items
                    // (data corruption — no clip/ammo metadata available) or
                    // the lookup hit a transient error. In both cases we
                    // delegate to the full sync path, which queries fresh
                    // bandolier state under FOR UPDATE and emits
                    // SyncBandolierItems with whatever the DB actually has.
                    if matches!(row, Err(ref _e)) {
                        if let Err(e) = row {
                            tracing::error!(item_id, "GrantItem: bandolier metadata lookup failed ({e}); falling back to sync_bandolier_after_inventory_change");
                        }
                    } else {
                        tracing::warn!(item_id, "GrantItem: no resources.items row for granted bandolier item; falling back to full bandolier resync");
                    }
                    super::super::vendor::helpers::sync_bandolier_after_inventory_change(
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
            entity_id,
            player_id,
            item_id,
            container_id,
            "Equipped item has visual — resending BeingAppearance"
        );
        refresh_player_appearance(
            entity_id,
            player_id,
            db_pool,
            socket,
            connected,
            entity_to_addr,
        )
        .await;
    }
}
