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
///
/// Wrapped in a `world_entry.init_player_state` span with all the per-burst
/// counts (saved_missions, abilities, bandolier_items, regions) as fields
/// so SigNoz can correlate freeze symptoms against initial-load burst size.
/// See the freeze investigation in the 15:50:49Z session — bursts > N
/// regions or > M missions are the suspected client-stall trigger.
#[tracing::instrument(
    name = "world_entry.init_player_state",
    level = "info",
    skip(saved_missions, abilities, bandolier_items, system_options, tx, space_mgr, engine),
    fields(
        entity_id,
        player_id,
        archetype_id,
        world = %world_name,
        saved_missions = saved_missions.len(),
        abilities = abilities.len(),
        bandolier_items = bandolier_items.len(),
        active_bandolier_slot,
        regions = tracing::field::Empty,
    ),
)]
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
    //
    // BURST WARNING: this loop can fire 20+ packets within ~1ms with no
    // throttling. Combined with mission-replay + appearance burst that
    // precedes it, this has been observed to stall some clients on
    // existing-character login (freeze investigation, 2026-05-26). The
    // span around it captures the burst size + duration so freezes can
    // be correlated to specific bursts in SigNoz.
    {
        use crate::cell::space_manager::REGION_FLAG_CLIENT_HINTED;
        let world_regions: Vec<_> = space_mgr
            .regions_for_world(&world_name)
            .iter()
            .filter(|r| r.flags & REGION_FLAG_CLIENT_HINTED != 0)
            .map(|r| (r.runtime_id, r.height, r.radius, r.flags, r.points.clone()))
            .collect();

        let region_count = world_regions.len();
        let burst_span = tracing::info_span!(
            "world_entry.region_burst",
            entity_id,
            world = %world_name,
            count = region_count,
        );
        let _burst_guard = burst_span.enter();
        let burst_start = std::time::Instant::now();
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
        let burst_elapsed = burst_start.elapsed();
        tracing::Span::current().record("regions", region_count);
        if region_count > 0 {
            tracing::info!(
                entity_id, player_id, world = %world_name,
                count = region_count,
                burst_micros = burst_elapsed.as_micros() as u64,
                "Sent region registrations"
            );
        }
    }

    // `reloadOnActivate` activation site for the world-entry path
    // (initial login, gate travel, cross-world ring). Same-world
    // respawn is deliberately NOT a trigger site — `ReanchorPlayer`
    // keeps the cell entity intact, so the weapon is never
    // "activated" in the sense `SystemOptions.xml` defines. Helper
    // self-gates on option-off, non-player, melee, full clip, and
    // in-flight reload, so the unconditional call is safe.
    crate::cell::cell_methods::player::world::maybe_trigger_reload_on_activate(
        entity_id, tx, space_mgr,
    )
    .await;

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

    /// Stage an InitPlayerState payload with one bandolier item in
    /// slot 0 (clip 30) and the given `current_ammo` (so 30 = full,
    /// 10 = partial, 30 = no-op for the reload check). Stamps the
    /// caller-chosen `system_options` so each test exercises the
    /// gate path it intends. Returns the tuple bound to
    /// `handle_init_player_state`'s positional args.
    fn init_args_with_bandolier_clip(
        current_ammo: i32,
        system_options: SystemOptions,
    ) -> (
        i32,
        Vec<i32>,
        i32,
        Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)>,
        SystemOptions,
    ) {
        (
            1,      // archetype_id
            vec![], // no abilities
            0,      // active_bandolier_slot
            vec![(
                0,
                cimmeria_entity::cell_entity::BandolierItem {
                    item_id: 55,
                    clip_size: 30,
                    default_ammo_type: 1,
                    current_ammo,
                    cur_ammo_type: 1,
                },
            )],
            system_options,
        )
    }

    /// World entry with `reloadOnActivate = true` AND a partial-clip
    /// active bandolier weapon must trigger an automatic reload, the
    /// same as F1-F4 swap or in-game equip. Without this hook a
    /// player who logs in, gate-travels, or cross-world rings to a
    /// new map with a half-empty active weapon silently doesn't
    /// auto-reload until they manually swap slots and back —
    /// surfacing in play as "my option does nothing on login."
    ///
    /// Setup: 10/30 active clip + `reload_on_activate = true`.
    /// Assertion: `reload_complete_at` is `Some` post-handler. The
    /// Phase A draw guard (weapon holstered + threatened_mobs empty)
    /// is bypassed because `weapon_holstered` defaults to `false`
    /// on a freshly-restored entity in this test fixture; in
    /// production the same `handle_reload` path would walk through
    /// Phase A naturally if the weapon were holstered.
    #[tokio::test]
    async fn init_player_state_triggers_reload_on_activate_when_clip_partial() {
        let mut mgr = make_mgr();
        // Need the ABILITY_RELOAD_WEAPON def for `handle_reload` to
        // resolve warmup/cooldown — otherwise it falls back to its
        // hardcoded 2.0/1.0 defaults and the reload still fires.
        mgr.ability_defs.insert(
            596 /* ABILITY_RELOAD_WEAPON — mirrors the const in cell_methods/player/world */,
            cimmeria_entity::abilities::AbilityDef {
                ability_id: 596 /* ABILITY_RELOAD_WEAPON — mirrors the const in cell_methods/player/world */,
                is_ranged: false,
                min_range: 0,
                name: "Reload".into(),
                warmup: 1.0,
                cooldown: 0.5,
                flags: 0,
                max_range: 0,
                target_type_id: 0,
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
        // Pre-mark the weapon as drawn so the Phase A draw window
        // doesn't apply. This isolates the test to the
        // reload-on-activate trigger; Phase A is exercised by the
        // `handle_reload` tests in `cell_methods/player/world`.
        if let Some(p) = mgr.get_entity_mut(1) {
            p.weapon_holstered = false;
        }
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(32);

        let (archetype, abilities, slot, items, sys_opts) = init_args_with_bandolier_clip(
            10,
            SystemOptions {
                auto_reload: true,
                reload_on_activate: true,
            },
        );

        handle_init_player_state(
            1,
            100,
            "Castle_CellBlock".into(),
            archetype,
            vec![], // saved_missions
            abilities,
            slot,
            items,
            sys_opts,
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert!(
            e.reload_complete_at.is_some(),
            "InitPlayerState with reload_on_activate=true + partial clip \
             must trigger handle_reload (gate-travel / login coverage). \
             Pre-fix the only triggers were F1-F4 swap and inventory \
             equip — login silently no-op'd."
        );
    }

    /// Default-holstered world-entry path: the real production fixture
    /// for login / gate-travel / cross-world ring has
    /// `weapon_holstered = true` (the `CellEntity::new` default — the
    /// pawn is freshly instantiated and starts with no weapon drawn).
    /// The drawn-weapon test above isolates the trigger logic from
    /// Phase A draw choreography; this companion guard pins that the
    /// holstered-weapon path also queues a reload. Bug shape: a
    /// future refactor that adds a `weapon_holstered` short-circuit to
    /// `maybe_trigger_reload_on_activate` would silently break every
    /// real login.
    ///
    /// Assertion: `pending_reload_at` is `Some` (Phase A draw deadline)
    /// post-handler. Phase A is the correct entry path because the
    /// weapon is still holstered and OOC — `handle_reload` defers the
    /// real reload until the draw animation has played.
    #[tokio::test]
    async fn init_player_state_triggers_reload_on_activate_when_holstered() {
        let mut mgr = make_mgr();
        mgr.ability_defs.insert(
            596,
            cimmeria_entity::abilities::AbilityDef {
                ability_id: 596,
                is_ranged: false,
                min_range: 0,
                name: "Reload".into(),
                warmup: 1.0,
                cooldown: 0.5,
                flags: 0,
                max_range: 0,
                target_type_id: 0,
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
        // Sanity: the fixture starts with the production default
        // (holstered=true). Pin it so a future fixture change can't
        // silently relax this test into a no-op.
        assert!(
            mgr.get_entity(1).unwrap().weapon_holstered,
            "fixture sanity: CellEntity::new defaults weapon_holstered=true"
        );
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(32);

        let (archetype, abilities, slot, items, sys_opts) = init_args_with_bandolier_clip(
            10,
            SystemOptions {
                auto_reload: true,
                reload_on_activate: true,
            },
        );

        handle_init_player_state(
            1,
            100,
            "Castle_CellBlock".into(),
            archetype,
            vec![],
            abilities,
            slot,
            items,
            sys_opts,
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert!(
            e.pending_reload_at.is_some(),
            "holstered world-entry with reload_on_activate=true + partial clip \
             must queue Phase A (the draw window); pre-fix only the slot-swap \
             trigger path covered this and login was silent"
        );
        assert!(
            !e.weapon_holstered,
            "Phase A entry must flip weapon_holstered=false so the draw \
             animation plays — without the flip the client renders the \
             reload with the weapon still holstered"
        );
    }

    /// Symmetric negative: option DEFAULT (`reload_on_activate = false`)
    /// must NOT trigger on login, even with a partial clip. The XML
    /// default is off — players who never touched the checkbox
    /// shouldn't get behavior change.
    #[tokio::test]
    async fn init_player_state_does_not_reload_when_option_off() {
        let mut mgr = make_mgr();
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(32);

        let (archetype, abilities, slot, items, sys_opts) = init_args_with_bandolier_clip(
            10, // partial clip
            SystemOptions {
                auto_reload: true,
                reload_on_activate: false, // XML default
            },
        );

        handle_init_player_state(
            1,
            100,
            "Castle_CellBlock".into(),
            archetype,
            vec![],
            abilities,
            slot,
            items,
            sys_opts,
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert!(
            e.reload_complete_at.is_none() && e.pending_reload_at.is_none(),
            "InitPlayerState with option off must NOT trigger any reload"
        );
    }

    /// Symmetric negative #2: full clip on login + option on → no-op.
    /// `maybe_trigger_reload_on_activate` has a `active_ammo() <
    /// active_clip_size()` gate; this guard pins that gate at the
    /// handler boundary so a future refactor that removes it (e.g.
    /// to "always reload on activate") would trip here.
    #[tokio::test]
    async fn init_player_state_does_not_reload_when_clip_full() {
        let mut mgr = make_mgr();
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(32);

        let (archetype, abilities, slot, items, sys_opts) = init_args_with_bandolier_clip(
            30, // already full
            SystemOptions {
                auto_reload: true,
                reload_on_activate: true,
            },
        );

        handle_init_player_state(
            1,
            100,
            "Castle_CellBlock".into(),
            archetype,
            vec![],
            abilities,
            slot,
            items,
            sys_opts,
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert!(
            e.reload_complete_at.is_none() && e.pending_reload_at.is_none(),
            "full-clip InitPlayerState must NOT queue a reload"
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
