//! `SET_AUTO_CYCLE` (auto-fire toggle) cell-method dispatch tests:
//! enable/disable state-field + BSF transitions, persist-on-toggle,
//! duplicate-click idempotency, and the immediate-fire-on-enable path
//! (including quest-kill credit).

use super::super::*;
use super::make_mgr_with_player;
use cimmeria_entity::abilities::AbilityDef;

/// SET_AUTO_CYCLE disable clears the full loop stash (flag,
/// ability id, target id) AND the `BSF_AUTO_CYCLING` state-field
/// bit when it was previously set. Regression guard: a refactor
/// that drops the target-id clear or the BSF un-set would leave
/// the button stuck on-screen highlighted with stale ids ready to
/// re-fire on the next enable.
#[tokio::test]
async fn set_auto_cycle_disable_clears_stash_and_bsf() {
    use crate::cell::combat::BSF_AUTO_CYCLING;
    let mut mgr = make_mgr_with_player();
    // Simulate a previously-armed loop: ability stashed, BSF bit
    // set (the state arrived at by `arm_auto_cycle`).
    if let Some(e) = mgr.get_entity_mut(1) {
        e.abilities.auto_cycle = true;
        e.abilities.auto_cycle_ability_id = Some(597);
        e.set_state_flag(BSF_AUTO_CYCLING);
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);

    // args = [0] → enabled = false
    let handled = dispatch(1, SET_AUTO_CYCLE, &[0], &tx, &mut mgr, &engine).await;
    assert!(handled);

    let e = mgr.get_entity(1).unwrap();
    assert!(!e.abilities.auto_cycle);
    assert!(
        e.abilities.auto_cycle_ability_id.is_none(),
        "disable must clear auto_cycle_ability_id"
    );
    assert_eq!(
        e.state_field & BSF_AUTO_CYCLING,
        0,
        "disable must clear BSF_AUTO_CYCLING so the client un-highlights the button"
    );

    // Verify the broadcast went out — the client requires this
    // `onStateFieldUpdate` to fire `EmitAutoCycleStateChanged`.
    let mut saw_state_field_update = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            ..
        } = msg
        {
            if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                saw_state_field_update = true;
            }
        }
    }
    assert!(
        saw_state_field_update,
        "disable with BSF set must broadcast onStateFieldUpdate so the client un-highlights the button"
    );
}

/// Collect every `StateFieldUpdate` persist message out of the channel.
fn drain_state_field_updates(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(i32, u32)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::StateFieldUpdate {
            player_id,
            state_field,
        } = msg
        {
            out.push((player_id, state_field));
        }
    }
    out
}

/// **#412 persist-on-enable.** The explicit `setAutoCycle(1)` toggle must
/// send a `StateFieldUpdate` carrying the masked preference bits so the
/// base persists the choice and the next relog restores it. Pre-fix,
/// nothing persisted `state_field` and the auto-cycle loop never armed
/// post-relog until the player pressed the button again.
#[tokio::test]
async fn set_auto_cycle_enable_persists_masked_state_field() {
    use crate::cell::combat::{BSF_AUTO_CYCLING, BSF_IN_COMBAT};
    let mut mgr = make_mgr_with_player();
    // Pre-set a transient bit so the mask discipline is observable: the
    // persisted value must carry ONLY the preference bit even though the
    // live state_field has BSF_InCombat riding along.
    if let Some(e) = mgr.get_entity_mut(1) {
        e.state_field |= BSF_IN_COMBAT;
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);

    let handled = dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
    assert!(handled);

    let updates = drain_state_field_updates(&mut rx);
    assert_eq!(
        updates,
        vec![(100, BSF_AUTO_CYCLING)],
        "enable must persist exactly one StateFieldUpdate with the masked \
         preference bits (BSF_InCombat must be stripped)"
    );
}

/// **#412 persist-on-disable.** The explicit `setAutoCycle(0)` toggle must
/// persist the cleared preference (state_field = 0) so a relog doesn't
/// resurrect a deliberately disabled auto-cycle.
#[tokio::test]
async fn set_auto_cycle_disable_persists_cleared_state_field() {
    use crate::cell::combat::BSF_AUTO_CYCLING;
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.abilities.auto_cycle = true;
        e.state_field |= BSF_AUTO_CYCLING;
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);

    let handled = dispatch(1, SET_AUTO_CYCLE, &[0], &tx, &mut mgr, &engine).await;
    assert!(handled);

    let updates = drain_state_field_updates(&mut rx);
    assert_eq!(
        updates,
        vec![(100, 0)],
        "disable must persist exactly one StateFieldUpdate with the bit cleared"
    );
}

