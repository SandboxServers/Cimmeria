use super::*;
use cimmeria_entity::abilities::AbilityDef;
use cimmeria_entity::cell_entity::BandolierItem;

fn make_mgr_with_player() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(1, "Castle_CellBlock", [0.0; 3], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    mgr.connect_entity(1);
    mgr
}

#[tokio::test]
async fn dispatch_returns_false_for_unknown_method() {
    let mut mgr = make_mgr_with_player();
    let engine = ChainEngine::new();
    let (tx, _rx) = mpsc::channel(8);
    let handled = dispatch(1, 9999, &[], &tx, &mut mgr, &engine).await;
    assert!(!handled);
}

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

/// TRIGGER_REGION with a negative region_id must be rejected by
/// the explicit `u32::try_from` guard, NOT by accidentally
/// missing a sign-extended u32 lookup. Pre-seed a real region at
/// the sign-extended id (`-5i32 as u32 == 0xFFFFFFFB`); if the
/// regression resurfaces (the cast slips through), the lookup
/// would match the planted region and fire content events.
/// With the negative-id guard in place the planted region must
/// stay invisible.
#[tokio::test]
async fn trigger_region_with_negative_id_rejects_via_explicit_guard() {
    use crate::cell::space_manager::RegionData;
    let mut mgr = make_mgr_with_player();
    // Plant a region at the sign-extended id of -5. If a regression
    // reintroduces the `region_id as u32` cast, get_region(0xFFFFFFFB)
    // would match this row and fire ring_transport / fire_enter_region.
    let trap_id: u32 = (-5i32) as u32;
    mgr.regions.insert(
        trap_id,
        RegionData {
            runtime_id: trap_id,
            db_set_id: 9999,
            tag: "trap".to_string(),
            world_name: "Castle_CellBlock".to_string(),
            height: 0.0,
            radius: 0.0,
            flags: 0,
            points: vec![],
        },
    );

    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(8);

    // Layout: i32 region_id + u8 b_entering + 3 × f32 position.
    let mut args = Vec::with_capacity(17);
    args.extend_from_slice(&(-5i32).to_le_bytes());
    args.push(1);
    args.extend_from_slice(&0.0f32.to_le_bytes());
    args.extend_from_slice(&0.0f32.to_le_bytes());
    args.extend_from_slice(&0.0f32.to_le_bytes());

    let handled = dispatch(1, TRIGGER_REGION, &args, &tx, &mut mgr, &engine).await;
    assert!(
        handled,
        "TRIGGER_REGION must claim the method even when region_id is bogus"
    );
    // The planted trap region MUST NOT match. No fire_*_region
    // cascade, no ring_transport message.
    assert!(
        rx.try_recv().is_err(),
        "negative region_id must be rejected by u32::try_from before lookup, \
         so the trap region at 0xFFFFFFFB can't fire"
    );
}

/// `handle_reload` is a no-op when the active slot is at full
/// clip and no reload is in flight. Pin so a refactor that
/// always starts a reload (and therefore wastes ammo on every
/// keypress) gets caught.
#[tokio::test]
async fn handle_reload_no_op_when_already_full() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 30, // full
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    let (tx, mut rx) = mpsc::channel(8);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_none(),
        "no reload should be queued when full"
    );
    assert!(rx.try_recv().is_err(), "no packets should be emitted");
}

/// `handle_reload` from an empty magazine pins the slot id at
/// the time of issue. If the player swaps mid-reload, the
/// completion tick must refill THIS slot, not whatever slot is
/// active when the deadline elapses.
#[tokio::test]
async fn handle_reload_pins_reload_slot_id_to_current_active_slot() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        // Weapon already drawn so Phase A (defer-reload-for-draw)
        // doesn't kick in — this test asserts the Phase B slot
        // pin, not the Phase A defer path.
        e.weapon_holstered = false;
        e.bandolier_items.insert(
            2,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 2;
    }
    // Seed the reload AbilityDef so warmup/cooldown/event_set are read.
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    let (tx, _rx) = mpsc::channel(16);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.reload_complete_at.is_some(),
        "reload must arm the deadline"
    );
    assert_eq!(
        e.reload_slot_id,
        Some(2),
        "reload_slot_id must capture the active slot at issue time, not be re-read at completion"
    );
}

