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
        SET_AUTO_CYCLE => {
            if !args.is_empty() {
                let enabled = args[0] != 0;
                tracing::debug!(entity_id, enabled, "setAutoCycle");
                if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
                    entity.abilities.auto_cycle = enabled;
                    if !enabled {
                        entity.abilities.auto_cycle_ability_id = None;
                    }
                }
            }
            true
        }

        LOOT_ITEM => {
            if args.len() >= 4 {
                let index = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                crate::cell::interactions::handle_loot_item(
                    entity_id, index, tx, space_mgr,
                ).await;
            }
            true
        }

        TRIGGER_REGION => {
            if args.len() >= 17 {
                let region_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let b_entering = args[4] != 0;
                let _x = f32::from_le_bytes([args[5], args[6], args[7], args[8]]);
                let _y = f32::from_le_bytes([args[9], args[10], args[11], args[12]]);
                let _z = f32::from_le_bytes([args[13], args[14], args[15], args[16]]);

                // Region IDs are wire-encoded as i32 but stored as u32 internally.
                // Reject negative values up-front rather than sign-extending them
                // into a high u32 that no real region will match.
                let region_tag = match u32::try_from(region_id) {
                    Ok(rid) => space_mgr.get_region(rid).map(|r| r.tag.clone()),
                    Err(_) => {
                        tracing::warn!(entity_id, region_id, "triggerClientHintedGenericRegion: negative region_id, ignoring");
                        None
                    }
                };

                if let Some(tag) = region_tag {
                    tracing::info!(entity_id, region_id, %tag, b_entering, "triggerClientHintedGenericRegion");

                    let player_id = space_mgr.get_entity(entity_id)
                        .and_then(|e| e.player_id).unwrap_or(0);

                    if b_entering {
                        crate::cell::content::fire_enter_region(
                            entity_id, player_id, &tag, engine, tx, space_mgr,
                        ).await;
                    } else {
                        crate::cell::content::fire_exit_region(
                            entity_id, player_id, &tag, engine, tx, space_mgr,
                        ).await;
                    }
                } else {
                    tracing::warn!(entity_id, region_id, "Unknown region ID in triggerClientHintedGenericRegion");
                }
            }
            true
        }

        REQUEST_RELOAD => {
            if !args.is_empty() {
                let _reload_type = args[0];
                tracing::debug!(entity_id, "requestReload");
                handle_reload(entity_id, tx, space_mgr).await;
            }
            true
        }

        CHOSEN_REWARDS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: chosenRewards");
            true
        }

        SET_RING_TRANSPORTER_DEST => {
            if args.len() >= 8 {
                let region_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let destination_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                tracing::info!(entity_id, region_id, destination_id, "UNIMPLEMENTED: setRingTransporterDestination");
            }
            true
        }

        WORLD_INSTANCE_RESET => {
            tracing::info!(entity_id, "UNIMPLEMENTED: onWorldInstanceReset");
            true
        }

        UPDATE_SYSTEM_OPTIONS => {
            tracing::info!(entity_id, "UNIMPLEMENTED: updateSystemOptions");
            true
        }

        _ => false,
    }
}

const ABILITY_RELOAD_WEAPON: i32 = 596;

async fn handle_reload(
    entity_id: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let reload_def = space_mgr.ability_defs.get(&ABILITY_RELOAD_WEAPON).cloned();
    let warmup = reload_def.as_ref().map_or(2.0f32, |d| d.warmup);
    let cooldown = reload_def.as_ref().map_or(1.0f32, |d| d.cooldown);
    let event_set_id = reload_def.as_ref().and_then(|d| d.event_set_id);

    let entity = match space_mgr.get_entity_mut(entity_id) {
        Some(e) => e,
        None => {
            tracing::warn!(entity_id, "requestReload: entity not found");
            return;
        }
    };

    if entity.current_ammo >= entity.max_ammo && entity.reload_complete_at.is_none() {
        tracing::debug!(entity_id, "requestReload: already at max ammo");
        return;
    }

    let old = entity.current_ammo;

    let total_time = warmup + cooldown;
    entity.abilities.start_ability_cooldown(
        ABILITY_RELOAD_WEAPON,
        std::time::Duration::from_secs_f32(total_time),
    );

    // Defer the actual ammo refill until after the warmup so the player can't
    // fire the new magazine before the weapon animation completes. The fire
    // path (cell::abilities::handle_use_ability) checks reload_complete_at and
    // promotes the pending refill on first attempt past the deadline.
    let warmup_duration = std::time::Duration::from_secs_f32(warmup.max(0.0));
    entity.reload_complete_at = Some(std::time::Instant::now() + warmup_duration);

    tracing::info!(entity_id, old, target = entity.max_ammo, warmup, cooldown, "Weapon reload started");

    let timer_args = cimmeria_entity::abilities::serialize_timer_update(
        ABILITY_RELOAD_WEAPON,
        cimmeria_entity::abilities::TIMER_ABILITY_COOLDOWN,
        entity_id as i32,
        total_time,
        0.0,
    );
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 12,
        args: timer_args,
    }).await;

    {
        const BSF_IN_COMBAT: u32 = 1 << 3;
        const BSF_HOLSTER: u32 = 1 << 8;
        if let Some(e) = space_mgr.get_entity_mut(entity_id) {
            let old = e.state_field;
            e.state_field |= BSF_IN_COMBAT;
            e.state_field &= !BSF_HOLSTER;
            if e.state_field != old {
                let new_state = e.state_field;
                let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: 19,
                    args: new_state.to_le_bytes().to_vec(),
                }).await;
            }
        }
    }

    if let Some(esid) = event_set_id {
        use crate::cell::spawner::EVENT_ABILITY_END;

        if let Some(&seq_id) = space_mgr.sequence_map.get(&(esid, EVENT_ABILITY_END)) {
            let mut seq_args = Vec::with_capacity(28);
            seq_args.extend_from_slice(&seq_id.to_le_bytes());
            seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes());
            seq_args.extend_from_slice(&(entity_id as i32).to_le_bytes());
            seq_args.push(1);
            seq_args.extend_from_slice(&0.0f32.to_le_bytes());
            seq_args.extend_from_slice(&0u32.to_le_bytes());
            seq_args.push(0);
            seq_args.extend_from_slice(&0i32.to_le_bytes());
            let _ = tx.send(CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index: 1,
                args: seq_args,
            }).await;
        } else {
            tracing::debug!(entity_id, event_set_id = esid, "reload: no Ability_End sequence found");
        }
    }

    let mut args = Vec::with_capacity(8);
    args.extend_from_slice(&7i32.to_le_bytes());
    let ammo_type = space_mgr.get_entity(entity_id)
        .map_or(0, |e| e.ammo_type);
    args.extend_from_slice(&ammo_type.to_le_bytes());
    let _ = tx.send(CellToBaseMsg::EntityMethodCall {
        entity_id,
        method_index: 7,
        args,
    }).await;
}