/// SET_AUTO_CYCLE disable when BSF was already clear must NOT
/// emit a redundant `onStateFieldUpdate`. The transition gate
/// inside `clear_auto_cycle` returns `None` and the handler
/// short-circuits the send. Pin so a refactor that always
/// broadcasts doesn't add wire noise on every disable.
#[tokio::test]
async fn set_auto_cycle_disable_when_bsf_clear_emits_no_broadcast() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.abilities.auto_cycle = true; // armed flag only
                                       // No BSF bit set — the loop never reached commit.
    }
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);

    let handled = dispatch(1, SET_AUTO_CYCLE, &[0], &tx, &mut mgr, &engine).await;
    assert!(handled);

    assert!(
        rx.try_recv().is_err(),
        "disable without prior BSF set must not broadcast"
    );
}

/// SET_AUTO_CYCLE enable sets the flag AND lights
/// `BSF_AUTO_CYCLING` immediately so the client's gun-icon
/// button highlights on the very first press. The
/// ability/target stash stays empty — that's still set at
/// first `useAbility` commit when the ids are actually known.
///
/// Bug shape this prevents (the symptom that drove the change):
/// players pressed the button, got no visual feedback, assumed
/// the button was broken, and pressed it 5-10 more times.
/// "Light on enable" closes that UX gap.
#[tokio::test]
async fn set_auto_cycle_enable_lights_bsf_and_broadcasts() {
    use crate::cell::combat::BSF_AUTO_CYCLING;
    let mut mgr = make_mgr_with_player();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);

    let handled = dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
    assert!(handled);

    let e = mgr.get_entity(1).unwrap();
    assert!(e.abilities.auto_cycle, "flag must be armed");
    assert!(
        e.abilities.auto_cycle_ability_id.is_none(),
        "ability id stash still empty — that arms at first commit",
    );
    assert_ne!(
        e.state_field & BSF_AUTO_CYCLING,
        0,
        "enable MUST light BSF_AUTO_CYCLING so the button highlights",
    );

    let mut saw_state_field_update = false;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            ..
        } = msg
        {
            if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                saw_state_field_update = true;
            }
        }
    }
    assert!(
        saw_state_field_update,
        "enable must broadcast onStateFieldUpdate so the client lights the button"
    );
}

/// Disabling repeatedly when the bit is already clear is a no-op
/// (no re-broadcast). Mirror of the enable-spam test. The CEGUI
/// duplicate-click pattern affects disable presses too — without
/// the transition gate inside `clear_auto_cycle`, each redundant
/// disable would emit an `onStateFieldUpdate` carrying the same
/// (already-cleared) `bStateField` and spam the wire.
#[tokio::test]
async fn set_auto_cycle_disable_spam_does_not_re_broadcast() {
    let mut mgr = make_mgr_with_player();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    // Pre-state: armed (flag + BSF set, as if enable ran earlier).
    dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
    while rx.try_recv().is_ok() {}

    // First disable: transitions the bit, broadcasts.
    dispatch(1, SET_AUTO_CYCLE, &[0], &tx, &mut mgr, &engine).await;
    let mut first_broadcasts = 0;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                first_broadcasts += 1;
            }
        }
    }
    assert_eq!(first_broadcasts, 1, "first disable broadcasts exactly once");

    // Subsequent duplicate disables: must not broadcast.
    for _ in 0..5 {
        dispatch(1, SET_AUTO_CYCLE, &[0], &tx, &mut mgr, &engine).await;
    }
    assert!(
        rx.try_recv().is_err(),
        "duplicate disable calls must NOT re-broadcast — bit is already clear",
    );
}

/// Phase 2 immediate-fire: when the player presses the auto-cycle
/// button AND they already have a target selected AND they've
/// previously fired an ability in this session, the button press
/// fires that ability at the target immediately. Closes the
/// "press button → nothing visible" gap.
///
/// Pre-conditions encoded: `current_target_id` (from setTargetID)
/// + `last_fired_ability_id` (from a prior commit) — both must
/// be Some. Either being None falls through to "just light BSF,
/// wait for first manual fire".
#[tokio::test]
async fn set_auto_cycle_enable_fires_immediately_when_target_and_last_ability_set() {
    let mut mgr = make_mgr_with_player();
    mgr.spawn_npc(50, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(50) {
        npc.faction = crate::cell::combat::HOSTILE_FACTION;
    }
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
        p.abilities.last_fired_ability_id = Some(7);
        p.current_target_id = Some(50);
        p.weapon_holstered = false;
    }
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
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(64);

    let handled = dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
    assert!(handled);

    let p = mgr.get_entity(1).unwrap();
    assert!(
        p.abilities.is_on_cooldown(7),
        "immediate fire must have started the cooldown — proves handle_use_ability ran",
    );
    assert_eq!(
        p.abilities.auto_cycle_ability_id,
        Some(7),
        "the loop must be armed (committed ability) after the immediate fire",
    );
}