/// Reload-start must fire the `Item_Reload` (event 4002) sequence from
/// the player's archetype-keyed "Item handling" event set so the
/// client plays the visible reload animation. Mirrors
/// `python/cell/SGWBeing.py:863-874` (`getItemSequence(Item_Reload)` +
/// `playSequence`). Previously this site looked up the *reload
/// ability's* `event_set_id`, which is NULL in the seed — the lookup
/// short-circuited and no animation ever played in production.
///
/// Bug shape this catches: a refactor that goes back to keying off
/// `ability_defs[596].event_set_id` reintroduces the dead path.
#[tokio::test]
async fn handle_reload_sends_item_reload_sequence() {
    use crate::cell::client_methods::spawnable_entity::ON_SEQUENCE;
    use crate::cell::spawner::{EVENT_ABILITY_BEGIN, EVENT_ITEM_RELOAD};

    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        // Soldier archetype → event set 804 (the human "Item handling"
        // set per `archetype_item_event_set`).
        e.archetype_id = Some(1);
        // Weapon already drawn so Phase A (defer-reload-for-draw)
        // doesn't kick in — this test asserts the Phase B byte
        // layout of Item_Reload's ON_SEQUENCE.
        e.weapon_holstered = false;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    // Seed the sequence_map so the (804, Item_Reload) lookup finds a
    // sentinel sequence id we can recognise on the wire.
    const HUMAN_ITEM_EVENT_SET: i32 = 804;
    const ITEM_RELOAD_SEQ_ID: i32 = 1874;
    mgr.sequence_map.insert(
        (HUMAN_ITEM_EVENT_SET, EVENT_ITEM_RELOAD),
        ITEM_RELOAD_SEQ_ID,
    );
    // Seed a decoy (ability's event set → Ability_Begin) — the
    // regression we're guarding against would have sent THIS one.
    const DECOY_EVENT_SET: i32 = 7777;
    const DECOY_BEGIN_SEQ_ID: i32 = 9001;
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: Some(DECOY_EVENT_SET),
            velocity: 0.0,
        },
    );
    mgr.sequence_map
        .insert((DECOY_EVENT_SET, EVENT_ABILITY_BEGIN), DECOY_BEGIN_SEQ_ID);

    let (tx, mut rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    // ON_SEQUENCE wire layout (26 bytes — matches use_ability.rs's fire path):
    //   sequence_id   i32 LE  @ 0..4
    //   source_id     i32 LE  @ 4..8
    //   target_id     i32 LE  @ 8..12
    //   primary       u8      @ 12
    //   impact_time   f32 LE  @ 13..17
    //   nvp_count     u32 LE  @ 17..21
    //   view_type     u8      @ 21
    //   instance_id   i32 LE  @ 22..26
    let mut item_reload_count = 0;
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == ON_SEQUENCE {
                assert_eq!(
                    args.len(),
                    26,
                    "ON_SEQUENCE payload must be exactly 26 bytes — any drift \
                     in the serializer would silently corrupt the kismet event \
                     frame on the wire"
                );
                let seq_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                assert_ne!(
                    seq_id, DECOY_BEGIN_SEQ_ID,
                    "reload-start must NOT fire the reload ability's Ability_Begin \
                     (that's the dead pre-fix path)",
                );
                if seq_id != ITEM_RELOAD_SEQ_ID {
                    continue;
                }
                let source_id = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                let target_id = i32::from_le_bytes([args[8], args[9], args[10], args[11]]);
                let primary = args[12];
                let impact_time = f32::from_le_bytes([args[13], args[14], args[15], args[16]]);
                let nvp_count = u32::from_le_bytes([args[17], args[18], args[19], args[20]]);
                let view_type = args[21];
                let instance_id = i32::from_le_bytes([args[22], args[23], args[24], args[25]]);
                assert_eq!(
                    source_id, 1,
                    "source = entity_id (player firing the reload)"
                );
                assert_eq!(target_id, 1, "reload targets self");
                assert_eq!(primary, 1, "primary target flag set");
                assert_eq!(impact_time, 0.0, "no projectile impact time for reload");
                assert_eq!(nvp_count, 0, "no name-value pairs in payload");
                assert_eq!(
                    view_type, 0,
                    "ViewType=0 (KISMET_VIEW_Witness) — matches use_ability.rs's \
                     fire path so reload-begin animates consistently with weapon \
                     fire animations"
                );
                assert_eq!(instance_id, 0, "no effect instance for the reload sequence");
                item_reload_count += 1;
            }
        }
    }
    assert_eq!(
        item_reload_count, 1,
        "reload-start must send exactly one onSequence with the Item_Reload \
         sequence id; without it the client plays no visible reload animation. \
         Got {item_reload_count}.",
    );
}

