//! Holster-deferred phase-A → phase-B promotion ticks.
//!
//! Both ticks promote an action that `handle_use_ability` / `handle_reload`
//! deferred while the player's weapon was holstered: the original call drew
//! the weapon (firing `Item_Equip`), stamped a `pending_*_at` deadline, and
//! returned without committing. Once `UNHOLSTER_DRAW_DURATION` has elapsed
//! these ticks re-invoke the handler against the now-drawn weapon.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::super::super::messages::CellToBaseMsg;
use super::super::super::space_manager::SpaceManager;

/// Promote queued attack-while-holstered: dispatch the deferred
/// ability after the draw animation has had time to play.
///
/// `handle_use_ability` detects "player is holstered + OOC + attempting
/// to fire," draws the weapon + fires `Item_Equip`, stashes the
/// ability/target on the entity, and returns false WITHOUT committing
/// cooldown or ammo. This tick re-invokes `handle_use_ability` once
/// `UNHOLSTER_DRAW_DURATION` has elapsed — Phase B runs the normal
/// fire path against an already-drawn weapon.
///
/// Cadence: every 100ms AoI tick. Cost is one filter pass; the inner
/// re-invocation only fires on transition.
pub(in crate::cell::service) async fn pending_attack_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    let now = std::time::Instant::now();
    // Snapshot the queue entries first — `handle_use_ability` takes
    // `&mut space_mgr` and we don't want to hold a `&` across the
    // re-invocation.
    let ready: Vec<(u32, i32, i32)> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter_map(|eid| {
            let e = space_mgr.get_entity(eid)?;
            let at = e.pending_attack_at?;
            if now < at {
                return None;
            }
            let ability = e.pending_attack_ability_id?;
            let target = e.pending_attack_target_id?;
            Some((eid, ability, target))
        })
        .collect();

    for (entity_id, ability_id, target_id) in ready {
        // Clear the queue BEFORE re-invoking so the early-return
        // guard in handle_use_ability (which rejects on
        // `pending_attack_at.is_some()`) lets Phase B through.
        if let Some(e) = space_mgr.get_entity_mut(entity_id) {
            e.pending_attack_at = None;
            e.pending_attack_ability_id = None;
            e.pending_attack_target_id = None;
        }
        tracing::info!(
            entity_id,
            ability_id,
            target_id,
            "pending_attack_tick: draw window elapsed, firing queued attack"
        );
        // Route Phase-B queued attacks through the kill-credit
        // wrapper. Without this, a player who fires a weapon while
        // holstered (the attack is deferred to Phase B until the draw
        // window elapses) doesn't credit a quest objective if that
        // queued shot makes the kill — same divergence the auto-cycle
        // tick had until both paths joined the canonical helper.
        let _ = crate::cell::abilities::handle_use_ability_with_kill_credit(
            entity_id, ability_id, target_id, engine, tx, space_mgr,
        )
        .await;
    }
}

