//! Handler for `BaseToCellMsg::InitPlayerState` — restores persisted player
//! state (missions, abilities, bandolier) onto the cell entity and fires the
//! content engine's `player_loaded` trigger.

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
    system_options: cimmeria_entity::cell_entity::SystemOptions,
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

        // Apply bandolier state to entity — restore persisted bandolier slot and items
        entity.active_bandolier_slot = active_bandolier_slot;
        entity.bandolier_items = bandolier_items.into_iter().collect();
        tracing::debug!(
            entity_id,
            active_bandolier_slot,
            bandolier_item_count = entity.bandolier_items.len(),
            "Applied bandolier state to cell entity"
        );

        // Apply server-synced client options. Without this assignment
        // the entity would silently fall back to `SystemOptions::default()`
        // on every login — the user could toggle the checkbox in-game,
        // see it appear to save (we persist to DB), then find it back
        // on default after a relog. The hydrate path closes that loop.
        entity.system_options = system_options;
        tracing::debug!(
            entity_id,
            auto_reload = entity.system_options.auto_reload,
            reload_on_activate = entity.system_options.reload_on_activate,
            "Applied system options to cell entity"
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
                    method_index: crate::mercury::method_idx::ADD_CLIENT_HINTED_GENERIC_REGION,
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

    content::fire_player_loaded(entity_id, player_id, &world_name, engine, tx, space_mgr).await;
}

#[cfg(test)]
mod system_options_assignment_tests {
    //! The InitPlayerState handler is the hydrate-on-login site for
    //! `CellEntity::system_options`. These guards pin that the
    //! incoming `SystemOptions` actually lands on the entity — without
    //! this, a regression that drops the field assignment would let
    //! the cell fall back to `SystemOptions::default()` every login
    //! and the user's saved checkbox values would silently revert
    //! after every reconnect.

    use super::*;
    use cimmeria_entity::cell_entity::SystemOptions;

    fn make_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        mgr
    }

    /// The hydrated SystemOptions block must replace the entity's
    /// default. Bug shape: a refactor that drops the assignment
    /// silently leaves auto_reload=true / reload_on_activate=false
    /// regardless of what the DB returned.
    #[tokio::test]
    async fn init_player_state_assigns_system_options() {
        let mut mgr = make_mgr();
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(32);
        // Hydrate values DIFFERENT from `SystemOptions::default()` so a
        // missed assignment is observable. Defaults are auto_reload=true,
        // reload_on_activate=false; flip both.
        let hydrated = SystemOptions {
            auto_reload: false,
            reload_on_activate: true,
        };

        handle_init_player_state(
            1,
            100,
            "Castle_CellBlock".into(),
            1,
            vec![],
            vec![],
            0,
            vec![],
            hydrated.clone(),
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(
            e.system_options, hydrated,
            "InitPlayerState must overwrite the entity's default \
             SystemOptions with the DB-hydrated values",
        );
    }

    /// Hydrating with the same value as the default still has to
    /// assign (not skip) — otherwise a hand-edited row that explicitly
    /// stores the defaults could be silently treated as "unset" if
    /// somebody added a "skip if equals default" optimisation.
    #[tokio::test]
    async fn init_player_state_assigns_default_values_explicitly() {
        let mut mgr = make_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            // Pre-stuff the entity with non-defaults so the assignment
            // is observable even when the hydrated value is default.
            p.system_options.auto_reload = false;
            p.system_options.reload_on_activate = true;
        }
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(32);

        handle_init_player_state(
            1,
            100,
            "Castle_CellBlock".into(),
            1,
            vec![],
            vec![],
            0,
            vec![],
            SystemOptions::default(),
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(
            e.system_options,
            SystemOptions::default(),
            "InitPlayerState must always overwrite — even an explicit \
             default-equal hydrate must reset prior in-memory state",
        );
    }
}
