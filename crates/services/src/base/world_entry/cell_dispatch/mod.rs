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

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
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
    handle_use_inventory_item, send_full_inventory_resync,
};
use super::reanchor_player::handle_reanchor_player;
use super::space_registry::register_space;
use super::teleport::handle_teleport_player;

mod aoi;
mod bandolier;
mod minigame;
mod state_field;
mod system_options;

pub(crate) use aoi::flush_deferred_aoi;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_dispatch_arms;

/// Handle a message from CellService -- dispatches AoI packets to witness clients.
pub(crate) async fn handle_cell_message(
    msg: CellToBaseMsg,
    transport: &Arc<dyn Transport>,
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
            // Gate: while the witness is pre-`onClientReady`, the client
            // hasn't loaded terrain yet — buffer the CREATE_ENTITY +
            // cascade so it fires AFTER the client is ready to ACK.
            // See `crate::base::deferred_aoi`.
            let witness_addr = entity_to_addr
                .lock()
                .ok()
                .and_then(|m| m.get(&witness_id).copied());
            if let Some(addr) = witness_addr {
                if super::super::deferred_aoi::should_defer(connected, addr) {
                    super::super::deferred_aoi::push_deferred(
                        connected,
                        addr,
                        super::super::deferred_aoi::DeferredAoiMsg::EnteredAoI {
                            entity_id,
                            class_id,
                            position,
                            direction,
                            level,
                            npc_data,
                        },
                    );
                    return;
                }
            } else {
                // No addr mapping for this witness yet. `entered_aoi` below will
                // call `send_to_witness_reliable`, which silently no-ops with no
                // resolvable address — the CREATE_ENTITY + cascade are lost. The
                // cell has ALREADY marked this entity in the witness set, so the
                // idempotent AoI tick will NOT re-introduce it: the entity stays
                // invisible to this witness for the whole session and only heals
                // on relog (which resets the witness set). This is the suspected
                // mechanism behind "static NPC missing until relog" — surface it
                // loudly. See docs/architecture/negative-logging-convention.md.
                tracing::warn!(
                    target: "aoi.entered_no_witness_addr",
                    witness_id,
                    entity_id,
                    class_id,
                    reason = "witness_addr_unmapped",
                    "EnteredAoI for unmapped witness — CREATE_ENTITY + cascade will be dropped; entity invisible to witness until relog"
                );
            }
            aoi::entered_aoi(
                witness_id,
                entity_id,
                class_id,
                position,
                direction,
                level,
                npc_data,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::LeftAoI {
            witness_id,
            entity_id,
        } => {
            // Gate: while the witness is pre-`onClientReady`, buffer
            // the LEFT event so it pairs correctly with the buffered
            // ENTERED event on flush. If the entity enters AND leaves
            // before the client is ready, the client never sees either
            // (both buffered, both dispatched at flush).
            let witness_addr = entity_to_addr
                .lock()
                .ok()
                .and_then(|m| m.get(&witness_id).copied());
            if let Some(addr) = witness_addr {
                if super::super::deferred_aoi::should_defer(connected, addr) {
                    super::super::deferred_aoi::push_deferred(
                        connected,
                        addr,
                        super::super::deferred_aoi::DeferredAoiMsg::LeftAoI { entity_id },
                    );
                    return;
                }
            }
            aoi::left_aoi(witness_id, entity_id, transport, connected, entity_to_addr).await;
        }
        CellToBaseMsg::EntityMoved {
            witness_id,
            entity_id,
            space_id: _,
            position,
            direction,
            velocity,
        } => {
            // Position updates are unreliable and superseded by the next
            // frame. While the witness is pre-`onClientReady` the client
            // doesn't have the moving entity yet (its CREATE_ENTITY is
            // buffered), so this update would target an unknown id and
            // be dropped client-side anyway. Drop pre-ready instead of
            // buffering — saves the buffer space and matches what the
            // client will do with it. See `crate::base::deferred_aoi`
            // module docs.
            let witness_addr = entity_to_addr
                .lock()
                .ok()
                .and_then(|m| m.get(&witness_id).copied());
            if let Some(addr) = witness_addr {
                if super::super::deferred_aoi::should_defer(connected, addr) {
                    tracing::trace!(
                        %addr,
                        witness_id,
                        entity_id,
                        "Dropping EntityMoved while witness is pre-onClientReady"
                    );
                    return;
                }
            }
            aoi::entity_moved(
                witness_id,
                entity_id,
                position,
                direction,
                velocity,
                transport,
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
            // Logged before the deferred-buffer check so pre-
            // onClientReady buffered calls still appear once on the
            // wire log; deferred-replay does NOT re-emit.
            crate::wire_log::log_outbound_entity_method(entity_id, entity_id, method_index, &args);
            // Gate on the TARGET entity's session — entity-method calls
            // are dispatched to the entity_id's owning client (see
            // `aoi::entity_method_call`'s lookup). If that client is
            // still pre-`onClientReady`, buffer the method.
            let target_addr = entity_to_addr
                .lock()
                .ok()
                .and_then(|m| m.get(&entity_id).copied());
            if let Some(addr) = target_addr {
                if super::super::deferred_aoi::should_defer(connected, addr) {
                    super::super::deferred_aoi::push_deferred(
                        connected,
                        addr,
                        super::super::deferred_aoi::DeferredAoiMsg::EntityMethodCall {
                            entity_id,
                            method_index,
                            args,
                        },
                    );
                    return;
                }
            }
            aoi::entity_method_call(
                entity_id,
                method_index,
                args,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::EntityMethodCallBatch { entity_id, calls } => {
            // Same wire-log + deferred-buffer pattern as EntityMethodCall,
            // applied per-method so the SigNoz `wire.out` stream sees every
            // method exactly once even when they ride the same packet.
            for (method_index, args) in &calls {
                crate::wire_log::log_outbound_entity_method(
                    entity_id,
                    entity_id,
                    *method_index,
                    args,
                );
            }
            let target_addr = entity_to_addr
                .lock()
                .ok()
                .and_then(|m| m.get(&entity_id).copied());
            if let Some(addr) = target_addr {
                if super::super::deferred_aoi::should_defer(connected, addr) {
                    // Buffer each call individually so the existing
                    // single-method flush path replays them; we lose the
                    // batching benefit during deferred replay but
                    // preserve the at-most-once delivery guarantee.
                    for (method_index, args) in calls {
                        super::super::deferred_aoi::push_deferred(
                            connected,
                            addr,
                            super::super::deferred_aoi::DeferredAoiMsg::EntityMethodCall {
                                entity_id,
                                method_index,
                                args,
                            },
                        );
                    }
                    return;
                }
            }
            aoi::entity_method_call_batch(entity_id, calls, transport, connected, entity_to_addr)
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
                transport,
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
                transport,
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
                transport,
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
            notify_gm,
        } => {
            handle_grant_xp(
                entity_id,
                xp_amount,
                notify_gm,
                db_pool,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::TrainAbility {
            entity_id,
            player_id,
            ability_id,
        } => {
            super::methods::progression::handle_train_ability(
                entity_id,
                player_id,
                ability_id,
                db_pool,
                connected,
                cell_tx,
                transport,
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
            notify_gm,
        } => {
            handle_grant_item(
                entity_id,
                player_id,
                item_id,
                container_id,
                count,
                notify_gm,
                db_pool,
                cell_tx,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::GrantCash {
            entity_id,
            player_id,
            amount,
            notify_gm,
        } => {
            handle_grant_cash(
                entity_id,
                player_id,
                amount,
                notify_gm,
                db_pool,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::GrantExpertise {
            entity_id,
            player_id,
            discipline_id,
            amount,
        } => {
            crate::base::crafting::handlers::handle_grant_expertise(
                entity_id,
                player_id,
                discipline_id,
                amount,
                db_pool,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::GrantAppliedSciencePoints {
            entity_id,
            player_id,
            amount,
        } => {
            crate::base::crafting::handlers::handle_grant_applied_science(
                entity_id,
                player_id,
                amount,
                db_pool,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::ExecuteAuthoringSql {
            entity_id,
            label,
            sql,
        } => {
            crate::base::console_authoring::handle_execute_authoring_sql(
                entity_id,
                &label,
                &sql,
                db_pool,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::ConsoleSearch {
            entity_id,
            kind,
            query,
        } => {
            crate::base::console_authoring::handle_console_search(
                entity_id,
                kind,
                &query,
                db_pool,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::GmSpawnNpc {
            entity_id,
            template_id,
            space_id,
            world_name,
            position,
        } => {
            crate::base::gm_spawn::handle_gm_spawn_npc(
                entity_id,
                template_id,
                space_id,
                world_name,
                position,
                db_pool,
                cell_tx,
                transport,
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
            // observer (witness_id) and observee (entity_id) differ
            // on the AoI fanout — both are recorded so SigNoz can
            // answer "what did entity X broadcast to its witnesses?"
            crate::wire_log::log_outbound_entity_method(witness_id, entity_id, method_index, &args);
            aoi::witness_entity_method(
                witness_id,
                entity_id,
                method_index,
                args,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::EntityInvisible {
            witness_id,
            entity_id,
        } => {
            aoi::entity_invisible(witness_id, entity_id, transport, connected, entity_to_addr)
                .await;
        }
        CellToBaseMsg::TeleportPlayer {
            entity_id,
            space_id,
            position,
            prev_pos,
        } => {
            handle_teleport_player(
                entity_id,
                space_id,
                position,
                prev_pos,
                transport,
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
                transport,
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
                transport,
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
                transport,
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
                transport,
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
                transport,
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
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::ListInventoryItems {
            entity_id,
            player_id,
        } => {
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
                transport,
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
            notify_gm,
        } => {
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
        CellToBaseMsg::UseInventoryItem {
            entity_id,
            player_id,
            item_id,
            target_id,
        } => {
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
                transport,
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
                transport,
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
                transport,
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
                transport,
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
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        CellToBaseMsg::SystemOptionsUpdate {
            player_id,
            auto_reload,
            reload_on_activate,
        } => {
            system_options::persist_system_options(
                player_id,
                auto_reload,
                reload_on_activate,
                db_pool,
            )
            .await;
        }
        CellToBaseMsg::StateFieldUpdate {
            player_id,
            state_field,
        } => {
            state_field::persist_state_field(player_id, state_field, db_pool).await;
        }
        CellToBaseMsg::RefreshAppearance {
            entity_id,
            player_id,
            holstered,
        } => {
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
        CellToBaseMsg::BandolierAmmoUpdate {
            player_id,
            slot_id,
            expected_instance_id,
            current_ammo,
            cur_ammo_type,
        } => {
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
        CellToBaseMsg::ExecuteTrade {
            entity_id,
            player_id,
            partner_entity_id,
            partner_player_id,
            p1_item_instance_ids,
            p1_cash,
            p2_item_instance_ids,
            p2_cash,
        } => {
            super::methods::trade::handle_execute_trade(
                entity_id,
                player_id,
                partner_entity_id,
                partner_player_id,
                p1_item_instance_ids,
                p1_cash,
                p2_item_instance_ids,
                p2_cash,
                db_pool,
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
    }
}
