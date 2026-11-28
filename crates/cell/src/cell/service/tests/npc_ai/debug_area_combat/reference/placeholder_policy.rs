//! NC-30 (D-NC1): the placeholder policy is `RequireWeaponEvidence`. An NPC
//! whose template has no ability set gets 592 Pistol Shot only with a
//! human firearm component. A creature, or a `WP-Jaffa.*` NPC, knows no ability,
//! holds fire, and logs one `combat_profile_unresolved` per template.

mod live_db {
    use std::collections::HashMap;

    use super::super::super::live_db::{fight_space, pass, set_pools};
    use super::super::super::{PLAYER, WORLD};
    use crate::cell::abilities::warmup_tick;
    use crate::cell::content::EngineEvents;
    use crate::cell::service::npc_ai::{force_ai_state, npc_ai_tick};
    use crate::cell::space_manager::SpaceManager;
    use crate::cell::spawner::{load_spawn_templates, SpawnRecord};
    use crate::test_support::{require_db_or_skip, LogCapture};
    use cimmeria_content_engine::chain::ChainEngine;
    use cimmeria_entity::cell_entity::AiState;
    use cimmeria_entity::stats::HEALTH;
    use tokio::sync::mpsc;

    /// Template 4 (the PRU) with its ability set and components replaced, under
    /// the sentinel id `template_id`. A sentinel keeps this test's
    /// once-per-template report (`first_report`) clear of the other tests.
    fn probe(
        templates: &HashMap<i32, SpawnRecord>,
        template_id: i32,
        body_set: &str,
        components: Option<Vec<String>>,
    ) -> SpawnRecord {
        let mut record = templates.get(&4).expect("seeded template 4").clone();
        record.template_id = template_id;
        record.template_name = format!("NC30_PROBE_{template_id:x}");
        record.body_set = body_set.to_string();
        record.components = components;
        record.ability_ids = Vec::new();
        record.world_name = WORLD.to_string();
        record
    }

    /// Spawn `record` as `id` at `(x, 0, z)`, Idle, in the fight space.
    fn place(mgr: &mut SpaceManager, record: &SpawnRecord, id: u32, x: f32, z: f32) {
        let mut record = record.clone();
        (record.x, record.y, record.z) = (x, 0.0, z);
        record.tag = Some(format!("CIMMERIA_TEST_NC30_{id}"));
        mgr.spawn_npc_from_record(id, &record).expect("spawn");
        force_ai_state(mgr.get_entity_mut(id).unwrap(), AiState::Idle);
    }

    fn weapon(component: &str) -> Option<Vec<String>> {
        Some(vec![component.to_string()])
    }