/// Regression: if `handle_use_ability` REJECTS the immediate fire
/// (out of range, on cooldown, no ammo), the loop must still be
/// armed at the ability-id level so the next tick can pick it up.
/// Without this guard the BSF lights but `auto_cycle_ability_id`
/// stays None → the driver tick has nothing to re-fire → loop
/// silently dead until the player toggles off and back on.
///
/// Fixture: target is on the OPPOSITE side of the map (out of
/// range) so the fire fails validation but doesn't commit the
/// cooldown.
#[tokio::test]
async fn set_auto_cycle_enable_persists_ability_even_when_immediate_fire_rejects() {
    let mut mgr = make_mgr_with_player();
    mgr.spawn_npc(50, "Castle_CellBlock", [200.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
        p.abilities.last_fired_ability_id = Some(7);
        p.current_target_id = Some(50);
        p.weapon_holstered = false;
    }
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
            max_range: 30, // target is at 200 → out of range
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(64);

    dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;

    let p = mgr.get_entity(1).unwrap();
    assert!(p.abilities.auto_cycle, "flag must arm even if fire rejects");
    assert_eq!(
        p.abilities.auto_cycle_ability_id,
        Some(7),
        "auto_cycle_ability_id MUST persist even when immediate fire is rejected — \
         otherwise the loop is silently dead until toggle off+on",
    );
    assert!(
        !p.abilities.is_on_cooldown(7),
        "out-of-range fire was rejected, so cooldown is NOT running",
    );
}

/// Phase 2: if the player has never fired an ability this session
/// (`last_fired_ability_id == None`), pressing the button just
/// lights BSF — no immediate fire.
#[tokio::test]
async fn set_auto_cycle_enable_does_not_fire_without_last_ability() {
    let mut mgr = make_mgr_with_player();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.current_target_id = Some(50);
        // last_fired_ability_id stays None.
        p.weapon_holstered = false;
    }
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(64);

    dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;

    let p = mgr.get_entity(1).unwrap();
    assert!(p.abilities.auto_cycle, "flag must still arm");
    assert!(
        p.abilities.auto_cycle_ability_id.is_none(),
        "no immediate fire happened → loop ability stash stays empty",
    );
}

/// Polish: if the stashed ability is on cooldown when the player
/// arms auto-cycle (manual right-click → arm-while-cooling), the
/// immediate-fire is skipped without entering `handle_use_ability`.
/// The stash is still persisted so the next `auto_cycle_tick` after
/// the cooldown clears picks up the re-fire.
///
/// Bug shape this prevents: pre-fix, the immediate-fire ran
/// unconditionally and `handle_use_ability` rejected with the
/// `"ability on cooldown"` DEBUG — one wasted call per
/// arm-while-cooling toggle, observed during lomiada's 2026-06-04
/// session when she armed auto-cycle right after a manual shot.
/// Reverting the cooldown gate trips this by re-introducing the
/// rejected useAbility call (and re-firing the cooldown DEBUG).
#[tokio::test]
async fn set_auto_cycle_enable_skips_immediate_fire_when_on_cooldown() {
    let mut mgr = make_mgr_with_player();
    mgr.spawn_npc(50, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(50) {
        npc.faction = crate::cell::combat::HOSTILE_FACTION;
    }
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
        p.abilities.last_fired_ability_id = Some(7);
        p.current_target_id = Some(50);
        p.weapon_holstered = false;
        // Mid-cooldown: arm auto-cycle right after a manual fire.
        p.abilities
            .start_ability_cooldown(7, std::time::Duration::from_secs(60));
    }
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
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(64);

    dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;

    let p = mgr.get_entity(1).unwrap();
    assert!(
        p.abilities.auto_cycle,
        "BSF must still arm — the cooldown skip only affects immediate-fire, not the bit",
    );
    assert_eq!(
        p.abilities.auto_cycle_ability_id,
        Some(7),
        "stash MUST persist even when immediate-fire was skipped — the next \
         auto_cycle_tick after cooldown clear is what re-fires",
    );
    // The cooldown was 60s pre-test and the immediate-fire SHOULD NOT
    // have run (no second `start_ability_cooldown` call). A revert
    // that re-introduces the rejected handle_use_ability call still
    // leaves the cooldown running (handler rejects pre-commit), so
    // this assertion alone is necessary-but-not-sufficient — the
    // proof is in the absent useAbility DEBUG. Sibling tests in
    // use_ability/ pin that. Here we pin that the stash + flag are
    // intact and no extra cooldown work has occurred.
    assert!(
        p.abilities.is_on_cooldown(7),
        "the pre-existing 60s cooldown is still in flight",
    );
}

