//! After an equipment grant commits: the bandolier's active-slot broadcast
//! and cell sync, and the appearance refresh for other equipment slots.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::appearance::refresh_player_appearance;
use crate::base::{helpers, ConnectedClientState};
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// Run the post-commit equipment work for a fresh-slot grant into
/// `container_id` (nothing happens outside the equipment containers 3-14).
pub(super) async fn equip_epilogue(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    container_id: i32,
    next_slot: i32,
    instance_id: i32,
    bandolier_became_active: bool,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let Some(pool) = db_pool else {
        return;
    };
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
            helpers::send_to_witness_reliable(
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
                        method_idx::ON_ACTIVE_SLOT_UPDATE,
                        &args,
                        version,
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
                        // The instance PK captured from the grant INSERT's
                        // RETURNING above — this is the ammo-persist TOCTOU
                        // guard, distinct from the design id (`row.item_id`).
                        instance_id,
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
                    super::super::super::vendor::helpers::sync_bandolier_after_inventory_change(
                        entity_id,
                        player_id,
                        db_pool,
                        cell_tx,
                        transport,
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

    // Bandolier items (container_id 3) get their appearance refresh
    // from the cell side: `BaseToCellMsg::UpdateBandolierItem`'s
    // handler dispatches `RefreshAppearance` back to base after
    // flipping `weapon_holstered` correctly. Calling
    // `refresh_player_appearance` here too races the cell side and
    // can broadcast a stale "no weapon" appearance (cached state is
    // still `holstered=true` because the cell hasn't processed the
    // update yet) — that's why initial weapon equips appeared to
    // not show the weapon in playtest.
    //
    // Non-bandolier equipment (helmet, armor, accessories — slot 4
    // and up) doesn't go through the cell side, so we still need
    // this call for those.
    if visual.is_some() && container_id != 3 {
        tracing::info!(
            entity_id,
            player_id,
            item_id,
            container_id,
            "Equipped non-bandolier item has visual — resending BeingAppearance"
        );
        refresh_player_appearance(
            entity_id,
            player_id,
            db_pool,
            transport,
            connected,
            entity_to_addr,
            cell_tx,
        )
        .await;
    }
}
