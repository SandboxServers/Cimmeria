use tokio::sync::mpsc;
use cimmeria_content_engine::chain::ChainEngine;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::constants::*;

pub async fn dispatch(
    entity_id: u32,
    method_index: u16,
    args: &[u8],
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) -> bool {
    match method_index {
        WHO => {
            tracing::info!(entity_id, "UNIMPLEMENTED: who");
            true
        }

        INTERACT => {
            if args.len() >= 4 {
                let target_entity_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, target_entity_id, "interact");

                let is_hostile = space_mgr.get_entity(target_entity_id as u32)
                    .map_or(false, |t| !t.is_player && t.faction == 10);
                if is_hostile {
                    tracing::info!(entity_id, target_entity_id, "interact: targeting hostile NPC for combat");
                    let mut reply = Vec::with_capacity(4);
                    reply.extend_from_slice(&target_entity_id.to_le_bytes());
                    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                        entity_id,
                        method_index: 16,
                        args: reply,
                    }).await;
                    crate::cell::abilities::handle_use_ability(
                        entity_id, 592, target_entity_id, tx, space_mgr,
                    ).await;
                    return true;
                }

                let mut handled = false;
                if let Some(target) = space_mgr.get_entity(target_entity_id as u32) {
                    let tag = target.tag.clone();
                    let template_name = target.npc_name.clone();
                    let player_id = space_mgr.get_entity(entity_id)
                        .and_then(|e| e.player_id).unwrap_or(0);

                    if let Some(ref tag) = tag {
                        handled = crate::cell::content::fire_interact_tag(
                            entity_id, player_id, tag, target_entity_id as u32,
                            engine, tx, space_mgr,
                        ).await;
                    }

                    if !handled {
                        if let Some(ref name) = template_name {
                            handled = crate::cell::content::fire_interact_template(
                                entity_id, player_id, name, target_entity_id as u32,
                                engine, tx, space_mgr,
                            ).await;
                        }
                    }
                }

                if !handled {
                    let dialog_id = crate::cell::interactions::handle_interact(
                        entity_id, target_entity_id as u32, tx, space_mgr,
                    ).await;

                    if let Some(did) = dialog_id {
                        let player_id = space_mgr.get_entity(entity_id)
                            .and_then(|e| e.player_id)
                            .unwrap_or(0);
                        crate::cell::content::fire_dialog_open(
                            entity_id, player_id, did, engine, tx, space_mgr,
                        ).await;
                    } else {
                        let is_hostile_npc = space_mgr.get_entity(target_entity_id as u32)
                            .map_or(false, |t| !t.is_player);
                        if is_hostile_npc {
                            tracing::debug!(entity_id, target_entity_id, "interact: targeting hostile NPC for combat");
                            let mut reply = Vec::with_capacity(4);
                            reply.extend_from_slice(&target_entity_id.to_le_bytes());
                            let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                                entity_id,
                                method_index: 16,
                                args: reply,
                            }).await;
                        }
                    }
                }
            }
            true
        }

        DIALOG_BUTTON_CHOICE => {
            if args.len() >= 8 {
                let dialog_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let button_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, dialog_id, button_id, "dialogButtonChoice");

                let player_id = space_mgr.get_entity(entity_id)
                    .and_then(|e| e.player_id).unwrap_or(0);
                crate::cell::content::fire_dialog_choice(
                    entity_id, player_id, dialog_id, engine, tx, space_mgr,
                ).await;
            }
            true
        }

        INITIAL_RESPONSE => {
            if args.len() >= 4 {
                let interaction_set_map_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, interaction_set_map_id, "initialResponse");

                crate::cell::interactions::handle_initial_response(
                    entity_id, interaction_set_map_id, engine, tx, space_mgr,
                ).await;
            }
            true
        }

        _ => false,
    }
}
