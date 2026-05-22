//! Handler for `BaseToCellMsg::InitPlayerState` — restores persisted player
//! state (missions, abilities, bandolier) onto the cell entity and fires the
//! content engine`s `player_loaded` trigger.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::content;
use crate::cell::messages::{CellToBaseMsg, SavedMission};
use crate::cell::space_manager::SpaceManager;

/// Handles the `InitPlayerState` message: restores player missions, abilities,
/// bandolier items, and fires the content-engine `player_loaded` trigger.
pub(in crate::cell::service) async fn handle_init_player_state(
    entity_id: u32,
    player_id: i32,
    world_name: String,
    archetype_id: i32,
    saved_missions: Vec<SavedMission>,
    abilities: Vec<i32>,
    active_bandolier_slot: i32,
    bandolier_items: Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    tracing::debug!(entity_id, player_id, archetype_id, %world_name, saved_count = saved_missions.len(), ability_count = abilities.len(), "InitPlayerState");
    if let Some(entity) = space_mgr.get_entity_mut(entity_id) {
        entity.player_id = Some(player_id);
        entity.archetype_id = Some(archetype_id);

        // Register player's known abilities on the server-side entity
        for &ability_id in &abilities {
            entity.abilities.add_ability(ability_id);
        }
        tracing::debug!(
            entity_id,
            count = abilities.len(),
            "Registered player abilities on cell entity"
        );

        // Apply bandolier state to entity (Bug #2: restore persisted bandolier slot and items)
        entity.active_bandolier_slot = active_bandolier_slot;
        entity.bandolier_items = bandolier_items.into_iter().collect();
        tracing::debug!(
            entity_id,
            active_bandolier_slot,
            bandolier_item_count = entity.bandolier_items.len(),
            "Applied bandolier state to cell entity"
        );

        // Stage B: Seed each populated bandolier slot's AmmoSlot{N} stat
        // from its persisted current_ammo / clip_size. The default stat
        // tuple is (0,0,0), and `set_slot_ammo` clamps via the stat
        // bounds — without this seed, every later refill/decrement
        // would silently pin to 0. Clearing dirty avoids a duplicate
        // stat send (the initial mapLoaded uses serialize_all()).
        let slot_seed: Vec<(i32, i32, i32)> = entity
            .bandolier_items
            .iter()
            .map(|(&slot, item)| (slot, item.current_ammo, item.clip_size))
            .collect();
        for (slot_id, current, clip) in slot_seed {
            let stat_id = cimmeria_entity::stats::AMMO_SLOT_1 + slot_id;
            if let Some(stat) = entity.stats.get_mut(stat_id) {
                stat.update(0, current, clip);
                stat.clear_dirty();
            }
        }

        // Restore saved missions BEFORE content engine fires, so that
        // chain conditions correctly see existing mission state and
        // don't re-trigger already-active or completed missions.
        for saved in &saved_missions {
            use cimmeria_entity::missions::{
                MissionInstance, MissionObjective, STATUS_ACTIVE, STATUS_COMPLETED,
            };
            let objectives: Vec<MissionObjective> = saved
                .active_objective_ids
                .iter()
                .map(|&oid| {
                    let status = if saved.completed_objective_ids.contains(&oid) {
                        STATUS_COMPLETED
                    } else {
                        STATUS_ACTIVE
                    };
                    MissionObjective {
                        objective_id: oid,
                        status,
                        hidden: false,
                        optional: false,
                    }
                })
                .collect();

            let mut mission = MissionInstance::new(
                saved.mission_id,
                saved.current_step_id.unwrap_or(0),
                objectives,
            );
            mission.status = saved.status;
            mission.completed_steps = saved.completed_step_ids.clone();
            mission.completed_objectives = saved.completed_objective_ids.clone();
            // Without this, `complete()` on a re-accepted repeatable
            // mission post-relog would jump from 0 -> 1 instead of
            // N -> N+1, defeating the numRepeats cap. (#118)
            mission.repeats = saved.repeats;

            entity.missions.add_mission(mission);
            tracing::debug!(
                entity_id,
                mission_id = saved.mission_id,
                status = saved.status,
                "Restored saved mission"
            );
        }
        entity.saved_missions_loaded = true;
    }

    // Send addClientHintedGenericRegion for each client-hinted region in
    // this world. Matches Python Space.playerEntered() → queryRegions():
    // clearClientHintedGenericRegions was already sent in mapLoaded body,
    // now register all regions so the client can fire triggerRegion events.
    {
        use crate::cell::space_manager::REGION_FLAG_CLIENT_HINTED;
        let world_regions: Vec<_> = space_mgr
            .regions_for_world(&world_name)
            .iter()
            .filter(|r| r.flags & REGION_FLAG_CLIENT_HINTED != 0)
            .map(|r| (r.runtime_id, r.height, r.radius, r.flags, r.points.clone()))
            .collect();

        let region_count = world_regions.len();
        for (rid, height, radius, flags, points) in world_regions {
            let mut args = Vec::with_capacity(16 + points.len() * 12);
            args.extend_from_slice(&(rid as i32).to_le_bytes());
            args.extend_from_slice(&height.to_le_bytes());
            args.extend_from_slice(&radius.to_le_bytes());
            args.extend_from_slice(&flags.to_le_bytes());
            args.extend_from_slice(&(points.len() as u32).to_le_bytes()); // ARRAY count
            for p in &points {
                args.extend_from_slice(&p[0].to_le_bytes()); // x
                args.extend_from_slice(&p[1].to_le_bytes()); // y
                args.extend_from_slice(&p[2].to_le_bytes()); // z
            }
            let _ = tx
                .send(CellToBaseMsg::EntityMethodCall {
                    entity_id,
                    method_index: 125, // addClientHintedGenericRegion
                    args,
                })
                .await;
        }
        if region_count > 0 {
            tracing::info!(
                entity_id, player_id, world = %world_name,
                count = region_count, "Sent region registrations"
            );
        }
    }

    content::fire_player_loaded(entity_id, player_id, &world_name, engine, tx, space_mgr)
        .await;
}