/// Reload-start must emit the ammo-type update under
/// `GENERICPROPERTY_AmmoTypeId` (propId 3), NOT under propId 7
/// (`GENERICPROPERTY_AccessLevel`).
///
/// Bug shape this catches (issue #168): the historical handler used
/// a hardcoded `7i32` for the propId arg of `onEntityProperty`,
/// which made the post-reload property update land on the client's
/// `setAccessLevel` slot with the ammo type as the value. The HUD's
/// ammo-type indicator still updated in practice because the
/// bandolier sync path independently emits propId 3 on cur_ammo_type
/// changes — so reverting the fix wouldn't break the visible HUD,
/// but it would re-plant a stray `setAccessLevel(<ammo_type>)` on
/// the wire. A refactor that goes back to `7i32.to_le_bytes()` here
/// would fail this guard.
#[tokio::test]
async fn handle_reload_emits_ammo_type_under_correct_propid() {
    use crate::cell::cell_methods::inventory::GENERICPROPERTY_AMMO_TYPE_ID;
    use crate::cell::client_methods::spawnable_entity::ON_ENTITY_PROPERTY;
    use crate::cell::spawner::EVENT_ITEM_RELOAD;

    const AMMO_TYPE: i32 = 42;

    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_holstered = false;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: AMMO_TYPE,
                current_ammo: 0,
                cur_ammo_type: AMMO_TYPE,
            },
        );
        e.active_bandolier_slot = 0;
    }
    // Seed the sequence_map so Phase B doesn't bail before reaching
    // the propId emit — same shape as the sibling Item_Reload test.
    mgr.sequence_map.insert((804, EVENT_ITEM_RELOAD), 1874);

    let (tx, mut rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let mut entity_property_calls = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            method_index, args, ..
        } = msg
        {
            if method_index == ON_ENTITY_PROPERTY {
                assert_eq!(args.len(), 8, "onEntityProperty payload is 8 bytes LE");
                let prop_id = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
                let value = i32::from_le_bytes([args[4], args[5], args[6], args[7]]);
                entity_property_calls.push((prop_id, value));
            }
        }
    }

    let ammo_call = entity_property_calls
        .iter()
        .find(|(_, v)| *v == AMMO_TYPE)
        .copied();
    assert_eq!(
        ammo_call,
        Some((GENERICPROPERTY_AMMO_TYPE_ID, AMMO_TYPE)),
        "ammo-type onEntityProperty must use propId {GENERICPROPERTY_AMMO_TYPE_ID} (AmmoTypeId), \
         not propId 7 (AccessLevel). All onEntityProperty calls observed: {entity_property_calls:?}",
    );
    assert!(
        !entity_property_calls
            .iter()
            .any(|(p, v)| *p == 7 && *v == AMMO_TYPE),
        "no onEntityProperty(7, <ammo_type>) — that's the pre-fix shape (issue #168). \
         All onEntityProperty calls observed: {entity_property_calls:?}",
    );
}

/// Reload-while-holstered Phase A: a player who's OOC and
/// holstered presses reload. The handler defers the actual reload
/// to give the draw animation time to play. Phase A must:
///   1. Flip `weapon_holstered` to false.
///   2. Stamp `combat_exit_at` so the OOC re-holster timer fires
///      AFTER the eventual Phase B reload completes.
///   3. Set `pending_reload_at = now + UNHOLSTER_DRAW_DURATION` so
///      `pending_reload_tick` can promote Phase A → Phase B.
///   4. Dispatch `RefreshAppearance` (mesh attaches at hand socket).
///   5. NOT start the reload-completion timer or fire `Item_Reload`
///      yet — those land in Phase B.
///
/// Bug shape this catches (the playtest report that drove the fix):
/// firing `Item_Reload` and the appearance change in the same tick
/// makes the weapon "teleport into the hand + reload anim plays on
/// empty space", and the player has to press reload twice.
#[tokio::test]
async fn reload_while_holstered_phase_a_defers_reload() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = true; // OOC + holstered
        e.combat_exit_at = None;
        e.pending_reload_at = None;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }

    let (tx, mut rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(!e.weapon_holstered, "Phase A must draw the weapon");
    assert!(
        e.combat_exit_at.is_some(),
        "Phase A must stamp combat_exit_at so OOC re-holster fires AFTER \
         the eventual reload completes",
    );
    assert!(
        e.pending_reload_at.is_some(),
        "Phase A must set pending_reload_at so the deferred-reload tick \
         can promote to Phase B once the draw window elapses",
    );
    assert!(
        e.reload_complete_at.is_none(),
        "Phase A must NOT start the reload-completion timer — the actual \
         reload hasn't started yet, only the draw. Firing the reload here \
         is the bug shape we're explicitly avoiding (user playtest: \
         'weapon teleports into my hand and I still need to hit reload again')",
    );

    let mut saw_refresh = false;
    while let Ok(msg) = rx.try_recv() {
        if matches!(
            msg,
            CellToBaseMsg::RefreshAppearance {
                holstered: false,
                ..
            }
        ) {
            saw_refresh = true;
            break;
        }
    }
    assert!(
        saw_refresh,
        "Phase A must dispatch RefreshAppearance(holstered=false) so the \
         client attaches the weapon mesh at the hand socket before the \
         draw animation triggers",
    );
}