    /// A creature (`MOB_*`) with an empty set and no `WP-` component knows no
    /// ability. At 5 m it holds fire: the player is undamaged and the fight
    /// records `no_ability`. Revert proof: under `Legacy` the creature takes 592
    /// and fires, so the player loses Health and no `no_ability` row appears.
    #[tokio::test]
    async fn live_db_creature_without_evidence_holds_fire() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool).await.expect("templates");
        let logs = LogCapture::install();
        let mut mgr = fight_space(&pool, [0.0, 0.0, 0.0]).await;
        set_pools(&mut mgr, PLAYER, 1000, 1000);
        mgr.connect_entity(PLAYER);
        const NPC: u32 = 300_501;
        let record = probe(&templates, 0x7000_0301, "MOB_Creature", None);
        place(&mut mgr, &record, NPC, 5.0, 0.0);
        assert!(
            mgr.get_entity(NPC)
                .unwrap()
                .abilities
                .known_ability_ids()
                .is_empty(),
            "a creature with no set and no WP- knows no ability"
        );
        let _ = mgr.compute_aoi_changes();
        let (tx, mut rx) = mpsc::channel(65_536);
        for _ in 0..4 {
            pass(&mut mgr, &[NPC], &tx, &mut rx).await;
        }
        let npc = NPC.to_string();
        assert!(
            logs.all()
                .into_iter()
                .any(|c| c.has_field("npc_id", &npc)
                    && c.has_field("decision_outcome", "no_ability")),
            "the fight records no_ability"
        );
        let p = mgr.get_entity(PLAYER).unwrap();
        assert_eq!(p.stats.get(HEALTH).unwrap().cur, 1000, "undamaged");
    }

    /// A human firearm earns 592 even with no set: the NPC knows it and
    /// starts it at 5 m (its cooldown is set) with no unresolved row. Revert
    /// proof: dropping the firearm grant leaves the NPC knowing nothing.
    #[tokio::test]
    async fn live_db_human_with_firearm_still_knows_pistol_shot() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool).await.expect("templates");
        let logs = LogCapture::install();
        let mut mgr = fight_space(&pool, [0.0, 0.0, 0.0]).await;
        set_pools(&mut mgr, PLAYER, 1000, 1000);
        mgr.connect_entity(PLAYER);
        const NPC: u32 = 300_502;
        let template_id = 0x7000_0302;
        let record = probe(
            &templates,
            template_id,
            "BS_Human",
            weapon("WP-Human.WP_SMG_1A"),
        );
        place(&mut mgr, &record, NPC, 5.0, 0.0);
        assert_eq!(
            mgr.get_entity(NPC).unwrap().abilities.known_ability_ids(),
            vec![592],
            "a human with a WP-Human firearm knows Pistol Shot"
        );
        let _ = mgr.compute_aoi_changes();
        let engine = ChainEngine::new();
        let events = EngineEvents(&engine);
        let (tx, mut rx) = mpsc::channel(65_536);
        let mut fired = false;
        for _ in 0..4 {
            npc_ai_tick(&tx, &mut mgr, &events).await;
            if let Some(pc) = mgr
                .get_entity_mut(NPC)
                .and_then(|e| e.pending_cast.as_mut())
            {
                pc.fire_at = std::time::Instant::now();
            }
            warmup_tick(&tx, &mut mgr, &events).await;
            while rx.try_recv().is_ok() {}
            if mgr.get_entity(NPC).unwrap().abilities.is_on_cooldown(592) {
                fired = true;
                break;
            }
        }
        assert!(fired, "the WP-Human NPC fires Pistol Shot (592) at 5 m");
        let unresolved = logs.all().into_iter().any(|c| {
            c.has_field("event", "combat_profile_unresolved")
                && c.has_field("template_id", &template_id.to_string())
        });
        assert!(!unresolved, "a WP-Human NPC is not unresolved");
    }

    /// A `WP-Jaffa.*` weapon is not a human firearm: the template with no set
    /// logs exactly one `combat_profile_unresolved` for two spawns, and both
    /// NPCs hold fire at 5 m. Revert proof: under `Legacy` no unresolved row
    /// is logged and both NPCs fire 592.
    #[tokio::test]
    async fn live_db_jaffa_weapon_without_human_firearm_holds_fire() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool).await.expect("templates");
        let logs = LogCapture::install();
        let mut mgr = fight_space(&pool, [0.0, 0.0, 0.0]).await;
        set_pools(&mut mgr, PLAYER, 1000, 1000);
        mgr.connect_entity(PLAYER);
        let template_id = 0x7000_0303;
        let record = probe(
            &templates,
            template_id,
            "BS_Jaffa",
            weapon("WP-Jaffa.WP_Staff"),
        );
        const NPCS: [u32; 2] = [300_503, 300_504];
        place(&mut mgr, &record, NPCS[0], 5.0, 0.0);
        place(&mut mgr, &record, NPCS[1], -5.0, 0.0);
        for id in NPCS {
            assert!(
                mgr.get_entity(id)
                    .unwrap()
                    .abilities
                    .known_ability_ids()
                    .is_empty(),
                "a WP-Jaffa NPC knows no ability"
            );
        }
        let _ = mgr.compute_aoi_changes();
        let (tx, mut rx) = mpsc::channel(65_536);
        for _ in 0..4 {
            pass(&mut mgr, &NPCS, &tx, &mut rx).await;
        }
        let unresolved = logs
            .all()
            .into_iter()
            .filter(|c| {
                c.target == "npc.combat_profile"
                    && c.has_field("template_id", &template_id.to_string())
            })
            .count();
        assert_eq!(unresolved, 1, "one combat_profile_unresolved per template");
        let p = mgr.get_entity(PLAYER).unwrap();
        assert_eq!(p.stats.get(HEALTH).unwrap().cur, 1000, "undamaged");
    }
}
