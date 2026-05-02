//! `CellToBaseMsg` dispatch -- routes messages from CellService to client packets,
//! delegating gate-travel, vendor, inventory, mail, missions, and minigame work
//! to focused handlers.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::mercury::{
    build_avatar_update, build_create_entity_base, build_create_entity_cascade,
    build_entity_leave, build_entity_method_packet, build_forced_position,
    build_reset_entities, method_idx,
};

use super::super::ConnectedClientState;
use super::super::helpers::send_to_witness;
use super::gate_travel::handle_gate_travel;
use super::methods::inventory::update_bandolier_ammo;
use super::methods::{
    handle_buyback_vendor_items, handle_grant_cash, handle_grant_item, handle_grant_xp,
    handle_mail_request, handle_mission_update, handle_move_inventory_item,
    handle_open_vendor_store, handle_purchase_vendor_items,
    handle_recharge_inventory_items, handle_remove_inventory_item,
    handle_repair_inventory_item, handle_repair_inventory_items, handle_sell_vendor_items,
    handle_use_inventory_item, send_full_inventory_update,
};
use super::space_registry::register_space;

/// Handle a message from CellService -- dispatches AoI packets to witness clients.
pub(crate) async fn handle_cell_message(
    msg: CellToBaseMsg,
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    db_pool: &Option<Arc<PgPool>>,
    minigame_registry: &Option<crate::minigame::SessionRegistry>,
    minigame_external_host: &str,
    minigame_external_port: u16,
) {
    match msg {
        CellToBaseMsg::SpaceData { space_id, world_name } => {
            register_space(world_name, space_id);
        }
        CellToBaseMsg::EntityCreated { entity_id, space_id, position } => {
            tracing::debug!(entity_id, space_id, ?position, "CellService: entity created");
        }
        CellToBaseMsg::EnteredAoI { witness_id, entity_id, space_id: _, class_id, position, direction, level, npc_data } => {
            tracing::debug!(witness_id, entity_id, class_id, level, "AoI: entity entered witness range");
            // Packet 1: CREATE_ENTITY + UPDATE_AVATAR (BaseApp immediate)
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_create_entity_base(
                        key, seq, acks, entity_id,
                        class_id, position, direction,
                    )
                },
            ).await;
            // Packet 2: createOnClient() property cascade (CellApp round-trip)
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_create_entity_cascade(
                        key, seq, acks, entity_id,
                        class_id, level, npc_data.as_ref(),
                    )
                },
            ).await;
        }
        CellToBaseMsg::LeftAoI { witness_id, entity_id } => {
            tracing::debug!(witness_id, entity_id, "AoI: entity left witness range");
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_entity_leave(key, seq, acks, entity_id)
                },
            ).await;
        }
        CellToBaseMsg::EntityMoved { witness_id, entity_id, space_id: _, position, direction, velocity } => {
            tracing::trace!(witness_id, entity_id, "AoI: entity position update");
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_avatar_update(
                        key, seq, acks, entity_id,
                        position, velocity, direction,
                    )
                },
            ).await;
        }
        CellToBaseMsg::EntityMethodCall { entity_id, method_index, args } => {
            tracing::debug!(entity_id, method_index, args_len = args.len(), "CellService->client entity method call");
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id, method_index, &args)
                },
            ).await;
        }
        CellToBaseMsg::GateTravel { entity_id, target_world_name, position, rotation } => {
            if let Err(e) = handle_gate_travel(
                entity_id, &target_world_name, position, rotation,
                socket, connected, entity_to_addr, cell_tx, db_pool,
            ).await {
                tracing::error!(entity_id, world = %target_world_name, "Gate travel failed: {e}");
            }
        }
        CellToBaseMsg::MailRequest { entity_id, player_id, op } => {
            handle_mail_request(entity_id, player_id, op, socket, connected, entity_to_addr, db_pool).await;
        }
        CellToBaseMsg::MissionUpdate { player_id, mission_id, status, current_step_id,
                                        completed_step_ids, completed_objective_ids, active_objective_ids,
                                        failed_objective_ids } => {
            handle_mission_update(
                player_id, mission_id, status, current_step_id,
                &completed_step_ids, &completed_objective_ids, &active_objective_ids,
                &failed_objective_ids, db_pool,
            ).await;
        }
        CellToBaseMsg::GrantXP { entity_id, xp_amount } => {
            handle_grant_xp(entity_id, xp_amount, socket, connected, entity_to_addr).await;
        }
        CellToBaseMsg::GrantItem { entity_id, player_id, item_id, container_id, count } => {
            handle_grant_item(
                entity_id, player_id, item_id, container_id, count,
                db_pool, cell_tx, socket, connected, entity_to_addr,
            ).await;
        }
        CellToBaseMsg::GrantCash { entity_id, player_id, amount } => {
            handle_grant_cash(entity_id, player_id, amount, db_pool, socket, connected, entity_to_addr).await;
        }
        CellToBaseMsg::WitnessEntityMethod { witness_id, entity_id, method_index, args } => {
            tracing::debug!(witness_id, entity_id, method_index, "Broadcast entity method to witness");
            send_to_witness(
                socket, connected, entity_to_addr, witness_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id, method_index, &args)
                },
            ).await;
        }
        CellToBaseMsg::TeleportPlayer { entity_id, space_id, position } => {
            handle_teleport_player(
                entity_id, space_id, position,
                socket, connected, entity_to_addr, db_pool,
            ).await;
        }
        CellToBaseMsg::RespawnReload { entity_id, world_name, spawn_pos } => {
            tracing::info!(entity_id, %world_name, ?spawn_pos, "RespawnReload: triggering map reload");

            // 1a. Send RESET_ENTITIES to destroy the ragdolled pawn.
            // Without this, onClientMapLoad re-uses the existing pawn which
            // retains its ragdoll physics mode. RESET_ENTITIES forces the client
            // to destroy all entities so mapLoaded creates them fresh.
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_reset_entities(key, seq, acks)
                },
            ).await;

            // 1b. Send onClientMapLoad to trigger loading screen
            let mut ml_args = Vec::new();
            crate::mercury::write_wstring(&mut ml_args, &world_name); // areaName
            crate::mercury::write_wstring(&mut ml_args, &world_name); // mapPath
            ml_args.extend_from_slice(&0i32.to_le_bytes());           // WorldID
            ml_args.extend_from_slice(&spawn_pos[0].to_le_bytes());   // Location X
            ml_args.extend_from_slice(&spawn_pos[1].to_le_bytes());   // Location Y
            ml_args.extend_from_slice(&spawn_pos[2].to_le_bytes());   // Location Z
            ml_args.extend_from_slice(&0.0f32.to_le_bytes());         // Direction X
            ml_args.extend_from_slice(&0.0f32.to_le_bytes());         // Direction Y
            ml_args.extend_from_slice(&0.0f32.to_le_bytes());         // Direction Z
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id,
                        method_idx::ON_CLIENT_MAP_LOAD, &ml_args)
                },
            ).await;

            // 2. Set up pending_client_ready so the next onClientReady triggers
            //    a fresh mapLoaded sequence (stats, appearance, state_field=0).
            //    This is what actually clears the ragdoll — the full world re-entry.
            //    We query player_id from the DB via account_id since we don't cache it.
            let addr = {
                entity_to_addr.lock().unwrap().get(&entity_id).copied()
            };
            if let Some(addr) = addr {
                let (account_id, cached_player_id, appearance_args, tint_args) = {
                    let clients = connected.lock().unwrap();
                    if let Some(c) = clients.get(&addr) {
                        (c.account_id,
                         c.active_player_id,
                         c.cached_appearance_args.clone().unwrap_or_default(),
                         c.cached_tint_args.clone().unwrap_or_default())
                    } else {
                        (0, None, vec![], vec![])
                    }
                };
                // Fail closed: a missing active_player_id means we cannot
                // safely identify which character to respawn. The previous
                // "lowest player_id for account" fallback would silently
                // respawn a different character on multi-character accounts —
                // the same bug class hardened in handle_gate_travel.
                let player_id: i32 = match cached_player_id {
                    Some(pid) => pid,
                    None => {
                        tracing::error!(
                            entity_id, account_id,
                            "Respawn: no active_player_id cached — aborting respawn (would risk respawning the wrong character on multi-character accounts)"
                        );
                        return;
                    }
                };

                let mut clients = connected.lock().unwrap();
                if let Some(c) = clients.get_mut(&addr) {
                    c.pending_client_ready = Some(super::super::PendingClientReadyInfo {
                        entity_id,
                        player_id,
                        world_name: world_name.clone(),
                        appearance_args,
                        tint_args,
                    });
                    tracing::debug!(entity_id, player_id, "Set pending_client_ready for respawn");
                }
            }
        }
        CellToBaseMsg::StartMinigame { entity_id, player_id, game_name, difficulty, on_victory_chains } => {
            tracing::info!(entity_id, player_id, %game_name, difficulty, "Starting minigame session");
            if let Some(registry) = minigame_registry {
                let seed = rand::random::<u32>();
                let ticket = registry.register(
                    entity_id, player_id, game_name.clone(), difficulty,
                    1, // tech_competency — TODO: read from player entity
                    seed, 0, 0, 1, // abilities, intelligence, player_level
                    on_victory_chains,
                ).await;

                if let Some(ticket) = ticket {
                    // Build URL: http://unused/{ip}/{port}/{gameName}/{entityId}/{ticket}
                    let url = format!(
                        "http://unused/{}/{}/{}/{}/{}",
                        minigame_external_host, minigame_external_port,
                        game_name, entity_id, ticket
                    );
                    tracing::info!(entity_id, %url, "Sending onStartMinigame to client");

                    // onStartMinigame(URL: WSTRING) — MinigamePlayer client method
                    // Method index for onStartMinigame in the SGWPlayer flat dispatch table
                    let url_utf16: Vec<u16> = url.encode_utf16().collect();
                    let mut args = Vec::with_capacity(4 + url_utf16.len() * 2);
                    args.extend_from_slice(&(url_utf16.len() as u32).to_le_bytes());
                    for ch in &url_utf16 {
                        args.extend_from_slice(&ch.to_le_bytes());
                    }
                    let method = crate::cell::dispatch::CLIENT_MG_ON_START_MINIGAME;
                    send_to_witness(
                        socket, connected, entity_to_addr, entity_id,
                        |key, seq, acks| {
                            build_entity_method_packet(key, seq, acks, entity_id, method, &args)
                        },
                    ).await;
                } else {
                    tracing::warn!(entity_id, "Failed to register minigame session (duplicate?)");
                }
            }
        }
        CellToBaseMsg::MinigameResult { entity_id, result_code, on_victory_chains } => {
            tracing::info!(entity_id, result_code, "Minigame result received");
            // Send onEndMinigame to client
            let method = crate::cell::dispatch::CLIENT_MG_ON_END_MINIGAME;
            send_to_witness(
                socket, connected, entity_to_addr, entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id, method, &[])
                },
            ).await;
            // Forward to CellApp for victory chain processing
            if let Some(cell_tx) = cell_tx {
                let _ = cell_tx.send(BaseToCellMsg::MinigameResult {
                    entity_id,
                    result_code,
                    on_victory_chains,
                }).await;
            }
        }
        CellToBaseMsg::OpenVendorStore { entity_id, player_id, vendor_entity_id, vendor_template_id } => {
            handle_open_vendor_store(
                entity_id, player_id, vendor_entity_id, vendor_template_id,
                &db_pool, socket, connected, entity_to_addr,
            ).await;
        }
        CellToBaseMsg::PurchaseVendorItems { entity_id, player_id, vendor_entity_id, vendor_template_id, items } => {
            handle_purchase_vendor_items(
                entity_id, player_id, vendor_entity_id, vendor_template_id, items,
                &db_pool, cell_tx, socket, connected, entity_to_addr,
            ).await;
        }
        CellToBaseMsg::SellVendorItems { entity_id, player_id, vendor_entity_id, vendor_template_id, items } => {
            handle_sell_vendor_items(
                entity_id, player_id, vendor_entity_id, vendor_template_id, items,
                &db_pool, cell_tx, socket, connected, entity_to_addr,
            ).await;
        }
        CellToBaseMsg::BuybackVendorItems { entity_id, player_id, vendor_entity_id, vendor_template_id, items } => {
            handle_buyback_vendor_items(
                entity_id, player_id, vendor_entity_id, vendor_template_id, items,
                &db_pool, cell_tx, socket, connected, entity_to_addr,
            ).await;
        }
        CellToBaseMsg::ListInventoryItems { entity_id, player_id } => {
            if let Some(pool) = &db_pool {
                send_full_inventory_update(entity_id, player_id, pool, socket, connected, entity_to_addr).await;
            }
        }
        CellToBaseMsg::MoveInventoryItem { entity_id, player_id, item_id, target_container_id, target_slot_id, quantity } => {
            handle_move_inventory_item(
                entity_id, player_id, item_id, target_container_id, target_slot_id, quantity,
                &db_pool, cell_tx, socket, connected, entity_to_addr,
            ).await;
        }
        CellToBaseMsg::RemoveInventoryItem { entity_id, player_id, item_id, quantity } => {
            handle_remove_inventory_item(
                entity_id, player_id, item_id, quantity,
                &db_pool, cell_tx, socket, connected, entity_to_addr,
            ).await;
        }
        CellToBaseMsg::UseInventoryItem { entity_id, player_id, item_id, target_id } => {
            handle_use_inventory_item(
                entity_id, player_id, item_id, target_id,
                &db_pool, cell_tx, socket, connected, entity_to_addr,
            ).await;
        }
        CellToBaseMsg::RepairInventoryItem { entity_id, player_id, item_id, repair_ratio } => {
            handle_repair_inventory_item(
                entity_id, player_id, item_id, repair_ratio,
                &db_pool, socket, connected, entity_to_addr,
            ).await;
        }
        CellToBaseMsg::RepairInventoryItems { entity_id, player_id, item_ids, vendor_template_id } => {
            // Route through the wrapper so the `None` (free repair) path is
            // reachable. The wrapper dispatches to handle_paid_repair when a
            // template id is supplied and to the free-repair UPDATE otherwise.
            handle_repair_inventory_items(
                entity_id, player_id, item_ids, vendor_template_id,
                &db_pool, socket, connected, entity_to_addr,
            ).await;
        }
        CellToBaseMsg::RechargeInventoryItems { entity_id, player_id, item_ids, vendor_template_id } => {
            // Route through the wrapper so the `None` (free recharge) path is
            // reachable. The wrapper dispatches to handle_paid_recharge when a
            // template id is supplied and to the free-recharge UPDATE otherwise.
            handle_recharge_inventory_items(
                entity_id, player_id, item_ids, vendor_template_id,
                &db_pool, socket, connected, entity_to_addr,
            ).await;
        }
        CellToBaseMsg::ActiveSlotUpdate { player_id, slot_id } => {
            if let Some(pool) = db_pool {
                // The schema column is `bandolier_slot` (see sgw_player.sql);
                // an earlier draft used `active_bandolier_slot` which never
                // existed and would hard-fail at runtime.
                match sqlx::query(
                    "UPDATE sgw_player SET bandolier_slot = $1 WHERE player_id = $2"
                )
                    .bind(slot_id)
                    .bind(player_id)
                    .execute(pool.as_ref())
                    .await
                {
                    Ok(res) if res.rows_affected() == 0 => {
                        tracing::warn!(player_id, slot_id, "ActiveSlotUpdate: no rows updated");
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(player_id, slot_id, error = %e, "ActiveSlotUpdate: DB write failed");
                    }
                }
            }
        }
        CellToBaseMsg::BandolierAmmoUpdate { player_id, slot_id, expected_item_id, current_ammo, cur_ammo_type } => {
            // `player_id` from the cell is the DB character_id (matches the
            // `ActiveSlotUpdate` convention right above).
            //
            // Validate bounds before persisting — these payloads cross a
            // service boundary, and any out-of-range value would become
            // durable corruption. Bandolier holds 5 slots (0-4); ammo and
            // ammo_type IDs are non-negative.
            if !(0..5).contains(&slot_id)
                || current_ammo < 0
                || cur_ammo_type < 0
                || expected_item_id <= 0
            {
                tracing::warn!(
                    player_id, slot_id, expected_item_id, current_ammo, cur_ammo_type,
                    "BandolierAmmoUpdate: dropping out-of-range payload"
                );
                return;
            }
            if let Some(pool) = db_pool {
                if let Err(e) = update_bandolier_ammo(
                    pool.as_ref(), player_id, slot_id, expected_item_id, current_ammo, cur_ammo_type,
                ).await {
                    tracing::warn!(
                        player_id, slot_id, expected_item_id, current_ammo, cur_ammo_type, error = %e,
                        "BandolierAmmoUpdate: DB write failed"
                    );
                }
            }
        }
    }
}

