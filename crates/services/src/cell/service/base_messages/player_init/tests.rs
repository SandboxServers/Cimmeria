//! Tests for the `InitPlayerState` handler and `send_known_abilities_update`
//! helper in [`super`]. Split out of `player_init/mod.rs` to keep the handler
//! file under the size cap; pure test-code move (no logic changes).

#[cfg(test)]
mod state_field_restore_tests {
    //! **#412 restore guards.** `InitPlayerState` must restore the
    //! persisted preference bits of `state_field` (today:
    //! `BSF_AutoCycling`), re-arm `abilities.auto_cycle`, and
    //! re-broadcast `onStateFieldUpdate` so the client's button
    //! highlight survives the relog — while a corrupt row carrying
    //! transient combat bits must be masked out so the player never
    //! spawns dead / frozen / in-combat.

    use super::super::*;
    use crate::cell::combat::{BSF_AUTO_CYCLING, BSF_DEAD, BSF_IN_COMBAT, BSF_MOVEMENT_LOCK};
    use cimmeria_entity::cell_entity::SystemOptions;

    fn make_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
        }
        mgr.connect_entity(1);
        mgr
    }

    fn drain_state_field_broadcasts(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<u32> {
        let mut out = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                args,
            } = msg
            {
                if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE
                    && args.len() == 4
                {
                    out.push(u32::from_le_bytes([args[0], args[1], args[2], args[3]]));
                }
            }
        }
        out
    }

    /// Happy path: persisted BSF_AutoCycling restores onto the entity,
    /// re-arms the loop flag, and re-broadcasts the bit to the client.
    /// This is the acceptance shape from #412 — after relog, the first
    /// attack enters `arm_auto_cycle` (gated on `abilities.auto_cycle`)
    /// and the loop drives itself without a second button press.
    #[tokio::test]
    async fn restores_auto_cycle_bit_and_rearms_loop_flag() {
        let mut mgr = make_mgr();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(64);

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
            BSF_AUTO_CYCLING,
            0, // access_level
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert_ne!(
            e.state_field & BSF_AUTO_CYCLING,
            0,
            "persisted BSF_AutoCycling must restore onto the entity"
        );
        assert!(
            e.abilities.auto_cycle,
            "auto_cycle must re-arm so the first attack of the session \
             enters arm_auto_cycle and the loop starts (the #412 symptom \
             was exactly this flag staying false)"
        );
        assert!(
            e.abilities.auto_cycle_ability_id.is_none(),
            "the ability stash stays empty until first commit, by design"
        );

        let broadcasts = drain_state_field_broadcasts(&mut rx);
        assert_eq!(
            broadcasts,
            vec![BSF_AUTO_CYCLING],
            "restore must re-broadcast onStateFieldUpdate so the client's \
             gun-icon button highlights without an in-session toggle"
        );
    }

    /// Mask guard: a corrupt / hand-edited row carrying transient combat
    /// bits must NOT restore them — spawning dead + movement-locked is
    /// the failure shape the mask exists to prevent. No broadcast either
    /// (nothing legitimate was restored).
    #[tokio::test]
    async fn transient_bits_in_saved_row_are_not_restored() {
        let mut mgr = make_mgr();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(64);

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
            BSF_DEAD | BSF_IN_COMBAT | BSF_MOVEMENT_LOCK,
            0, // access_level
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let e = mgr.get_entity(1).unwrap();
        assert_eq!(
            e.state_field, 0,
            "transient combat bits must be masked out on restore — a \
             relog is always a clean combat slate"
        );
        assert!(!e.abilities.auto_cycle, "loop flag must stay disarmed");
        assert!(
            drain_state_field_broadcasts(&mut rx).is_empty(),
            "nothing restored → no onStateFieldUpdate"
        );
    }

    /// Zero-state companion: the common case (player never touched the
    /// button) must not emit a spurious state-field packet during the
    /// already-busy world-entry burst.
    #[tokio::test]
    async fn zero_state_field_emits_no_broadcast() {
        let mut mgr = make_mgr();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(64);

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
            0,
            0, // access_level
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        assert!(
            drain_state_field_broadcasts(&mut rx).is_empty(),
            "state_field 0 must not add a packet to the login burst"
        );
    }
}

#[cfg(test)]
mod relog_mission_resurrection_tests {
    //! **#411 end-to-end guard.** A mis-gated `player_loaded` chain (one
    //! whose author forgot the `mission_status eq not_active` condition)
    //! fires `accept_mission` on every world entry. Pre-fix, that
    //! overwrote a relog-restored COMPLETED mission with a fresh ACTIVE
    //! instance and persisted `MissionUpdate status=1` over the saved
    //! row — completed missions reappeared as active in the quest log
    //! after relog. The server-side offer guard must hold even when the
    //! chain data is wrong.

