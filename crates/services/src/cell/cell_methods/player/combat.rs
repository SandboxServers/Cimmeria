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

                // Snapshot whether the target was alive *before* the ability
                // resolves. Without this, hitting an already-dead corpse would
                // re-fire fire_entity_death (and stomp the AI cleanup that
                // handle_use_ability already performed on the original kill),
                // double-counting mission progress on every post-death swing.
                let was_alive_before = if target_id > 0 {
                    space_mgr.get_entity(target_id as u32).map_or(false, |t| {
                        !t.is_player
                            && t.stats.get(HEALTH).map_or(false, |s| s.cur > 0)
                    })
                } else {
                    false
                };

                crate::cell::abilities::handle_use_ability(entity_id, ability_id, target_id, tx, space_mgr).await;

                // Only react to alive→dead transitions caused by *this* call.
                // handle_use_ability already handles AI/loot/XP on the kill
                // itself; we only need to fire the content-engine death event
                // here, since that's a separate concern wired off the killing
                // player's player_id.
                if was_alive_before {
                    let target_eid = target_id as u32;
                    let just_died = space_mgr.get_entity(target_eid).map_or(false, |t| {
                        t.stats.get(HEALTH).map_or(false, |s| s.cur <= 0)
                    });
                    if just_died {
                        let tag = space_mgr.get_entity(target_eid).and_then(|t| t.tag.clone());
                        if let Some(tag) = tag {
                            match space_mgr.get_entity(entity_id).and_then(|e| e.player_id) {
                                Some(player_id) => {
                                    crate::cell::content::fire_entity_death(
                                        entity_id, player_id, &tag, engine, tx, space_mgr,
                                    ).await;
                                }
                                None => {
                                    tracing::warn!(
                                        entity_id, npc_tag = %tag,
                                        "Skipping entity_death event: killer entity has no player_id"
                                    );
                                }
                            }
                        }
                    }
                }
            }
            true
        }

        USE_ABILITY_ON_GROUND => {
            if args.len() >= 16 {
                let ability_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let x = f32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let y = f32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let z = f32::from_le_bytes([args[12], args[13], args[14], args[15]]);
                tracing::debug!(entity_id, ability_id, x, y, z, "useAbilityOnGroundTarget");

                // handle_use_ability_on_ground returns the entity IDs of every
                // NPC that died during this cast (primary + AoE secondaries).
                // We fire the content-engine death event for each, so kill-
                // count missions and other death-triggered chains advance for
                // every AoE kill — not just the primary. Empty Vec means
                // either no targets in radius, primary cast rejected, or
                // nothing died.
                let deaths = crate::cell::abilities::handle_use_ability_on_ground(
                    entity_id, ability_id, [x, y, z], tx, space_mgr,
                ).await;

                if !deaths.is_empty() {
                    // Resolve player_id once — it doesn't change across kills.
                    let player_id = space_mgr.get_entity(entity_id).and_then(|e| e.player_id);
                    for dead_eid in deaths {
                        let tag = space_mgr.get_entity(dead_eid).and_then(|t| t.tag.clone());
                        if let Some(tag) = tag {
                            match player_id {
                                Some(pid) => {
                                    crate::cell::content::fire_entity_death(
                                        entity_id, pid, &tag, engine, tx, space_mgr,
                                    ).await;
                                }
                                None => {
                                    tracing::warn!(
                                        entity_id, npc_tag = %tag, dead_eid,
                                        "Skipping entity_death event (ground target): killer entity has no player_id"
                                    );
                                }
                            }
                        }
                    }
                }
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

    // Push the refreshed health/focus to the client via onEntityStat (method 20)
    // — without this, mapLoaded after RespawnReload would query the stale DB
    // values and the player would render with their pre-death stats.
    if !stat_update.is_empty() {
        crate::cell::abilities::send_entity_method(entity_id, 20, stat_update, tx, space_mgr).await;
    }

    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: crate::mercury::method_idx::ON_END_AID_WAIT,
        args: Vec::new(),
    }).await;

    let spawn_pos: [f32; 3] = resolve_respawn_position(respawner_id, entity_id, space_mgr);
    let world_name = space_mgr.get_entity_world_name(entity_id)
        .unwrap_or_else(|| "Castle_CellBlock".to_string());
    space_mgr.update_entity_position(entity_id, spawn_pos, [0, 0, 0], [0.0; 3]);

    if let Err(e) = tx.send(CellToBaseMsg::RespawnReload {
        entity_id,
        world_name: world_name.clone(),
        spawn_pos,
    }).await {
        tracing::error!(
            entity_id, %world_name, ?spawn_pos, error = %e,
            "RespawnReload send to base failed -- player will not be teleported to spawn"
        );
        return;
    }
    tracing::info!(entity_id, ?spawn_pos, "Sent RespawnReload to BaseApp");
}

fn resolve_respawn_position(
    respawner_id: i32,
    entity_id: u32,
    space_mgr: &SpaceManager,
) -> [f32; 3] {
    // Castle_CellBlock starting hub coordinates — only used as a final fallback
    // for that world; in other worlds we respawn in place rather than teleporting
    // players to Castle.
    const CASTLE_DEFAULT_POS: [f32; 3] = [-334.231, 73.472, -228.026];

    if respawner_id > 0 {
        if let Some(resp) = space_mgr.respawners.iter().find(|r| r.respawner_id == respawner_id) {
            return resp.pos;
        }
        tracing::warn!(entity_id, respawner_id, "Respawner not found, falling back to world default");
    }

    let world_name = space_mgr.get_entity_world_name(entity_id);
    if let Some(ref wn) = world_name {
        if let Some(resp) = space_mgr.respawners.iter().find(|r| r.world_name == *wn) {
            return resp.pos;
        }
    }

    // Castle has a known safe default; for other worlds, respawn in place to
    // avoid silently teleporting the player across worlds.
    //
    // Operational note: in-place respawn outside Castle can produce death
    // loops if the player died standing in damaging geometry (e.g., a lava
    // tile or AoE pool) and no respawner is configured for that world —
    // they'll respawn at full health, immediately take damage from the
    // surrounding geometry, and die again. The clean fix is content-side:
    // every world should ship at least one respawner. This warn log is the
    // signal to operators that a world is missing one. A future combat pass
    // can also add a brief invuln window after respawn to absorb the first
    // damage tick if the player happens to respawn inside an active hazard.
    match world_name.as_deref() {
        Some("Castle_CellBlock") | None => {
            tracing::debug!(entity_id, world = ?world_name, "No respawner; using Castle default position");
            CASTLE_DEFAULT_POS
        }
        Some(world) => {
            let in_place = space_mgr
                .get_entity(entity_id)
                .map(|e| [e.position.x, e.position.y, e.position.z])
                .unwrap_or(CASTLE_DEFAULT_POS);
            tracing::warn!(
                entity_id, world = world,
                "No respawner configured for this world — respawning in place at current position"
            );
            in_place
        }
    }
}
