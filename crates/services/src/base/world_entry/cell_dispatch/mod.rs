//! `CellToBaseMsg` dispatch -- routes messages from CellService to client
//! packets, delegating gate-travel, vendor, inventory, mail, missions, and
//! minigame work to focused handlers.
//!
//! The match shell is the "what message → which handler" decision; the
//! per-family handlers live in sibling modules so each family's wire-emitting
//! / DB-touching logic can grow without bloating the match body:
//!
//! - [`aoi`]       — AoI packet emitters (entered/left/moved/method/invisible)
//! - [`minigame`]  — minigame session start + result handling
//! - [`bandolier`] — active-slot + bandolier-ammo persistence

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::net::UdpSocket;
use tokio::sync::mpsc;

use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};

use super::super::ConnectedClientState;
use super::gate_travel::handle_gate_travel;
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

mod aoi;
mod bandolier;
mod minigame;

#[cfg(test)]
mod tests;

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
            aoi::entered_aoi(
                witness_id,
                entity_id,
                class_id,
                position,
                direction,
                level,
                npc_data,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::LeftAoI {
            witness_id,
            entity_id,
        } => {
            aoi::left_aoi(witness_id, entity_id, socket, connected, entity_to_addr).await;
        }
        CellToBaseMsg::EntityMoved {
            witness_id,
            entity_id,
            space_id: _,
            position,
            direction,
            velocity,
        } => {
            aoi::entity_moved(
                witness_id,
                entity_id,
                position,
                direction,
                velocity,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            aoi::entity_method_call(
                entity_id,
                method_index,
                args,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::GateTravel {
            entity_id,
            target_world_name,
            position,
            rotation,
            destination_ring_id,
        } => {
            if let Err(e) = handle_gate_travel(
                entity_id,
                &target_world_name,
                position,
                rotation,
                destination_ring_id,
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
            aoi::witness_entity_method(
                witness_id,
                entity_id,
                method_index,
                args,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::EntityInvisible {
            witness_id,
            entity_id,
        } => {
            aoi::entity_invisible(witness_id, entity_id, socket, connected, entity_to_addr).await;
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
            minigame::start_minigame(
                entity_id,
                player_id,
                game_name,
                difficulty,
                on_victory_chains,
                socket,
                connected,
                entity_to_addr,
                minigame_registry,
                minigame_external_host,
                minigame_external_port,
            )
            .await;
        }
        CellToBaseMsg::MinigameResult {
            entity_id,
            result_code,
            on_victory_chains,
        } => {
            minigame::minigame_result(
                entity_id,
                result_code,
                on_victory_chains,
                socket,
                connected,
                entity_to_addr,
                cell_tx,
            )
            .await;
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
            bandolier::active_slot_update(
                entity_id,
                player_id,
                slot_id,
                db_pool,
                socket,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::BandolierAmmoUpdate {
            player_id,
            slot_id,
            expected_item_id,
            current_ammo,
            cur_ammo_type,
        } => {
            bandolier::bandolier_ammo_update(
                player_id,
                slot_id,
                expected_item_id,
                current_ammo,
                cur_ammo_type,
                db_pool,
            )
            .await;
        }
    }
}