    use super::super::*;
    use crate::cell::messages::SavedMission;
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;
    use cimmeria_entity::missions::MISSION_COMPLETED;

    #[tokio::test]
    async fn mis_gated_player_loaded_chain_cannot_resurrect_completed_mission() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
            .unwrap();
        mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
            .unwrap();
        mgr.connect_entity(1);
        mgr.mission_defs.insert(
            622,
            crate::cell::spawner::MissionDefEntry {
                step_id: 2113,
                objectives: vec![],
                is_hidden: false,
                num_repeats: 1,
                can_repeat_on_fail: true,
            },
        );

        // The mis-gated chain: player_loaded, NO conditions, accepts 622.
        let mut engine = ChainEngine::new();
        engine.register_chain(Chain {
            id: 9999,
            name: "mis-gated 622 grant (no not_active condition)".into(),
            enabled: true,
            trigger: Trigger::OnPlayerLoaded { world_name: None },
            conditions: vec![],
            actions: vec![Action::AcceptMission { mission_id: 622 }],
            priority: 0,
        });

        // Relog payload: 622 completed, repeat counter at the cap.
        let saved = vec![SavedMission {
            mission_id: 622,
            status: MISSION_COMPLETED,
            current_step_id: None,
            completed_step_ids: vec![2113],
            completed_objective_ids: vec![],
            active_objective_ids: vec![],
            failed_objective_ids: vec![],
            repeats: 2, // > num_repeats = 1 → not re-offerable
        }];

