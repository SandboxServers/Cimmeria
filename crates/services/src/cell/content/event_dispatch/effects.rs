//! Effect-lifecycle event dispatchers: currently `OnEffectInit`.
//!
//! `OnEffectInit` fires when a pulsing effect is (re)registered on an
//! entity through the content-engine `ApplyEffect` / `LaunchAbility`
//! entry points in [`crate::cell::content::effect_apply`]. The ability-
//! driven combat path (`use_ability` → `damage_apply`) does not carry a
//! `ChainEngine` handle today, so it registers effects without firing the
//! trigger — see the follow-up note in `docs/content/content-engine.md`.
//!
//! `effect_id` travels in `TriggerEvent.params` (the established
//! `ResolvedActions.params` convention, same as `item_use`'s
//! `instance_id`). The trigger matcher is a pure unit match and the
//! loader ignores `event_key` for effect triggers, so no filtering
//! happens here — params carry the id as *data* (the seeded
//! `effects_chains.sql` rows are NULL-keyed and match every effect).
//!
//! Re-entrancy: a chain fired from here can `ApplyEffect` another
//! effect, which registers and fires `OnEffectInit` again. The depth
//! guard on [`SpaceManager::effect_init_depth`] bounds the cycle.

use tokio::sync::mpsc;

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_content_engine::context::ExecutionContext;
use cimmeria_content_engine::triggers::{TriggerEvent, TriggerType};

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

use super::super::executor;
use super::super::mission_context::populate_world_context;

/// Bound on the chain → `ApplyEffect` → register → `OnEffectInit`
/// recursion. Mirrors the step-activation replay guard's `MAX_REPLAY_DEPTH`.
const MAX_EFFECT_INIT_DEPTH: u32 = 16;

/// Fire `OnEffectInit` for a just-registered pulsing effect.
///
/// `entity_id` is the effect's target entity (the chain's source when
/// the target is a player). `effect_id` is the registered effect's id.
///
/// Boxed for the same reason as `fire_step_activation_regions`: the
/// resolved actions run through `executor::execute_actions`, whose
/// `ApplyEffect` arm can call back into this function, and an `async fn`
/// would compute an infinitely sized future.
pub fn fire_effect_init<'a>(
    entity_id: u32,
    effect_id: i32,
    engine: &'a ChainEngine,
    tx: &'a mpsc::Sender<CellToBaseMsg>,
    space_mgr: &'a mut SpaceManager,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        // The enter/exit pair brackets exactly one execution: every early
        // return sits before the depth increment, so the depth can never
        // be left unbalanced.
        if space_mgr.effect_init_depth >= MAX_EFFECT_INIT_DEPTH {
            tracing::warn!(
                entity_id,
                effect_id,
                max_depth = MAX_EFFECT_INIT_DEPTH,
                reason = "effect_init_depth_exceeded",
                "fire_effect_init refused — a chain applied an effect whose \
                 effect_init fired back into another chain; the chain is looping"
            );
            return;
        }
        space_mgr.effect_init_depth += 1;

        let mut ctx =
            ExecutionContext::new().with_source(cimmeria_common::EntityId(entity_id as i32));
        // Per the `fire_item_use` convention, the id that identifies the
        // thing that happened rides in params as data; the matcher does
        // not filter on it (yet).
        ctx.set_param("effect_id".to_string(), serde_json::json!(effect_id));
        populate_world_context(entity_id, space_mgr, &mut ctx);

        let event = TriggerEvent {
            trigger_type: TriggerType::EffectInit,
            source_entity: Some(cimmeria_common::EntityId(entity_id as i32)),
            target_entity: None,
            params: ctx.params.clone(),
        };

        let resolved = engine.resolve_event(&event, &ctx);
        if resolved.actions.is_empty() {
            tracing::debug!(entity_id, effect_id, "fire_effect_init: no chains matched");
        } else {
            tracing::info!(
                entity_id,
                effect_id,
                actions = resolved.actions.len(),
                "fire_effect_init: matched"
            );
        }

        // `player_id` for the executor: the effect's target is the chain
        // source — for the seeded content effects that is the player who
        // carries the debuff/buff.
        let player_id = space_mgr
            .get_entity(entity_id)
            .and_then(|e| e.player_id)
            .unwrap_or(-1);
        executor::execute_actions(resolved, entity_id, player_id, tx, space_mgr, engine).await;

        space_mgr.effect_init_depth -= 1;
    })
}

#[cfg(test)]
mod tests {
    use super::super::super::effect_apply::apply_effect;
    use super::super::super::tests::{make_test_effect_mgr, make_test_space_mgr};
    use super::*;
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;