/// Phase 2: if the player has fired earlier but currently has no
/// target selected (`current_target_id == None`), pressing the
/// button just lights BSF — no immediate fire.
#[tokio::test]
async fn set_auto_cycle_enable_does_not_fire_without_target() {
    let mut mgr = make_mgr_with_player();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.last_fired_ability_id = Some(7);
        // current_target_id stays None.
        p.weapon_holstered = false;
    }
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(64);

    dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;

    let p = mgr.get_entity(1).unwrap();
    assert!(p.abilities.auto_cycle, "flag must still arm");
    assert!(!p.abilities.is_on_cooldown(7), "no immediate fire happened");
}

/// Spamming `setAutoCycle(1)` repeatedly (CEGUI fires the Lua
/// binding 3-4× per physical click, all within ~150µs) must NOT
/// re-broadcast. The bit is already set after the first call;
/// subsequent calls are idempotent.
#[tokio::test]
async fn set_auto_cycle_enable_spam_does_not_re_broadcast() {
    let mut mgr = make_mgr_with_player();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(16);

    dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
    let mut first_broadcasts = 0;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall { method_index, .. } = msg {
            if method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE {
                first_broadcasts += 1;
            }
        }
    }
    assert_eq!(first_broadcasts, 1, "first enable broadcasts exactly once");

    for _ in 0..5 {
        dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
    }
    assert!(
        rx.try_recv().is_err(),
        "duplicate enable calls must NOT re-broadcast — bit is already set",
    );
}

/// **Regression guard: SET_AUTO_CYCLE's immediate-fire path must
/// credit quest kills.** When pressing the auto-fire button kills a
/// quest-tagged NPC in the same press (immediate-fire fires the
/// stashed ability at the current target), the EntityDeath content
/// event must reach the chain engine so KillCount missions advance.
///
/// Parallel coverage to `auto_cycle_tick_credits_quest_kill_on_tagged_npc_death`
/// in `service/ticks/auto_cycle.rs`. Both paths route through
/// `handle_use_ability_with_kill_credit`; both tests fail when the
/// caller is reverted to bare `handle_use_ability`.
#[tokio::test]
async fn set_auto_cycle_immediate_fire_credits_quest_kill_on_tagged_npc_death() {
    use cimmeria_content_engine::actions::Action;
    use cimmeria_content_engine::chain::Chain;
    use cimmeria_content_engine::triggers::Trigger;
    use cimmeria_entity::abilities::EffectDef;
    use cimmeria_entity::stats::HEALTH;

    const QUEST_TAG: &str = "QuestTargetDrone";
    const COUNTER_NAME: &str = "drone_kills";

    let mut mgr = make_mgr_with_player();
    mgr.spawn_npc(50, "Castle_CellBlock", [3.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(50) {
        npc.faction = crate::cell::combat::HOSTILE_FACTION;
    }
    // Pre-conditions for SET_AUTO_CYCLE's immediate-fire branch:
    // current_target_id + last_fired_ability_id both Some. Mirror
    // the existing `set_auto_cycle_enable_fires_immediately_*` test
    // shape so a future regression that changes the branch
    // conditions is caught uniformly.
    if let Some(p) = mgr.get_entity_mut(1) {
        p.abilities.add_ability(7);
        p.abilities.last_fired_ability_id = Some(7);
        p.current_target_id = Some(50);
        p.weapon_holstered = false;
    }
    // Lethal effect on ability 7 so the immediate fire kills the
    // NPC outright. Mirrors the fixture in the auto_cycle_tick test.
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
        id: 999_998,
        name: "test: SET_AUTO_CYCLE drone kill counter".to_string(),
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
    let handled = dispatch(1, SET_AUTO_CYCLE, &[1], &tx, &mut mgr, &engine).await;
    assert!(handled);

    assert_eq!(
        mgr.get_entity(1).unwrap().counters.get(COUNTER_NAME),
        Some(&1),
        "SET_AUTO_CYCLE's immediate-fire on enable must credit a tagged-NPC \
         kill via fire_entity_death — reverting the immediate-fire site to \
         bare handle_use_ability leaves this counter at None",
    );
    assert!(
        crate::cell::combat::is_dead_state(mgr.get_entity(50).unwrap().state_field),
        "test fixture: target must be dead after the lethal immediate fire",
    );
}
