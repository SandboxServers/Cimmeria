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
    build_entity_invisible, build_entity_leave, build_entity_method_packet,
};

use super::super::helpers::send_to_witness;
use super::super::ConnectedClientState;
use super::gate_travel::handle_gate_travel;
use super::methods::inventory::update_bandolier_ammo;
use super::methods::{
    handle_buyback_vendor_items, handle_grant_cash, handle_grant_item, handle_grant_xp,
    handle_mail_request, handle_mission_update, handle_move_inventory_item,
    handle_open_vendor_store, handle_purchase_vendor_items, handle_recharge_inventory_items,
    handle_remove_inventory_item, handle_remove_inventory_item_by_type,
    handle_repair_inventory_item, handle_repair_inventory_items, handle_sell_vendor_items,
    handle_use_inventory_item, send_full_inventory_update,
};
use super::reanchor_player::handle_reanchor_player;
use super::space_registry::register_space;
use super::teleport::handle_teleport_player;

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
        CellToBaseMsg::SpaceData {
            space_id,
            world_name,
        } => {
            register_space(world_name, space_id);
        }
        CellToBaseMsg::EntityCreated {
            entity_id,
            space_id,
            position,
        } => {
            tracing::debug!(
                entity_id,
                space_id,
                ?position,
                "CellService: entity created"
            );
        }
        CellToBaseMsg::EnteredAoI {
            witness_id,
            entity_id,
            space_id: _,
            class_id,
            position,
            direction,
            level,
            npc_data,
        } => {
            tracing::debug!(
                witness_id,
                entity_id,
                class_id,
                level,
                "AoI: entity entered witness range"
            );
            // Packet 1: CREATE_ENTITY + UPDATE_AVATAR (BaseApp immediate)
            send_to_witness(
                socket,
                connected,
                entity_to_addr,
                witness_id,
                |key, seq, acks| {
                    build_create_entity_base(
                        key, seq, acks, entity_id, class_id, position, direction,
                    )
                },
            )
            .await;
            // Packet 2: createOnClient() property cascade (CellApp round-trip)
            send_to_witness(
                socket,
                connected,
                entity_to_addr,
                witness_id,
                |key, seq, acks| {
                    build_create_entity_cascade(
                        key,
                        seq,
                        acks,
                        entity_id,
                        class_id,
                        level,
                        npc_data.as_ref(),
                    )
                },
            )
            .await;
        }
        CellToBaseMsg::LeftAoI {
            witness_id,
            entity_id,
        } => {
            tracing::debug!(witness_id, entity_id, "AoI: entity left witness range");
            send_to_witness(
                socket,
                connected,
                entity_to_addr,
                witness_id,
                |key, seq, acks| build_entity_leave(key, seq, acks, entity_id),
            )
            .await;
        }
        CellToBaseMsg::EntityMoved {
            witness_id,
            entity_id,
            space_id: _,
            position,
            direction,
            velocity,
        } => {
            tracing::trace!(witness_id, entity_id, "AoI: entity position update");
            send_to_witness(
                socket,
                connected,
                entity_to_addr,
                witness_id,
                |key, seq, acks| {
                    build_avatar_update(key, seq, acks, entity_id, position, velocity, direction)
                },
            )
            .await;
        }
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            tracing::debug!(
                entity_id,
                method_index,
                args_len = args.len(),
                "CellService->client entity method call"
            );
            send_to_witness(
                socket,
                connected,
                entity_to_addr,
                entity_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id, method_index, &args)
                },
            )
            .await;
        }
        CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name,
            position,
            rotation,
        } => {
            if let Err(e) = handle_gate_travel(
                entity_id,
                &target_world_name,
                position,
                rotation,
                socket,
                connected,
                entity_to_addr,
                cell_tx,
                db_pool,
            )
            .await
            {
                tracing::error!(entity_id, world = %target_world_name, "Gate travel failed: {e}");
            }
        }
        CellToBaseMsg::ReanchorPlayer {
            entity_id,
            space_id,
            position,
            rotation,
        } => {
            if let Err(e) = handle_reanchor_player(
                entity_id,
                space_id,
                position,
                rotation,
                socket,
                connected,
                entity_to_addr,
            )
            .await
            {
                tracing::error!(entity_id, "Reanchor player failed: {e}");
            }
        }
        CellToBaseMsg::MailRequest {
            entity_id,
            player_id,
            op,
        } => {
            handle_mail_request(
                entity_id,
                player_id,
                op,
                socket,
                connected,
                entity_to_addr,
                db_pool,
            )
            .await;
        }
        CellToBaseMsg::MissionUpdate {
            player_id,
            mission_id,
            status,
            current_step_id,
            completed_step_ids,
            completed_objective_ids,
            active_objective_ids,
            failed_objective_ids,
            repeats,
        } => {
            handle_mission_update(
                player_id,
                mission_id,
                status,
                current_step_id,
                &completed_step_ids,
                &completed_objective_ids,
                &active_objective_ids,
                &failed_objective_ids,
                repeats,
                db_pool,
            )
            .await;
        }
        CellToBaseMsg::GrantXP {
            entity_id,
            xp_amount,
        } => {
            handle_grant_xp(
                entity_id,
                xp_amount,
                db_pool,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::GrantItem {
            entity_id,
            player_id,
            item_id,
            container_id,
            count,
        } => {
            handle_grant_item(
                entity_id,
                player_id,
                item_id,
                container_id,
                count,
                db_pool,
                cell_tx,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::GrantCash {
            entity_id,
            player_id,
            amount,
        } => {
            handle_grant_cash(
                entity_id,
                player_id,
                amount,
                db_pool,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id,
            method_index,
            args,
        } => {
            tracing::debug!(
                witness_id,
                entity_id,
                method_index,
                "Broadcast entity method to witness"
            );
            send_to_witness(
                socket,
                connected,
                entity_to_addr,
                witness_id,
                |key, seq, acks| {
                    build_entity_method_packet(key, seq, acks, entity_id, method_index, &args)
                },
            )
            .await;
        }
        CellToBaseMsg::EntityInvisible {
            witness_id,
            entity_id,
        } => {
            tracing::debug!(witness_id, entity_id, "Send ENTITY_INVISIBLE to witness");
            send_to_witness(
                socket,
                connected,
                entity_to_addr,
                witness_id,
                |key, seq, acks| build_entity_invisible(key, seq, acks, entity_id),
            )
            .await;
        }
        CellToBaseMsg::TeleportPlayer {
            entity_id,
            space_id,
            position,
        } => {
            handle_teleport_player(
                entity_id,
                space_id,
                position,
                socket,
                connected,
                entity_to_addr,
                db_pool,
            )
            .await;
        }
        CellToBaseMsg::StartMinigame {
            entity_id,
            player_id,
            game_name,
            difficulty,
            on_victory_chains,
        } => {
            tracing::info!(entity_id, player_id, %game_name, difficulty, "Starting minigame session");
            if let Some(registry) = minigame_registry {
                let seed = rand::random::<u32>();
                let ticket = registry
                    .register(
                        entity_id,
                        player_id,
                        game_name.clone(),
                        difficulty,
                        1, // tech_competency — TODO: read from player entity
                        seed,
                        0,
                        0,
                        1, // abilities, intelligence, player_level
                        on_victory_chains,
                    )
                    .await;

                if let Some(ticket) = ticket {
                    // Build URL: http://unused/{ip}/{port}/{gameName}/{entityId}/{ticket}
                    let url = format!(
                        "http://unused/{}/{}/{}/{}/{}",
                        minigame_external_host,
                        minigame_external_port,
                        game_name,
                        entity_id,
                        ticket
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
                        socket,
                        connected,
                        entity_to_addr,
                        entity_id,
                        |key, seq, acks| {
                            build_entity_method_packet(key, seq, acks, entity_id, method, &args)
                        },
                    )
                    .await;
                } else {
                    tracing::warn!(
                        entity_id,
                        "Failed to register minigame session (duplicate?)"
                    );
                }
            }
        }
        CellToBaseMsg::MinigameResult {
            entity_id,
            result_code,
            on_victory_chains,
        } => {
            tracing::info!(entity_id, result_code, "Minigame result received");
            // Send onEndMinigame to client
            let method = crate::cell::dispatch::CLIENT_MG_ON_END_MINIGAME;
            send_to_witness(
                socket,
                connected,
                entity_to_addr,
                entity_id,
                |key, seq, acks| build_entity_method_packet(key, seq, acks, entity_id, method, &[]),
            )
            .await;
            // Forward to CellApp for victory chain processing
            if let Some(cell_tx) = cell_tx {
                let _ = cell_tx
                    .send(BaseToCellMsg::MinigameResult {
                        entity_id,
                        result_code,
                        on_victory_chains,
                    })
                    .await;
            }
        }
        CellToBaseMsg::OpenVendorStore {
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
        } => {
            handle_open_vendor_store(
                entity_id,
                player_id,
                vendor_entity_id,
                vendor_template_id,
                db_pool,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::PurchaseVendorItems {
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
            items,
        } => {
            handle_purchase_vendor_items(
                entity_id,
                player_id,
                vendor_entity_id,
                vendor_template_id,
                items,
                db_pool,
                cell_tx,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::SellVendorItems {
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
            items,
        } => {
            handle_sell_vendor_items(
                entity_id,
                player_id,
                vendor_entity_id,
                vendor_template_id,
                items,
                db_pool,
                cell_tx,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::BuybackVendorItems {
            entity_id,
            player_id,
            vendor_entity_id,
            vendor_template_id,
            items,
        } => {
            handle_buyback_vendor_items(
                entity_id,
                player_id,
                vendor_entity_id,
                vendor_template_id,
                items,
                db_pool,
                cell_tx,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::ListInventoryItems {
            entity_id,
            player_id,
        } => {
            if let Some(pool) = &db_pool {
                send_full_inventory_update(
                    entity_id,
                    player_id,
                    pool,
                    socket,
                    connected,
                    entity_to_addr,
                )
                .await;
            }
        }
        CellToBaseMsg::MoveInventoryItem {
            entity_id,
            player_id,
            item_id,
            target_container_id,
            target_slot_id,
            quantity,
        } => {
            handle_move_inventory_item(
                entity_id,
                player_id,
                item_id,
                target_container_id,
                target_slot_id,
                quantity,
                db_pool,
                cell_tx,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::RemoveInventoryItem {
            entity_id,
            player_id,
            item_id,
            quantity,
        } => {
            handle_remove_inventory_item(
                entity_id,
                player_id,
                item_id,
                quantity,
                db_pool,
                cell_tx,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::UseInventoryItem {
            entity_id,
            player_id,
            item_id,
            target_id,
        } => {
            handle_use_inventory_item(entity_id, player_id, item_id, target_id, db_pool, cell_tx)
                .await;
        }
        CellToBaseMsg::RemoveInventoryItemByType {
            entity_id,
            player_id,
            type_id,
            count,
        } => {
            handle_remove_inventory_item_by_type(
                entity_id,
                player_id,
                type_id,
                count,
                db_pool,
                cell_tx,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::RepairInventoryItem {
            entity_id,
            player_id,
            item_id,
            repair_ratio,
        } => {
            handle_repair_inventory_item(
                entity_id,
                player_id,
                item_id,
                repair_ratio,
                db_pool,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::RepairInventoryItems {
            entity_id,
            player_id,
            item_ids,
            vendor_template_id,
        } => {
            // Route through the wrapper so the `None` (free repair) path is
            // reachable. The wrapper dispatches to handle_paid_repair when a
            // template id is supplied and to the free-repair UPDATE otherwise.
            handle_repair_inventory_items(
                entity_id,
                player_id,
                item_ids,
                vendor_template_id,
                db_pool,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::RechargeInventoryItems {
            entity_id,
            player_id,
            item_ids,
            vendor_template_id,
        } => {
            // Route through the wrapper so the `None` (free recharge) path is
            // reachable. The wrapper dispatches to handle_paid_recharge when a
            // template id is supplied and to the free-recharge UPDATE otherwise.
            handle_recharge_inventory_items(
                entity_id,
                player_id,
                item_ids,
                vendor_template_id,
                db_pool,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::ActiveSlotUpdate {
            entity_id,
            player_id,
            slot_id,
        } => {
            if let Some(pool) = db_pool {
                // The schema column is `bandolier_slot` (see sgw_player.sql);
                // an earlier draft used `active_bandolier_slot` which never
                // existed and would hard-fail at runtime.
                let updated = match sqlx::query(
                    "UPDATE sgw_player SET bandolier_slot = $1 WHERE player_id = $2",
                )
                .bind(slot_id)
                .bind(player_id)
                .execute(pool.as_ref())
                .await
                {
                    Ok(res) if res.rows_affected() == 0 => {
                        tracing::warn!(player_id, slot_id, "ActiveSlotUpdate: no rows updated");
                        false
                    }
                    Ok(_) => true,
                    Err(e) => {
                        tracing::warn!(player_id, slot_id, error = %e, "ActiveSlotUpdate: DB write failed");
                        false
                    }
                };
                // Refresh the player's appearance after the slot is durable.
                // The appearance query at `player_load/core.rs` filters
                // bandolier visual components by the persisted `bandolier_slot`,
                // so this re-query (and the resulting `BEING_APPEARANCE`
                // broadcast) is what actually swaps the visible weapon on
                // the model. Without it, F1-F4 changes the active slot but
                // the player keeps holding whatever weapon was visible at
                // login.
                //
                // Skip when the UPDATE didn't land — the appearance is still
                // consistent with what the DB says, and a no-op refresh would
                // just spam witnesses with the same packet they already have.
                if updated {
                    super::methods::inventory::refresh_player_appearance(
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
        }
        CellToBaseMsg::BandolierAmmoUpdate {
            player_id,
            slot_id,
            expected_item_id,
            current_ammo,
            cur_ammo_type,
        } => {
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
                    player_id,
                    slot_id,
                    expected_item_id,
                    current_ammo,
                    cur_ammo_type,
                    "BandolierAmmoUpdate: dropping out-of-range payload"
                );
                return;
            }
            if let Some(pool) = db_pool {
                if let Err(e) = update_bandolier_ammo(
                    pool.as_ref(),
                    player_id,
                    slot_id,
                    expected_item_id,
                    current_ammo,
                    cur_ammo_type,
                )
                .await
                {
                    tracing::warn!(
                        player_id, slot_id, expected_item_id, current_ammo, cur_ammo_type, error = %e,
                        "BandolierAmmoUpdate: DB write failed"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_maps() -> (
        Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
        Arc<Mutex<HashMap<u32, SocketAddr>>>,
    ) {
        (
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
        )
    }

    #[tokio::test]
    async fn minigame_result_forwards_to_cell_service() {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let (connected, entity_to_addr) = empty_maps();
        let (cell_tx, mut cell_rx) = mpsc::channel(1);

        handle_cell_message(
            CellToBaseMsg::MinigameResult {
                entity_id: 10,
                result_code: 2,
                on_victory_chains: vec![100, 200],
            },
            &socket,
            &connected,
            &entity_to_addr,
            &Some(cell_tx),
            &None,
            &None,
            "127.0.0.1",
            7777,
        )
        .await;

        match cell_rx.try_recv().expect("minigame result forwarded") {
            BaseToCellMsg::MinigameResult {
                entity_id,
                result_code,
                on_victory_chains,
            } => {
                assert_eq!(entity_id, 10);
                assert_eq!(result_code, 2);
                assert_eq!(on_victory_chains, vec![100, 200]);
            }
            other => panic!("unexpected forwarded message: {other:?}"),
        }
    }

    #[tokio::test]
    async fn invalid_bandolier_ammo_update_drops_before_side_effects() {
        let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let (connected, entity_to_addr) = empty_maps();
        let (cell_tx, mut cell_rx) = mpsc::channel(1);

        handle_cell_message(
            CellToBaseMsg::BandolierAmmoUpdate {
                player_id: 10,
                slot_id: -1,
                expected_item_id: 42,
                current_ammo: 17,
                cur_ammo_type: 1,
            },
            &socket,
            &connected,
            &entity_to_addr,
            &Some(cell_tx),
            &None,
            &None,
            "127.0.0.1",
            7777,
        )
        .await;

        assert!(
            cell_rx.try_recv().is_err(),
            "invalid payload must not forward"
        );
        assert!(connected.lock().unwrap().is_empty());
        assert!(entity_to_addr.lock().unwrap().is_empty());
    }
}