/// Phase A → Phase B promotion: once the draw window has
/// elapsed, calling `handle_reload` again (as the
/// `pending_reload_tick` does) finds `pending_reload_at` set,
/// clears it, and runs the normal Phase B reload start
/// (`reload_complete_at` armed, `Item_Reload` sequence fired).
///
/// Bug shape this catches: a refactor that forgets to clear
/// `pending_reload_at` in Phase B leaves the tick re-firing
/// `handle_reload` every 100ms forever.
#[tokio::test]
async fn reload_phase_a_to_phase_b_clears_pending_and_starts_reload() {
    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        // Already drawn by Phase A; `pending_reload_at` is what the
        // promotion key reads.
        e.weapon_holstered = false;
        e.combat_exit_at = Some(std::time::Instant::now());
        e.pending_reload_at = Some(std::time::Instant::now());
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );

    let (tx, _rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.pending_reload_at.is_none(),
        "Phase B must clear pending_reload_at so the tick doesn't re-fire \
         handle_reload every 100ms forever",
    );
    assert!(
        e.reload_complete_at.is_some(),
        "Phase B must start the reload (set reload_complete_at) so the \
         completion tick can promote the ammo refill",
    );
}

/// Reload-while-in-OOC-grace (weapon already drawn): the timer
/// must be RE-STAMPED so it doesn't fire `OOC_HOLSTER_DELAY`
/// seconds after combat ended — which could land mid-reload and
/// holster the weapon while the animation is still playing.
#[tokio::test]
async fn reload_during_ooc_grace_resets_holster_timer() {
    let mut mgr = make_mgr_with_player();
    let stale_stamp = std::time::Instant::now() - std::time::Duration::from_secs(8);
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = false; // OOC but still drawn
        e.combat_exit_at = Some(stale_stamp);
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }

    let (tx, _rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(!e.weapon_holstered, "already-drawn weapon stays drawn");
    let new_stamp = e.combat_exit_at.expect("timer must remain armed");
    assert!(
        new_stamp > stale_stamp,
        "timer must be re-stamped to current time so the existing \
         OOC_HOLSTER_DELAY countdown doesn't expire mid-reload",
    );
}