    fn register_init_counter_chain(engine: &mut ChainEngine, chain_id: i64, counter: &str) {
        engine.register_chain(Chain {
            action_delays: Vec::new(),
            id: chain_id,
            name: format!("test: effect_init → bump {counter}"),
            enabled: true,
            trigger: Trigger::OnEffectInit,
            conditions: vec![],
            actions: vec![Action::IncrementCounter {
                counter_name: counter.to_string(),
                amount: 1,
            }],
            priority: 0,
        });
    }

    /// The guard that fails when the dispatch is removed: applying a
    /// pulsing effect through the content `ApplyEffect` body must resolve
    /// and execute an `OnEffectInit` chain on the target.
    #[tokio::test]
    async fn apply_effect_fires_effect_init_chain() {
        let mut mgr = make_test_effect_mgr();
        let mut engine = ChainEngine::new();
        register_init_counter_chain(&mut engine, 6101, "first_init");
        let (tx, _rx) = mpsc::channel(16);

        // Target a player entity so registration succeeds.
        let target = 1;
        let matched = apply_effect(700, target, target, 999, &tx, &mut mgr, &engine).await;

        assert!(matched, "pulsing effect must register");
        let entity = mgr.get_entity(target).expect("target must exist");
        assert_eq!(
            entity.counters.get("first_init"),
            Some(&1),
            "OnEffectInit chain must execute its IncrementCounter once",
        );
        assert_eq!(mgr.effect_init_depth, 0, "depth guard must unwind");
    }

    /// A regression guard for the depth guard itself: a chain that
    /// applies the very effect whose init fired must not loop forever —
    /// the recursion stops at `MAX_EFFECT_INIT_DEPTH` and the depth
    /// unwinds to zero.
    #[tokio::test]
    async fn effect_init_recursion_is_bounded_and_unwinds() {
        let mut mgr = make_test_effect_mgr();
        let mut engine = ChainEngine::new();
        // Chain A re-applies effect 700 (which fires init again).
        engine.register_chain(Chain {
            action_delays: Vec::new(),
            id: 6100,
            name: "test: effect_init re-applies the effect".to_string(),
            enabled: true,
            trigger: Trigger::OnEffectInit,
            conditions: vec![],
            actions: vec![Action::ApplyEffect {
                effect_id: 700,
                duration_secs: None,
            }],
            priority: 10,
        });
        // Chain B counts every init that got through the guard.
        register_init_counter_chain(&mut engine, 6101, "inits");

        let target = 1;
        // 64 capacity: every nested registration's buff-icon send is
        // buffered (17 sends in the worst case: the outer + one per depth
        // level) without blocking on a receiver that is never drained.
        let (tx, _rx) = mpsc::channel(64);
        apply_effect(700, target, target, 999, &tx, &mut mgr, &engine).await;

        let entity = mgr.get_entity(target).expect("target must exist");
        // Every pass through the guard increments; the first fire is depth
        // 0→1, the last depth 15→16, and the depth-16 entry is refused.
        assert_eq!(
            entity.counters.get("inits"),
            Some(&(super::MAX_EFFECT_INIT_DEPTH as i32)),
            "recursion must stop at the depth bound, not loop forever",
        );
        assert_eq!(mgr.effect_init_depth, 0, "guard must unwind to zero");
    }

    /// Empty engine: applying a pulsing effect must not panic and must
    /// not fire anything beyond the registration's own buff-icon send when
    /// no `OnEffectInit` chain is registered.
    #[tokio::test]
    async fn apply_effect_with_engine_missing_chain_is_quiet() {
        let mut mgr = make_test_effect_mgr();
        let engine = ChainEngine::new();
        let (tx, mut rx) = mpsc::channel(16);

        let target = 1;
        let matched = apply_effect(700, target, target, 999, &tx, &mut mgr, &engine).await;

        assert!(matched, "registration itself must still succeed");
        // `register_active_effect` emits its own onTimerUpdate buff icon, so
        // exactly one message is expected; the fired event must add nothing.
        assert!(
            rx.try_recv().is_ok(),
            "registration emits the buff-icon onTimerUpdate",
        );
        assert!(
            rx.try_recv().is_err(),
            "no OnEffectInit chain → fired event adds no further wire output",
        );
        assert_eq!(mgr.effect_init_depth, 0);
    }

    // Sanity for the fixture the tests above rely on: make_test_effect_mgr
    // must expose a player target and a registered pulsing effect def.
    #[tokio::test]
    async fn fixture_has_player_and_pulsing_effect() {
        let mgr = make_test_effect_mgr();
        let target = 1;
        assert!(mgr.get_entity(target).is_some_and(|e| e.is_player));
        let def = mgr.effect_defs.get(&700).expect("effect 700 seeded");
        assert!(def.is_pulsing());
        // make_test_space_mgr still exists as the plain-space builder.
        let plain = make_test_space_mgr();
        let _ = plain;
    }
}