/// Promote pending reload-while-holstered phase A → phase B.
///
/// `handle_reload` detects "player is holstered + OOC + no reload in
/// flight," dispatches an `Item_Equip` draw animation + appearance
/// refresh, and stamps `pending_reload_at = now + UNHOLSTER_DRAW_DURATION`.
/// This tick scans for elapsed stamps and re-invokes `handle_reload`,
/// which then finds the weapon already drawn and runs the normal reload
/// start (cooldown timer + `Item_Reload` sequence + deferred ammo refill
/// via [`super::reload_completion_tick`]).
///
/// Why two phases: firing the reload animation on a model that's still
/// in the middle of the draw motion produces "weapon teleports into
/// hand and the reload anim plays on empty space" — the symptom that
/// drove this fix. Giving the draw `UNHOLSTER_DRAW_DURATION` to play
/// out lets the hand reach the hold position before the reload
/// sequence triggers.
///
/// Cadence: every 100ms AoI tick. Cost is one filter pass; the inner
/// `handle_reload` re-invocation only fires on transition.
#[tracing::instrument(
    name = "combat.pending_reload_tick",
    level = "debug",
    skip_all,
    fields(ready_count = tracing::field::Empty),
)]
pub(in crate::cell::service) async fn pending_reload_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
) {
    let now = std::time::Instant::now();

    let ready: Vec<u32> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter(|&eid| {
            space_mgr
                .get_entity(eid)
                .and_then(|e| e.pending_reload_at)
                .is_some_and(|t| now >= t)
        })
        .collect();
    tracing::Span::current().record("ready_count", ready.len());

    for entity_id in ready {
        tracing::info!(
            entity_id,
            "pending_reload_tick: draw window elapsed, starting deferred reload"
        );
        // `handle_reload` clears `pending_reload_at` at the top of its
        // Phase B branch, so this won't re-fire next tick.
        crate::cell::cell_methods::player::world::handle_reload(entity_id, tx, space_mgr).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::space_manager::SpaceManager;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn pending_attack_tick_clears_queue_when_elapsed() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.pending_attack_at =
                Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
            e.pending_attack_ability_id = Some(42);
            e.pending_attack_target_id = Some(200);
        }
        mgr.connect_entity(1);

        let (tx, _rx) = mpsc::channel(8);
        pending_attack_tick(&tx, &mut mgr, &ChainEngine::new()).await;

        let entity = mgr.get_entity(1).unwrap();
        assert!(
            entity.pending_attack_at.is_none(),
            "pending_attack_at must be cleared after tick fires"
        );
        assert!(
            entity.pending_attack_ability_id.is_none(),
            "pending_attack_ability_id must be cleared"
        );
        assert!(
            entity.pending_attack_target_id.is_none(),
            "pending_attack_target_id must be cleared"
        );
    }

    #[tokio::test]
    async fn pending_attack_tick_no_op_when_stamp_in_future() {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        if let Some(e) = mgr.get_entity_mut(1) {
            e.is_player = true;
            e.player_id = Some(100);
            e.pending_attack_at =
                Some(std::time::Instant::now() + std::time::Duration::from_secs(60));
            e.pending_attack_ability_id = Some(42);
            e.pending_attack_target_id = Some(200);
        }
        mgr.connect_entity(1);

        let (tx, _rx) = mpsc::channel(8);
        pending_attack_tick(&tx, &mut mgr, &ChainEngine::new()).await;

        let entity = mgr.get_entity(1).unwrap();
        assert!(
            entity.pending_attack_at.is_some(),
            "future stamp must not be consumed"
        );
        assert_eq!(entity.pending_attack_ability_id, Some(42));
        assert_eq!(entity.pending_attack_target_id, Some(200));
    }

    /// **Regression guard: `pending_attack_tick` Phase B fires must
    /// credit quest kills.** A player who fires while holstered has
    /// the attack deferred to Phase B (after the draw window
    /// elapses). When Phase B kills a quest-tagged NPC, the
    /// EntityDeath content event must reach the chain engine.
    ///
    /// Parallel coverage to
    /// `auto_cycle_tick_credits_quest_kill_on_tagged_npc_death` (in
    /// `service/ticks/auto_cycle.rs`) and
    /// `set_auto_cycle_immediate_fire_credits_quest_kill_on_tagged_npc_death`
    /// (in `cell_methods/player/world/tests.rs`). All three player-
    /// driven re-fire paths route through
    /// `handle_use_ability_with_kill_credit`; all three tests fail
    /// when the caller is reverted to bare `handle_use_ability`.
    #[tokio::test]
    async fn pending_attack_tick_credits_quest_kill_on_tagged_npc_death() {
        use cimmeria_content_engine::actions::Action;
        use cimmeria_content_engine::chain::Chain;
        use cimmeria_content_engine::triggers::Trigger;
        use cimmeria_entity::abilities::{AbilityDef, EffectDef};
        use cimmeria_entity::stats::HEALTH;

        const QUEST_TAG: &str = "QuestTargetDrone";
        const COUNTER_NAME: &str = "drone_kills";

        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        mgr.spawn_npc(50, "Castle", [3.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        // Hostile so the #444 single-target target-validity gate allows
        // the deferred attack through to the kill path this test drives.
        if let Some(npc) = mgr.get_entity_mut(50) {
            npc.faction = crate::cell::combat::HOSTILE_FACTION;
        }
        // Phase-B queue pre-conditions: stamp is in the past so the
        // tick promotes it, and the queue carries the target+ability
        // the deferred fire should pick up.
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
            p.abilities.add_ability(7);
            p.weapon_holstered = false;
            p.pending_attack_at =
                Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
            p.pending_attack_ability_id = Some(7);
            p.pending_attack_target_id = Some(50);
        }
        mgr.connect_entity(1);
        let _ = mgr.compute_aoi_changes();

        // Lethal effect + ability def so the deferred fire kills the
        // NPC outright. Same fixture shape as the auto_cycle test.
        let mut params = std::collections::HashMap::new();
        params.insert("HealthDamage".to_string(), "9999".to_string());
        mgr.effect_defs.insert(
            100,
            EffectDef {
                effect_id: 100,
                ability_id: 7,
                delay: 0,
                effect_sequence: 0,
                event_set_id: None,
                script_name: None,
                params,
                ..Default::default()
            },
        );
        mgr.ability_defs.insert(
            7,
            AbilityDef {
                ability_id: 7,
                name: "test".to_string(),
                cooldown: 0.5,
                warmup: 0.0,
                flags: 0,
                is_ranged: false,
                min_range: 0,
                max_range: 30,
                target_type_id: 0,
                effect_ids: vec![100],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
        if let Some(npc) = mgr.get_entity_mut(50) {
            npc.tag = Some(QUEST_TAG.to_string());
            if let Some(stat) = npc.stats.get_mut(HEALTH) {
                stat.update(0, 1, 100);
                stat.clear_dirty();
            }
        }

        let mut engine = ChainEngine::new();
        engine.register_chain(Chain {
            id: 999_997,
            name: "test: pending_attack drone kill counter".to_string(),
            enabled: true,
            trigger: Trigger::OnEntityDeath {
                entity_type: None,
                entity_tag: Some(QUEST_TAG.to_string()),
            },
            conditions: vec![],
            actions: vec![Action::IncrementCounter {
                counter_name: COUNTER_NAME.to_string(),
                amount: 1,
            }],
            priority: 0,
        });

        let (tx, _rx) = mpsc::channel(64);
        pending_attack_tick(&tx, &mut mgr, &engine).await;

        assert_eq!(
            mgr.get_entity(1).unwrap().counters.get(COUNTER_NAME),
            Some(&1),
            "pending_attack_tick's Phase-B fire must credit a tagged-NPC kill \
             via fire_entity_death — reverting the tick to bare \
             handle_use_ability leaves this counter at None",
        );
        assert!(
            crate::cell::combat::is_dead_state(mgr.get_entity(50).unwrap().state_field),
            "test fixture: target must be dead after the lethal deferred fire",
        );
    }
}