/// Second reload press during the Phase A draw window must be
/// silently ignored. Without this gate, the second press falls
/// through to Phase B, clears `pending_reload_at` early, and
/// starts the reload cooldown immediately — defeating the draw
/// animation timing.
///
/// Bug shape: refactor drops the `now < pending_reload_at` check
/// at the top of Phase B; a player mashing R during the draw
/// window triggers Phase B prematurely and the reload anim
/// chains in mid-draw (the symptom that drove the original
/// two-phase split).
#[tokio::test]
async fn reload_second_press_during_draw_window_is_ignored() {
    let mut mgr = make_mgr_with_player();
    let future = std::time::Instant::now() + std::time::Duration::from_millis(800);
    if let Some(e) = mgr.get_entity_mut(1) {
        e.archetype_id = Some(1);
        e.weapon_visual = Some("WP-Human.WP_Pistol_1A".into());
        e.weapon_holstered = false; // weapon drawn (Phase A finished its draw)
        e.combat_exit_at = Some(std::time::Instant::now());
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
        // Phase A already fired — Phase B is queued for the future.
        e.pending_reload_at = Some(future);
    }
    // No reload ability def needed — the gate fires before any
    // ability lookup.

    let (tx, _rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert_eq!(
        e.pending_reload_at,
        Some(future),
        "second press must NOT clear pending_reload_at — the \
         tick still owns the Phase B promotion at the right time",
    );
    assert!(
        e.reload_complete_at.is_none(),
        "second press must NOT start the reload cooldown — Phase B \
         would otherwise fire mid-draw and chain the reload \
         animation before the unholster motion finishes",
    );
}

/// Reload-in-isolation regression: reloading without any aggro must
/// NOT flip BSF_InCombat on the player. The previous bug: the
/// reload handler set the bit raw, but reload doesn't generate
/// threat on anything — so no NPC death would ever clear the bit,
/// stranding the player in the in-combat HUD/cursor forever (and
/// blocking the out-of-combat regen tick, which gates on
/// `threatened_mobs.is_empty()`).
///
#[tokio::test]
async fn reload_in_isolation_does_not_flip_bsf_in_combat() {
    use crate::cell::combat::BSF_IN_COMBAT;

    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0,
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
    }
    // Seed the reload AbilityDef so the warmup path runs.
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );

    let (tx, _rx) = mpsc::channel(64);
    handle_reload(1, &tx, &mut mgr).await;

    let s = mgr.get_entity(1).unwrap().state_field;
    assert_eq!(
        s & BSF_IN_COMBAT,
        0,
        "reload MUST NOT flip BSF_InCombat — reload-without-aggro had no \
         NPC-death clear path and the bit would strand forever"
    );
    assert!(
        mgr.get_entity(1).unwrap().threatened_mobs.is_empty(),
        "reload must leave threatened_mobs empty — the source of truth \
         for the in-combat state"
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

/// `handle_reload` Phase B must cancel any in-flight OOC holster
/// Phase 2 (`holster_animation_complete_at`). Reload semantics imply
/// "the weapon stays drawn"; without this cancel, the Phase 2
/// deadline still elapses mid-reload and broadcasts a fresh
/// `BeingAppearance` with no weapon attached. The user sees the
/// reload animation play, then the weapon mesh vanishes on the
/// next AoI tick.
///
/// Bug shape: player drains clip with `autoReload = false`,
/// watches OOC holster Phase 1 fire (`Item_Unequip` animation),
/// then manually presses R during the ~half-second animation
/// window before Phase 2 elapses. Pre-fix Phase B was missing the
/// `holster_animation_complete_at = None` clear that Phase A
/// already had (line 554), so the holster timer kept running.
///
/// Pin: stage `holster_animation_complete_at = Some(future)`,
/// drain the clip, call `handle_reload` with the weapon drawn
/// (forcing the Phase B path), assert the field is `None`
/// afterward AND a fresh reload deadline is armed.
#[tokio::test]
async fn handle_reload_phase_b_cancels_in_flight_holster_phase_2() {
    use std::time::{Duration, Instant};

    let mut mgr = make_mgr_with_player();
    if let Some(e) = mgr.get_entity_mut(1) {
        // Weapon drawn — forces Phase B (drawn-reload) path, not
        // Phase A (reload-while-holstered).
        e.weapon_holstered = false;
        e.bandolier_items.insert(
            0,
            BandolierItem {
                item_id: 1,
                clip_size: 30,
                default_ammo_type: 2,
                current_ammo: 0, // empty → reload will arm
                cur_ammo_type: 2,
            },
        );
        e.active_bandolier_slot = 0;
        // Stage the in-flight Phase 2 holster — Phase 1 fired
        // moments ago, animation is playing, mesh-drop deadline is
        // ~half a second out.
        e.holster_animation_complete_at = Some(Instant::now() + Duration::from_millis(500));
    }
    mgr.ability_defs.insert(
        596,
        AbilityDef {
            ability_id: 596,
            name: "reload".to_string(),
            cooldown: 1.0,
            warmup: 0.5,
            flags: 0,
            is_ranged: false,
            min_range: 0,
            max_range: 0,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );

    let (tx, _rx) = mpsc::channel(16);
    handle_reload(1, &tx, &mut mgr).await;

    let e = mgr.get_entity(1).unwrap();
    assert!(
        e.holster_animation_complete_at.is_none(),
        "Phase B must clear holster_animation_complete_at — reload \
         semantics imply weapon stays drawn. Pre-fix, Phase 2 would \
         still elapse mid-reload and drop the weapon mesh."
    );
    assert!(
        e.reload_complete_at.is_some(),
        "fixture sanity: Phase B must arm reload_complete_at — if this \
         fails, the test isn't exercising the Phase B branch"
    );
}
