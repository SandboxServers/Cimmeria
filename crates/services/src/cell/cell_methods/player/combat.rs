use tokio::sync::mpsc;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_entity::stats::{HEALTH, FOCUS};
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
        CALL_FOR_AID => {
            if args.len() >= 4 {
                let respawner_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                tracing::info!(entity_id, respawner_id, "callForAid");
                handle_respawn(entity_id, respawner_id, tx, space_mgr).await;
            }
            true
        }

        USE_ABILITY => {
            if args.len() >= 8 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let target_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::debug!(entity_id, ability_id, target_id, "useAbility");
                crate::cell::abilities::handle_use_ability(entity_id, ability_id, target_id, tx, space_mgr).await;

                if target_id > 0 {
                    let target_eid = target_id as u32;
                    let death_info = space_mgr.get_entity(target_eid).and_then(|target| {
                        let is_dead = target.stats.get(HEALTH)
                            .map_or(false, |s| s.cur <= 0);
                        if is_dead && !target.is_player {
                            Some((target.tag.clone(), target.is_player))
                        } else {
                            None
                        }
                    });

                    if let Some((tag, _is_player)) = death_info {
                        if let Some(target) = space_mgr.get_entity_mut(target_eid) {
                            target.ai_state = cimmeria_entity::cell_entity::AiState::Dead;
                            target.threat_list.clear();
                        }

                        if let Some(tag) = tag {
                            let player_id = space_mgr.get_entity(entity_id)
                                .and_then(|e| e.player_id).unwrap_or(0);
                            crate::cell::content::fire_entity_death(
                                entity_id, player_id, &tag, engine, tx, space_mgr,
                            ).await;
                        }
                    }
                }
            }
            true
        }

        USE_ABILITY_ON_GROUND => {
            if args.len() >= 16 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let _x = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let _y = f32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let _z = f32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::debug!(entity_id, ability_id, "useAbilityOnGroundTarget");
            }
            true
        }

        RESPAWN => {
            tracing::debug!(entity_id, "respawn (auto)");
            handle_respawn(entity_id, -1, tx, space_mgr).await;
            true
        }

        UNSTUCK => {
            tracing::info!(entity_id, "UNIMPLEMENTED: unstuck");
            true
        }

        RESET_MY_ABILITIES => {
            tracing::info!(entity_id, "UNIMPLEMENTED: resetMyAbilities");
            true
        }

        _ => false,
    }
}

async fn handle_respawn(
    entity_id: u32,
    respawner_id: i32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(entity_id, "respawn: entity not found");
            return;
        }
    };

    if let Some(health) = entity.stats.get_mut(HEALTH) {
        health.set_current(health.max);
    }
    if let Some(focus) = entity.stats.get_mut(FOCUS) {
        focus.set_current(focus.max);
    }

    let stat_update = entity.stats.serialize_dirty();
    entity.stats.clear_dirty();

    entity.state_field = 0;
    entity.abilities.clear_all_cooldowns();

    tracing::info!(entity_id, "Player respawned, state_field=0");

    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: crate::mercury::method_idx::ON_END_AID_WAIT,
        args: Vec::new(),
    }).await;

    let spawn_pos: [f32; 3] = resolve_respawn_position(respawner_id, entity_id, space_mgr);
    let world_name = space_mgr.get_entity_world_name(entity_id)
        .unwrap_or_else(|| "Castle_CellBlock".to_string());
    space_mgr.update_entity_position(entity_id, spawn_pos, [0, 0, 0], [0.0; 3]);

    let _ = tx.send(CellToBaseMsg::RespawnReload {
        entity_id,
        world_name,
        spawn_pos,
    }).await;
    tracing::info!(entity_id, ?spawn_pos, "Sent RespawnReload to BaseApp");
}

fn resolve_respawn_position(
    respawner_id: i32,
    entity_id: u32,
    space_mgr: &SpaceManager,
) -> [f32; 3] {
    const DEFAULT_POS: [f32; 3] = [-334.231, 73.472, -228.026];

    if respawner_id > 0 {
        if let Some(resp) = space_mgr.respawners.iter().find(|r| r.respawner_id == respawner_id) {
            return resp.pos;
        }
        tracing::warn!(entity_id, respawner_id, "Respawner not found, falling back to world default");
    }

    if let Some(world_name) = space_mgr.get_entity_world_name(entity_id) {
        if let Some(resp) = space_mgr.respawners.iter().find(|r| r.world_name == world_name) {
            return resp.pos;
        }
    }

    tracing::debug!(entity_id, "No respawners for player's world, using default position");
    DEFAULT_POS
}
