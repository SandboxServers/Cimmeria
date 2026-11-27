//! NC-20 (D-NC4): a Jaffa inside `NPC_MELEE_RANGE` takes its melee swing
//! (710) before its ranged auto-attack (584), which has the lower id.
//! Pets are exempt and are pinned in `cell-combat`'s selector tests.

mod live_db {
    use super::super::super::live_db::{fight_space, set_pools, spawn};
    use super::super::super::PLAYER;
    use cimmeria_content_engine::chain::ChainEngine;
    use tokio::sync::mpsc;

    use crate::cell::abilities::warmup_tick;
    use crate::cell::content::EngineEvents;
    use crate::cell::service::npc_ai::npc_ai_tick;
    use crate::cell::spawner::load_spawn_templates;
    use crate::test_support::require_db_or_skip;

    /// The seeded Jaffa Guard (1380, set 4) two metres from the player. An Idle
    /// NPC first aggroes on one AI tick, then starts an ability on a later
    /// one, so passes run (each warmup fired at once) until an ability starts.
    /// The ability that starts is 710, so 710 is on cooldown and 584 is not.
    /// Deliberately no `pass`, which clears cooldowns. Revert proof: without
    /// the melee-first pass the first ability started is 584 and this fails.
    #[tokio::test]
    async fn live_db_reference_jaffa_swings_staff_in_reach() {
        let pool = require_db_or_skip!();
        let templates = load_spawn_templates(&pool).await.expect("templates load");
        let mut mgr = fight_space(&pool, [0.0, 0.0, 0.0]).await;
        set_pools(&mut mgr, PLAYER, 1000, 1000);
        mgr.connect_entity(PLAYER);
        let id = 300_001;
        spawn(&mut mgr, &templates, id, 1380, 2.0, 0.0);
        let _ = mgr.compute_aoi_changes();

        let (tx, mut rx) = mpsc::channel(65_536);
        let engine = ChainEngine::new();
        let events = EngineEvents(&engine);
        for _ in 0..3 {
            npc_ai_tick(&tx, &mut mgr, &events).await;
            if let Some(pc) = mgr.get_entity_mut(id).and_then(|e| e.pending_cast.as_mut()) {
                pc.fire_at = std::time::Instant::now();
            }
            warmup_tick(&tx, &mut mgr, &events).await;
            while rx.try_recv().is_ok() {}
            let npc = mgr.get_entity(id).expect("the Jaffa Guard exists");
            if npc.abilities.is_on_cooldown(710) || npc.abilities.is_on_cooldown(584) {
                break;
            }
        }

        let npc = mgr.get_entity(id).expect("the Jaffa Guard exists");
        assert!(
            npc.abilities.is_on_cooldown(710),
            "the Jaffa Guard at 2 m must swing its staff melee (710)"
        );
        assert!(
            !npc.abilities.is_on_cooldown(584),
            "the ranged auto-attack (584) must not be the one started at 2 m"
        );
    }
}