        let (tx, mut rx) = mpsc::channel(64);
        handle_init_player_state(
            1,
            100,
            "Castle_CellBlock".into(),
            1,
            saved,
            vec![],
            0,
            vec![],
            cimmeria_entity::cell_entity::SystemOptions::default(),
            0, // state_field
            0, // access_level
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        // Cell-side state: still completed, repeat counter intact.
        let m = mgr
            .get_entity(1)
            .unwrap()
            .missions
            .get_mission(622)
            .expect("restored mission must still be tracked");
        assert_eq!(
            m.status, MISSION_COMPLETED,
            "the offer guard must keep the restored mission COMPLETED even \
             when a mis-gated player_loaded chain re-fires accept_mission"
        );
        assert_eq!(m.repeats, 2, "repeat counter must survive the relog");

        // Wire/persist side: no MissionUpdate(status=1) may have been sent —
        // that's the message that UPSERTs "active" over the saved row.
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::MissionUpdate {
                mission_id, status, ..
            } = msg
            {
                assert!(
                    !(mission_id == 622 && status == 1),
                    "relog must not persist MissionUpdate(622, status=1) — \
                     this is the exact write that resurrected completed \
                     missions as active (#411)"
                );
            }
        }
    }
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

    use super::super::*;
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
            0, // state_field
            0, // access_level
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

    /// **#475 plumbing guard.** `InitPlayerState` must land the
    /// session's `access_level` on `CellEntity::access_level` — that's
    /// the authoritative value the cell-method GM gate reads. A refactor
    /// that drops the assignment would silently leave every entity at
    /// access_level 0, so legitimate GMs lose their commands AND (worse,
    /// once the gate is the only check) the plumbing that makes the gate
    /// meaningful is gone. Pin a non-zero level so a missed assignment is
    /// observable.
    #[tokio::test]
    async fn init_player_state_assigns_access_level() {
        let mut mgr = make_mgr();
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(32);

        const GM_LEVEL: u32 = 2; // GameMaster

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
            0, // state_field
            GM_LEVEL,
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        assert_eq!(
            mgr.get_entity(1).unwrap().access_level,
            GM_LEVEL,
            "InitPlayerState must store the session access_level on the \
             cell entity — the GM gate has nothing to check otherwise (#475)"
        );
    }

    /// **Lomiada's 2026-06-04 18:09 session regression.**
    /// `InitPlayerState` must seed the cell-entity's stats from the
    /// archetype's base values so the damage scripts see non-zero
    /// FOCUS + HEALTH pools. Without this, `RangedPhysicalDamage`
    /// reads `target.stats.get(FOCUS).cur` as 0 → focus_overflow ==
    /// focus_damage every shot → no shield absorb → full spillover
    /// damage every hit.
    ///
    /// Pins both pools at non-zero with the Soldier archetype
    /// values (760 HEALTH, 1570 FOCUS — straight out of the
    /// hardcoded `mercury::world_data::stats::archetype_stats`
    /// table). Reverting the `apply_archetype` call in
    /// `handle_init_player_state` trips this guard by leaving the
    /// stats at the `StatList::new()` default of (0, 0, 0).
    #[tokio::test]
    async fn init_player_state_seeds_stats_from_archetype() {
        use cimmeria_entity::stats::{FOCUS, HEALTH};

        let mut mgr = make_mgr();
        let engine = ChainEngine::new();
        let (tx, _rx) = mpsc::channel(32);

        handle_init_player_state(
            1,
            100,
            "Castle_CellBlock".into(),
            1, // archetype_id = Soldier
            vec![],
            vec![],
            0,
            vec![],
            SystemOptions::default(),
            0, // state_field
            0, // access_level
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        let e = mgr.get_entity(1).unwrap();

        let health = e
            .stats
            .get(HEALTH)
            .expect("HEALTH stat must exist on cell entity");
        assert_eq!(
            health.max, 760,
            "Soldier max HEALTH must be seeded to the archetype's 760 — \
             pre-fix the default (0, 0, 0) tuple meant respawn would \
             restore 0 HP and the player would spawn dead.",
        );
        assert_eq!(
            health.cur, 760,
            "Initial HEALTH cur must equal max — apply_archetype \
             sets (min=0, cur=max, max=max)",
        );

        let focus = e
            .stats
            .get(FOCUS)
            .expect("FOCUS stat must exist on cell entity");
        assert_eq!(
            focus.max, 1570,
            "Soldier max FOCUS must be seeded to the archetype's 1570",
        );
        assert_eq!(
            focus.cur, 1570,
            "Initial FOCUS cur must equal max — without this, \
             RangedPhysicalDamage absorbs zero (overflow == full \
             damage every shot, 86 HP per hit on lomiada's session).",
        );
    }

    /// **RE-driven defensive resync.** `InitPlayerState` must re-emit
    /// `onActiveSlotUpdate` to the client AFTER `onClientReady`,
    /// carrying the player's persisted active bandolier slot. The Ghidra-recovered client behaviour is that
    /// the NetIn handler `FUN_00da9ce0` walks `SGWPlayer.bagList`
    /// (at `+0x8c → +0x24`) and silently no-ops when the bag-list
    /// map is uninitialized. The login-burst `onActiveSlotUpdate`
    /// from `mapLoaded` can arrive before the bag-init packets in
    /// the same bundle are processed, leaving the cached active-slot
    /// value at the default (0). The Lua gate inside
    /// `BandolierMod.ActivateBandolierSlotN` then suppresses the
    /// wire emit for any F-key matching the stale value — symptom:
    /// "F2 doesn't swap to the P90," reported by lomiada
    /// 2026-06-04 18:09 with zero `requestActiveSlotChange` events
    /// in the entire session.
    ///
    /// Pin: handler must emit at least one `onActiveSlotUpdate`
    /// (method index `crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE`)
    /// carrying bag_id=3 and the 1-indexed wire slot (server slot + 1).
    /// Reverting the resend block trips this by leaving the only
    /// `onActiveSlotUpdate` emission on the wire as the one from the
    /// login burst — which the InitPlayerState handler doesn't see
    /// directly (it's a separate wire site).
    #[tokio::test]
    async fn init_player_state_resends_active_slot_update_for_resync() {
        let mut mgr = make_mgr();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(64);
        const SERVER_SLOT: i32 = 1; // arbitrary non-zero so the bag_id=3 + (slot+1)=2 encoding is observable

        handle_init_player_state(
            1,
            100,
            "Castle_CellBlock".into(),
            1,      // archetype_id
            vec![], // no abilities
            vec![], // no missions
            SERVER_SLOT,
            vec![], // no bandolier items (slot can still be active over an empty slot)
            SystemOptions::default(),
            0, // state_field
            0, // access_level
            &tx,
            &mut mgr,
            &engine,
        )
        .await;

        // Walk the channel for the onActiveSlotUpdate emit. Decode the
        // 8-byte payload: bag_id (i32 LE) + wire_slot (i32 LE).
        let mut found = None;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                args,
            } = msg
            {
                if method_index == crate::cell::client_methods::inventory::ON_ACTIVE_SLOT_UPDATE
                    && args.len() == 8
                {
                    let bag_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                    let wire_slot = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                    found = Some((bag_id, wire_slot));
                    break;
                }
            }
        }

        let (bag_id, wire_slot) = found.expect(
            "InitPlayerState must re-send onActiveSlotUpdate as a defensive \
             resync against the client bag-list init race documented in \
             docs/reverse-engineering/findings/client-wire-emit-suppression.md",
        );
        assert_eq!(bag_id, 3, "bag_id must be CONTAINER_BANDOLIER (3)");
        assert_eq!(
            wire_slot,
            SERVER_SLOT + 1,
            "wire slot is 1-indexed (server slot {SERVER_SLOT} + 1)"
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
                    instance_id: 0,
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
            0, // state_field
            0, // access_level
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
            0, // state_field
            0, // access_level
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
            0, // state_field
            0, // access_level
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
            0, // state_field
            0, // access_level
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
            0, // state_field
            0, // access_level
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
