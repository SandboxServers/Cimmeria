//! Server-driven auto-cycle (auto-fire) loop tick.
//!
//! Every 100 ms AoI tick, scans armed players (`auto_cycle == true` +
//! `auto_cycle_ability_id` set) and re-invokes
//! [`crate::cell::abilities::handle_use_ability`] against the live
//! [`cimmeria_entity::cell_entity::CellEntity::current_target_id`]. The
//! cooldown gate is the rate limiter; the eligibility filter
//! short-circuits to empty when nobody is auto-cycling.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

/// Drive the server-side auto-cycle (auto-fire) loop.
///
/// For every connected player with `auto_cycle == true` and the loop
/// armed (`auto_cycle_ability_id` set):
///
/// - **Target is LIVE** — re-fires use `CellEntity::current_target_id`
///   (written by `setTargetID`). Mirrors python's
///   `self.entity().targetId` live read. Switching targets via the
///   cursor mid-loop redirects the re-fires automatically;
///   deselecting (target = 0) clears the loop.
/// - **No target / dead target / despawned target** → clear the loop
///   (the death-transition sweep usually catches this first; the tick
///   is the safety net for despawn / instance cleanup paths that
///   bypass the death-transition broadcast). Runs BEFORE the cooldown
///   gate — correctness, not rate limiting.
/// - **Out of range** → skip silently WITHOUT clearing. Leaves the
///   loop armed so a player who strafes in and out of range resumes
///   firing automatically the moment they're back in range. Critical:
///   if we let `handle_use_ability` field the range check, it emits
///   `onErrorCode(OutsideWeaponRange)` every tick (~600ms at default
///   cooldown), flooding the wire. The pre-gate here makes the loop
///   silent while out of range, exactly like cooldown.
/// - **Ability still on cooldown** → skip without clearing. Cooldown
///   clears naturally and the next tick handles re-fire.
/// - **Otherwise** → re-invoke `handle_use_ability` with the
///   loop-armed ability and the live target. Produces the same
///   `onTimerUpdate` + `onSequence` + `onEffectResults` burst as a
///   manual fire — the client cannot distinguish loop-driven from
///   manual shots.
///
/// Cadence: every 100 ms AoI tick.
///
/// `level = "debug"` — fires 10×/sec but the inner `ready` snapshot is
/// often empty (nobody armed). When armed, each fire decision becomes
/// a child event, so SigNoz operators can answer "why didn't auto-fire
/// trigger this tick?" by drilling into the span attributes (cooldown,
/// range, target alive) without grepping log text.
#[tracing::instrument(
    name = "combat.auto_cycle_tick",
    level = "debug",
    skip_all,
    fields(ready_count = tracing::field::Empty),
)]
pub(in crate::cell::service) async fn auto_cycle_tick(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    engine: &ChainEngine,
) {
    // Snapshot armed players. Drop the borrow before re-invoking
    // `handle_use_ability` (takes `&mut space_mgr`).
    let ready: Vec<(u32, i32, i32, bool)> = space_mgr
        .all_player_entity_ids()
        .into_iter()
        .filter_map(|eid| {
            let e = space_mgr.get_entity(eid)?;
            if !e.abilities.auto_cycle {
                return None;
            }
            let ability_id = e.abilities.auto_cycle_ability_id?;
            // LIVE target read — mirrors python `self.entity().targetId`
            // in `abilityCooledDown`. None falls through to `target_id = 0`,
            // which the `!target_alive_or_existed` branch below treats
            // as "clear the loop".
            let target_id = e.current_target_id.unwrap_or(0);
            let target = if target_id > 0 {
                space_mgr.get_entity(target_id as u32)
            } else {
                None
            };
            let target_alive_or_existed = target
                .as_ref()
                .is_some_and(|t| !crate::cell::combat::is_dead_state(t.state_field));

            // Invalid target → push for clearing regardless of cooldown.
            // The clear path is correctness (BSF must un-light on
            // deselect/death/despawn even mid-cooldown); the cooldown
            // gate below is only the re-fire rate limiter. Letting
            // cooldown gate the clear would leave the loop armed for
            // up to a full cooldown AND let a player re-select a
            // different target before cooldown expires and get an
            // unintended re-fire at the new target.
            if !target_alive_or_existed {
                return Some((eid, ability_id, target_id, false));
            }

            // Cooldown gate. Skip without clearing — cooldown clears
            // naturally and the next tick handles re-fire.
            if e.abilities.is_on_cooldown(ability_id) {
                return None;
            }

            // Range pre-gate. Mirrors the rule inside
            // `handle_use_ability` but in skip-don't-error mode.
            // Without this, every out-of-range tick would invoke the
            // handler, fail the range check, and emit
            // `onErrorCode(OutsideWeaponRange)` — at a 0.5s cooldown
            // that's an error packet every ~600 ms while out of
            // range. Loop stays armed so walking back into range
            // resumes firing on the next tick.
            let max_range = space_mgr.ability_defs.get(&ability_id).map_or(30.0, |d| {
                if d.max_range > 0 {
                    d.max_range as f32
                } else {
                    30.0
                }
            });
            if let Some(t) = target {
                if e.position.distance_to(&t.position) > max_range {
                    return None;
                }
            }

            Some((eid, ability_id, target_id, true))
        })
        .collect();

    // Backfill the snapshot size so SigNoz can chart "how many auto-
    // cyclers ran per tick" — the operator-facing answer to "is auto-
    // attack actually firing?" Empty snapshots dominate but cost ~0.
    tracing::Span::current().record("ready_count", ready.len());

    for (entity_id, ability_id, target_id, target_alive) in ready {
        if !target_alive {
            // Target despawned or died without the death sweep
            // catching it. Clear the loop and broadcast so the client
            // un-highlights the button.
            if let Some(new_state) = crate::cell::combat::clear_auto_cycle(space_mgr, entity_id) {
                tracing::info!(
                    entity_id,
                    target_id,
                    "auto_cycle_tick: target gone — clearing loop"
                );
                crate::cell::abilities::send_entity_method(
                    entity_id,
                    crate::mercury::method_idx::ON_STATE_FIELD_UPDATE,
                    new_state.to_le_bytes().to_vec(),
                    tx,
                    space_mgr,
                )
                .await;
            }
            continue;
        }
        tracing::debug!(
            entity_id,
            ability_id,
            target_id,
            "auto_cycle_tick: re-firing"
        );
        // Route loop-driven re-fires through the kill-credit wrapper
        // so killing a quest-tagged NPC via auto-shoot advances the
        // mission's KillCount objective, matching the manual right-
        // click path. The wrapper is a no-op when the target wasn't a
        // live tagged NPC or when no kill happened this tick.
        let _ = crate::cell::abilities::handle_use_ability_with_kill_credit(
            entity_id, ability_id, target_id, engine, tx, space_mgr,
        )
        .await;
        // Commit/reject is the handler's call. A rejected re-fire
        // (e.g. out of ammo) leaves the loop armed — next tick
        // re-evaluates. A committed re-fire restarts the cooldown so
        // this same player won't fire again until that elapses.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::abilities::AbilityDef;

    /// Empty chain engine for tests that don't exercise content chains.
    /// The kill-credit hook is a no-op when the resolved-action list is
    /// empty, so these tests don't need real chain content loaded.
    fn empty_engine() -> ChainEngine {
        ChainEngine::new()
    }

    /// Shared fixture: one connected armed player and one target NPC,
    /// both in the same Castle space. Ability 7 is a 30-unit ranged
    /// ability with no ammo requirement — focuses the tests on loop
    /// semantics, not ammo.
    fn make_auto_cycle_mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
        mgr.parse_spaces_xml(xml).unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
        mgr.spawn_npc(50, "Castle", [5.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.is_player = true;
            p.player_id = Some(100);
            p.abilities.add_ability(7);
            p.abilities.auto_cycle = true;
            p.abilities.auto_cycle_ability_id = Some(7);
            // Phase 2: the tick reads `current_target_id` (live) as
            // the re-fire target — the cursor selection from
            // `setTargetID`.
            p.current_target_id = Some(50);
            p.weapon_holstered = false;
        }
        mgr.connect_entity(1);
        let _ = mgr.compute_aoi_changes();
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
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: None,
                velocity: 0.0,
            },
        );
        mgr
    }

    /// With the cooldown clear and a live target, the tick re-fires
    /// the stashed ability via `handle_use_ability` — which starts a
    /// fresh cooldown. Pin: dropping the re-fire branch silently
    /// breaks the loop.
    #[tokio::test]
    async fn auto_cycle_tick_refires_when_cooldown_clear() {
        let mut mgr = make_auto_cycle_mgr();
        assert!(!mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7));

        let (tx, _rx) = mpsc::channel(64);
        auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

        assert!(
            mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7),
            "tick must re-invoke handle_use_ability when cooldown is clear"
        );
    }

    /// With the cooldown in flight, the tick must NOT re-fire. The
    /// cooldown gate is the rate limiter — without it the tick would
    /// fire every 100 ms regardless of ability cooldown, turning a
    /// 2-second-cooldown weapon into a 10-Hz autocannon.
    #[tokio::test]
    async fn auto_cycle_tick_skips_when_on_cooldown() {
        let mut mgr = make_auto_cycle_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities
                .start_ability_cooldown(7, std::time::Duration::from_secs(60));
        }

        let (tx, mut rx) = mpsc::channel(64);
        auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

        assert!(
            rx.try_recv().is_err(),
            "tick must not emit anything when stashed ability is on cooldown"
        );
    }

    /// When the stashed target has died, the tick clears the loop and
    /// broadcasts onStateFieldUpdate so the client un-highlights the
    /// button. Secondary safety net — the primary stop is the
    /// death-transition sweep — for targets that died without going
    /// through `apply_death_transition` (despawned by space cleanup).
    #[tokio::test]
    async fn auto_cycle_tick_clears_loop_when_target_dead() {
        use crate::cell::combat::{BSF_AUTO_CYCLING, BSF_DEAD};
        let mut mgr = make_auto_cycle_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.set_state_flag(BSF_AUTO_CYCLING);
        }
        if let Some(t) = mgr.get_entity_mut(50) {
            t.set_state_flag(BSF_DEAD);
        }

        let (tx, mut rx) = mpsc::channel(64);
        auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

        let p = mgr.get_entity(1).unwrap();
        assert!(!p.abilities.auto_cycle, "loop must be cleared");
        assert_eq!(
            p.state_field & BSF_AUTO_CYCLING,
            0,
            "BSF_AUTO_CYCLING must be cleared so the client un-highlights the button",
        );

        let mut saw_broadcast = false;
        while let Ok(msg) = rx.try_recv() {
            if let CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                ..
            } = msg
            {
                if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                    saw_broadcast = true;
                }
            }
        }
        assert!(
            saw_broadcast,
            "tick must broadcast onStateFieldUpdate when it clears the loop"
        );
    }

    /// When the live target despawned entirely (no longer in the
    /// space manager), the tick treats it as gone and clears the
    /// loop. Same defensive sweep as the dead-target branch.
    #[tokio::test]
    async fn auto_cycle_tick_clears_loop_when_target_missing() {
        use crate::cell::combat::BSF_AUTO_CYCLING;
        let mut mgr = make_auto_cycle_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.set_state_flag(BSF_AUTO_CYCLING);
            p.current_target_id = Some(99999);
        }

        let (tx, _rx) = mpsc::channel(64);
        auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

        let p = mgr.get_entity(1).unwrap();
        assert!(!p.abilities.auto_cycle);
        assert_eq!(p.state_field & BSF_AUTO_CYCLING, 0);
    }

    /// Phase 2 live-target switch: player armed loop at NPC 50 then
    /// switched cursor to NPC 75 mid-loop. The tick must re-fire at
    /// 75, not 50.
    ///
    /// Verification: after the tick, NPC 75 (LIVE target) should have
    /// player 1 on its `threat_list` from the `generate_threat` call
    /// inside `damage_apply`. NPC 50 (original) should NOT.
    #[tokio::test]
    async fn auto_cycle_tick_refires_at_live_current_target() {
        use cimmeria_common::Vector3;
        let mut mgr = make_auto_cycle_mgr();
        mgr.spawn_npc(75, "Castle", [3.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.current_target_id = Some(75);
            p.position = Vector3::new(0.0, 0.0, 0.0);
        }
        let (tx, _rx) = mpsc::channel(64);
        auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

        let npc_b = mgr.get_entity(75).expect("NPC 75 should still exist");
        assert!(
            npc_b.threat_list.contains_key(&1),
            "tick must re-fire at LIVE current_target (75) — threat_list should contain player 1"
        );

        let npc_a = mgr.get_entity(50).expect("NPC 50 should still exist");
        assert!(
            !npc_a.threat_list.contains_key(&1),
            "tick must NOT re-fire at the original target (50) once the player has switched cursor"
        );
    }

    /// Regression: target dies/deselects DURING the cooldown window
    /// must still clear the loop. The cooldown gate is a re-fire
    /// rate limiter, NOT a hold against the clear path.
    #[tokio::test]
    async fn auto_cycle_tick_clears_dead_target_even_while_on_cooldown() {
        use crate::cell::combat::{BSF_AUTO_CYCLING, BSF_DEAD};
        let mut mgr = make_auto_cycle_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.set_state_flag(BSF_AUTO_CYCLING);
            p.abilities
                .start_ability_cooldown(7, std::time::Duration::from_secs(60));
        }
        if let Some(t) = mgr.get_entity_mut(50) {
            t.set_state_flag(BSF_DEAD);
        }

        let (tx, _rx) = mpsc::channel(64);
        auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

        let p = mgr.get_entity(1).unwrap();
        assert!(
            !p.abilities.auto_cycle,
            "dead target must clear the loop even while ability is on cooldown",
        );
        assert_eq!(p.state_field & BSF_AUTO_CYCLING, 0);
    }

    /// Target deselect (`current_target_id == None`) clears the loop.
    /// Player pressed escape / right-clicked empty space.
    #[tokio::test]
    async fn auto_cycle_tick_clears_loop_when_target_deselected() {
        use crate::cell::combat::BSF_AUTO_CYCLING;
        let mut mgr = make_auto_cycle_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.set_state_flag(BSF_AUTO_CYCLING);
            p.current_target_id = None;
        }

        let (tx, _rx) = mpsc::channel(64);
        auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

        let p = mgr.get_entity(1).unwrap();
        assert!(
            !p.abilities.auto_cycle,
            "target deselect must clear the auto-cycle loop"
        );
        assert_eq!(p.state_field & BSF_AUTO_CYCLING, 0);
    }

    /// Tick is a no-op for players whose `auto_cycle == false`, even
    /// when stale stash data lingers. The eligibility filter must
    /// gate on `auto_cycle`, not on the presence of a stashed
    /// ability id.
    #[tokio::test]
    async fn auto_cycle_tick_skips_when_flag_not_armed() {
        let mut mgr = make_auto_cycle_mgr();
        if let Some(p) = mgr.get_entity_mut(1) {
            p.abilities.auto_cycle = false;
        }

        let (tx, mut rx) = mpsc::channel(64);
        auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

        assert!(!mgr.get_entity(1).unwrap().abilities.is_on_cooldown(7));
        assert!(rx.try_recv().is_err());
    }

    /// Regression: out-of-range targets must be skipped silently — no
    /// `handle_use_ability` invocation, no `onErrorCode` packet, no
    /// cooldown started. The loop stays armed so walking back into
    /// range resumes firing on the next tick.
    ///
    /// Bug shape: without the pre-gate, every out-of-range tick
    /// invokes `handle_use_ability` → fails range → emits
    /// `onErrorCode(OutsideWeaponRange)`. At a 0.5s cooldown that's
    /// one error packet every ~600 ms while out of range — flooding
    /// the wire and the client's error UI.
    #[tokio::test]
    async fn auto_cycle_tick_skips_out_of_range_silently() {
        use cimmeria_common::Vector3;
        let mut mgr = make_auto_cycle_mgr();
        // Move the target far beyond ability 7's 30-unit max_range
        // (without despawning it, so the "missing target" branch
        // doesn't trip).
        if let Some(t) = mgr.get_entity_mut(50) {
            t.position = Vector3::new(500.0, 0.0, 0.0);
        }

        let (tx, mut rx) = mpsc::channel(64);
        auto_cycle_tick(&tx, &mut mgr, &empty_engine()).await;

        let p = mgr.get_entity(1).unwrap();
        assert!(
            p.abilities.auto_cycle,
            "out-of-range must NOT clear the loop — player may walk back into range",
        );
        assert!(
            !p.abilities.is_on_cooldown(7),
            "out-of-range must not commit the fire (no cooldown started)",
        );
        assert!(
            rx.try_recv().is_err(),
            "out-of-range tick must be wire-silent — no onErrorCode, no onTimerUpdate",
        );
    }

    /// **Regression guard: auto-cycle kills must credit quest
    /// objectives.** Auto-cycle kills against a quest-tagged NPC must
    /// fire the content-engine `EntityDeath` event so KillCount-style
    /// mission objectives advance, exactly the way a manual right-
    /// click kill does.
    ///
    /// Bug shape this catches: the tick used to call bare
    /// `handle_use_ability`, bypassing the kill-credit wrapper that
    /// the cell-method `USE_ABILITY` dispatch site wired around manual
    /// fires. Reverting the tick to `handle_use_ability` (instead of
    /// `handle_use_ability_with_kill_credit`) leaves the counter at
    /// `None` and fails this assertion.
    ///
    /// Pinned via an end-to-end signal: register a chain that
    /// increments a counter on `EntityDeath(entity_tag=QuestTargetDrone)`
    /// and assert the counter incremented on the killer entity after
    /// the tick fires. This proves the full path — tick →
    /// `handle_use_ability_with_kill_credit` →
    /// `apply_damage_to_target` (kill detected) →
    /// `fire_entity_death` → chain engine resolve + execute.
    #[tokio::test]
    async fn auto_cycle_tick_credits_quest_kill_on_tagged_npc_death() {
        use cimmeria_content_engine::actions::Action;
        use cimmeria_content_engine::chain::Chain;
        use cimmeria_content_engine::triggers::Trigger;
        use cimmeria_entity::abilities::EffectDef;
        use cimmeria_entity::stats::HEALTH;

        const QUEST_TAG: &str = "QuestTargetDrone";
        const COUNTER_NAME: &str = "drone_kills";

        let mut mgr = make_auto_cycle_mgr();

        // Give the ability a lethal effect so the tick's re-fire
        // actually kills the NPC. `apply_damage_to_target` reads
        // `HealthDamage` from the ability's effect NVPs; 9999 mirrors
        // the lethal-fixture pattern in damage_apply/tests.rs.
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
        if let Some(ability) = mgr.ability_defs.get_mut(&7) {
            ability.effect_ids = vec![100];
        }

        // Tag the NPC and seed it at 1 HP so the first re-fire kills
        // it. Without the tag, `fire_entity_death` has no `entity_tag`
        // to match against the chain trigger — the assertion would
        // pass even with the bug present.
        if let Some(npc) = mgr.get_entity_mut(50) {
            npc.tag = Some(QUEST_TAG.to_string());
            if let Some(stat) = npc.stats.get_mut(HEALTH) {
                stat.update(0, 1, 100);
                stat.clear_dirty();
            }
        }

        // Register a minimal chain that increments a counter on the
        // tagged death. The chain's resolved actions feed directly to
        // the executor (no DB lookup); the counter lands on the
        // killer entity's `counters` map per the `IncrementCounter`
        // handler at executor/counter.rs.
        let mut engine = ChainEngine::new();
        engine.register_chain(Chain {
            id: 999_999,
            name: "test: drone kill counter".to_string(),
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

        // Sanity: counter unset before the kill so the post-tick
        // assertion is genuinely measuring the increment, not a stale
        // value.
        assert!(
            !mgr.get_entity(1)
                .unwrap()
                .counters
                .contains_key(COUNTER_NAME),
            "test invariant: counter must start unset",
        );

        let (tx, _rx) = mpsc::channel(64);
        auto_cycle_tick(&tx, &mut mgr, &engine).await;

        // Primary assertion: the kill credit landed. Reverting the
        // tick to call `handle_use_ability` directly (the pre-fix
        // shape) leaves this at None — the chain never resolves
        // because `fire_entity_death` is never called.
        assert_eq!(
            mgr.get_entity(1).unwrap().counters.get(COUNTER_NAME),
            Some(&1),
            "auto-cycle kill of a quest-tagged NPC must increment the kill counter \
             via fire_entity_death — bypassing the kill-credit wrapper \
             (calling bare handle_use_ability from the tick) leaves this counter at None",
        );

        // Sanity: the NPC actually died (proves we exercised the kill
        // path, not a chain that fires on non-lethal hits). The
        // counter assertion above only fires if both (a) the kill
        // happened AND (b) fire_entity_death ran; this second check
        // disambiguates a future regression where the kill path
        // broke but the counter assertion happened to pass via some
        // other code path.
        let target = mgr.get_entity(50).unwrap();
        assert!(
            crate::cell::combat::is_dead_state(target.state_field),
            "test fixture: target must be dead after the lethal re-fire",
        );
    }
}