/// Authoritative same-world teleport: snap the player's avatar to `position`.
///
/// Sends three things, in order:
/// 1. `FORCED_POSITION` (0x31) — the engine-level snap. Without this the
///    avatar does not move (the client keeps sending `AVATAR_UPDATE_EXPLICIT`
///    from the source pad). See `build_forced_position` for wire details.
/// 2. `onPlayerTeleport` (method 116) — flags the client into streaming-load
///    waiting state with the new position so terrain chunks load cleanly.
///    See SGWPlayer.def's comment on this method.
/// 3. Persist new pos to `sgw_player` so a relog mid-ceremony doesn't
///    teleport the player back to the source pad. We fail closed on missing
///    `active_player_id` for the same reason as `gate_travel.rs`.
async fn handle_teleport_player(
    entity_id: u32,
    space_id: u32,
    position: [f32; 3],
    socket: &Arc<UdpSocket>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    db_pool: &Option<Arc<PgPool>>,
) {
    // The cell owns authoritative `space_id` and passes it through. We only
    // need the connection state for account_id/active_player_id (DB persist).
    let (account_id, active_player_id) = {
        let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
            Some(a) => a,
            None => {
                tracing::warn!(entity_id, "TeleportPlayer: no client addr for entity");
                return;
            }
        };
        let clients = connected.lock().unwrap();
        match clients.get(&addr) {
            Some(c) => (c.account_id, c.active_player_id),
            None => {
                tracing::warn!(entity_id, %addr, "TeleportPlayer: client state not found");
                return;
            }
        }
    };

    tracing::info!(
        entity_id, ?position, space_id,
        "TeleportPlayer: snapping avatar"
    );

    // 1. Engine-level snap.
    send_to_witness(
        socket, connected, entity_to_addr, entity_id,
        |key, seq, acks| {
            build_forced_position(key, seq, acks, entity_id, space_id, position)
        },
    ).await;

    // 2. Streaming-load waiting flag (method 116). Direction is zeroed —
    //    we don't currently rotate the avatar on ring travel.
    let mut args = Vec::with_capacity(24);
    for &c in &position {
        args.extend_from_slice(&c.to_le_bytes());
    }
    args.extend_from_slice(&[0u8; 12]); // direction = 0,0,0
    const METHOD_ON_PLAYER_TELEPORT: u16 = 116;
    send_to_witness(
        socket, connected, entity_to_addr, entity_id,
        |key, seq, acks| {
            build_entity_method_packet(key, seq, acks, entity_id,
                METHOD_ON_PLAYER_TELEPORT, &args)
        },
    ).await;

    // 3. Persist. Mirrors gate_travel's fail-closed on missing active_player_id.
    if let Some(pool) = db_pool {
        let pid = match active_player_id {
            Some(p) => p,
            None => {
                tracing::error!(
                    entity_id, account_id,
                    "TeleportPlayer: no active_player_id cached — refusing to persist"
                );
                return;
            }
        };
        let res = sqlx::query(
            "UPDATE sgw_player SET pos_x = $1, pos_y = $2, pos_z = $3 \
             WHERE player_id = $4 AND account_id = $5"
        )
        .bind(position[0]).bind(position[1]).bind(position[2])
        .bind(pid).bind(account_id as i32)
        .execute(pool.as_ref()).await;
        match res {
            Ok(r) if r.rows_affected() == 0 => {
                tracing::warn!(entity_id, pid, account_id,
                    "TeleportPlayer: persistence UPDATE matched 0 rows");
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(entity_id, pid, account_id, error = %e,
                    "TeleportPlayer: failed to persist position");
            }
        }
    }
}
