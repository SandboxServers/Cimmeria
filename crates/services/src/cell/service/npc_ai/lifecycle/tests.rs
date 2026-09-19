//! `AiState::Submit` disengagement guards.
//!
//! The bug shape these reproduce: `npc_ai_submit` used to wipe the NPC's
//! own `threat_list` in place, which leaves every attacker holding the
//! NPC in `threatened_mobs` forever (stuck `BSF_InCombat`, weapon stays
//! drawn, `regen_tick` permanently gated off) and leaves their auto-fire
//! loop running, so the surrendered NPC gets shot dead seconds later.
//!
//! Two stop mechanisms are pinned here because they run on different
//! clocks: the AI-side handler (~2 s cadence, reached via `npc_ai_tick`)
//! and the `auto_cycle_tick` target-validity gate (100 ms cadence), which
//! is what actually closes the kill window.

use std::collections::HashMap;
use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_entity::abilities::{AbilityDef, EffectDef};
use cimmeria_entity::cell_entity::{AiState, MobMovementType};
use cimmeria_entity::stats::HEALTH;

use crate::cell::combat::{
    arm_auto_cycle, generate_threat, HOSTILE_FACTION, {BSF_AUTO_CYCLING, BSF_IN_COMBAT},
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

const NPC: u32 = 200;
const PLAYER_A: u32 = 1;
const PLAYER_B: u32 = 2;
/// Ranged, 30-unit, no-ammo — keeps the auto-cycle fixtures about loop
/// semantics rather than ammo or range.
const ABILITY: i32 = 7;

fn make_mgr() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

/// Connected player at `x`. `connect_entity` is required: the auto-cycle
/// sweep iterates `all_player_entity_ids()`, which only returns players
/// in the space's `players` set.
fn add_player(mgr: &mut SpaceManager, id: u32, x: f32) {
    mgr.create_entity(id, "Castle", [x, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(id) {
        p.is_player = true;
        p.player_id = Some(100 + id as i32);
        p.weapon_holstered = false;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    mgr.connect_entity(id);
}

/// Hostile NPC at `x`. The hostile faction matters because the
/// idle-auto-aggro scan skips same-faction players, and players default
/// to faction 0.
fn add_npc(mgr: &mut SpaceManager, id: u32, x: f32) {
    mgr.spawn_npc(id, "Castle", [x, 0.0, 0.0], [0.0; 3]).unwrap();
    if let Some(npc) = mgr.get_entity_mut(id) {
        npc.faction = HOSTILE_FACTION;
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
}

/// Put `npc_id` into Submit exactly the way the content action does —
/// a bare `ai_state` write plus a nav clear, no cleanup of its own.
fn content_sets_submit(mgr: &mut SpaceManager, npc_id: u32) {
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.ai_state = AiState::Submit;
        npc.nav_path.clear();
    }
}

async fn run_ai_tick(tx: &mpsc::Sender<CellToBaseMsg>, mgr: &mut SpaceManager) {
    crate::cell::service::npc_ai::npc_ai_tick(
        tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

/// Arm `player_id`'s auto-fire loop at `target_id`, the way a first
/// committed fire with `setAutoCycle(1)` already pressed leaves it.
fn arm_loop_at(mgr: &mut SpaceManager, player_id: u32, target_id: u32) {
    if let Some(p) = mgr.get_entity_mut(player_id) {
        p.abilities.add_ability(ABILITY);
        p.abilities.auto_cycle = true;
        p.current_target_id = Some(target_id as i32);
    }
    let armed = arm_auto_cycle(mgr, player_id, ABILITY, target_id as i32);
    assert!(
        armed.is_some(),
        "fixture invariant: the loop must actually arm"
    );
}

fn install_ability_def(mgr: &mut SpaceManager) {
    mgr.ability_defs.insert(
        ABILITY,
        AbilityDef {
            ability_id: ABILITY,
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
}

/// `onStateFieldUpdate` payloads addressed to `entity_id`'s own client.
fn state_updates_for(msgs: &[CellToBaseMsg], entity_id: u32) -> Vec<u32> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: eid,
                method_index,
                args,
            } if *eid == entity_id
                && *method_index == crate::mercury::method_idx::ON_STATE_FIELD_UPDATE =>
            {
                Some(u32::from_le_bytes(args[..4].try_into().unwrap()))
            }
            _ => None,
        })
        .collect()
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

// ── The player-side scrub (H04 worknote request B) ─────────────────────

/// Submit reached the way the Rin'la ritual reaches it — the player shot
/// the NPC, so the attacker is genuinely in combat — must leave the
/// attacker OUT of combat: dropped from `threatened_mobs`, `BSF_InCombat`
/// cleared, and the clear pushed to their client.
///
/// Reverting the `clear_dead_npc_from_all_player_threat` call to an
/// in-place `threat_list.clear()` fails this on the `threatened_mobs`
/// assertion: the NPC's list empties, the player's set does not.
#[tokio::test]
async fn submit_from_a_health_crossing_takes_the_attacker_out_of_combat() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);

    // The crossing hit: player damages the NPC, both sides enter combat.
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);
    assert_ne!(
        mgr.get_entity(PLAYER_A).unwrap().state_field & BSF_IN_COMBAT,
        0,
        "fixture invariant: the attacker must be in combat before the submit"
    );

    content_sets_submit(&mut mgr, NPC);
    let (tx, mut rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert!(
        player.threatened_mobs.is_empty(),
        "the surrendered NPC must be dropped from the attacker's threatened_mobs — \
         this set, not the BSF bit, is what gates regen and the weapon posture",
    );
    assert_eq!(
        player.state_field & BSF_IN_COMBAT,
        0,
        "BSF_InCombat must clear once the last threatening mob surrendered",
    );
    assert!(
        mgr.get_entity(NPC).unwrap().threat_list.is_empty(),
        "and the NPC's own list is still drained",
    );

    let updates = state_updates_for(&drain(&mut rx), PLAYER_A);
    assert_eq!(
        updates.len(),
        1,
        "exactly one onStateFieldUpdate to the attacker's own client",
    );
    assert_eq!(
        updates[0] & BSF_IN_COMBAT,
        0,
        "and the broadcast payload must carry the cleared bit",
    );
}

/// Two players on the same NPC: the surrender must scrub BOTH, not just
/// whoever landed the crossing hit. Same multi-attacker shape the death
/// path fans out — a killer-only fix leaves the second player stuck in
/// combat with no mob to blame.
///
/// Fails on the same revert as the single-attacker case, and
/// additionally on any "scrub only the top-threat attacker" narrowing.
#[tokio::test]
async fn submit_clears_every_attacker_not_just_the_last_one() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_player(&mut mgr, PLAYER_B, 2.0);
    add_npc(&mut mgr, NPC, 5.0);

    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);
    let _ = generate_threat(&mut mgr, PLAYER_B, NPC, 90.0);
    for pid in [PLAYER_A, PLAYER_B] {
        assert_ne!(
            mgr.get_entity(pid).unwrap().state_field & BSF_IN_COMBAT,
            0,
            "fixture invariant: player {pid} must be in combat"
        );
    }

    content_sets_submit(&mut mgr, NPC);
    let (tx, mut rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    for pid in [PLAYER_A, PLAYER_B] {
        let player = mgr.get_entity(pid).unwrap();
        assert!(
            player.threatened_mobs.is_empty(),
            "player {pid} must be scrubbed too",
        );
        assert_eq!(player.state_field & BSF_IN_COMBAT, 0);
        assert_eq!(
            state_updates_for(&msgs, pid).len(),
            1,
            "player {pid} must get their own onStateFieldUpdate",
        );
    }
}

/// A player still threatened by a second, unrelated mob stays in combat
/// when the first one surrenders. Pins that the surrender routes through
/// the shared per-mob scrub rather than nuking `threatened_mobs`
/// wholesale — the failure shape would be a player who walks away from a
/// live fight with no combat HUD and full regen.
#[tokio::test]
async fn submit_leaves_a_player_in_combat_with_another_live_mob() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    add_npc(&mut mgr, NPC + 1, 8.0);

    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC + 1, 50.0);

    content_sets_submit(&mut mgr, NPC);
    let (tx, _rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert_eq!(
        player.threatened_mobs.len(),
        1,
        "only the surrendered mob leaves the set",
    );
    assert!(player.threatened_mobs.contains(&(NPC + 1)));
    assert_ne!(
        player.state_field & BSF_IN_COMBAT,
        0,
        "the other mob is still fighting, so the attacker stays in combat",
    );
}

// ── The auto-cycle stop (H04 worknote request C) ───────────────────────

/// Submit must stop the attacker's auto-fire loop: `auto_cycle` off,
/// `BSF_AUTO_CYCLING` cleared, and the un-highlight pushed to the client.
///
/// Reverting the `clear_auto_cycle_for_target` call fails this — the loop
/// stays armed and the tick keeps re-firing until the NPC dies.
#[tokio::test]
async fn submit_stops_the_attackers_auto_fire_loop() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);

    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);
    arm_loop_at(&mut mgr, PLAYER_A, NPC);

    content_sets_submit(&mut mgr, NPC);
    let (tx, mut rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert!(
        !player.abilities.auto_cycle,
        "the loop must be disarmed, not just skipped for a tick",
    );
    assert_eq!(player.abilities.auto_cycle_ability_id, None);
    assert_eq!(
        player.state_field & BSF_AUTO_CYCLING,
        0,
        "BSF_AutoCycling must clear so the client un-highlights the button",
    );
    assert!(
        !state_updates_for(&drain(&mut rx), PLAYER_A).is_empty(),
        "the attacker must be told about the state change",
    );
}

/// A player auto-firing at a DIFFERENT mob keeps their loop when this one
/// surrenders. Pins the live-`current_target_id` filter — the failure
/// shape is a surrender that silently cancels a bystander's attack.
#[tokio::test]
async fn submit_leaves_a_loop_aimed_at_another_mob_alone() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    add_npc(&mut mgr, NPC + 1, 8.0);

    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);
    arm_loop_at(&mut mgr, PLAYER_A, NPC + 1);

    content_sets_submit(&mut mgr, NPC);
    let (tx, _rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert!(
        player.abilities.auto_cycle,
        "the loop is aimed elsewhere and must survive",
    );
    assert_ne!(player.state_field & BSF_AUTO_CYCLING, 0);
}

/// The durable half of the auto-cycle stop, and the one that actually
/// makes a post-surrender kill impossible: `auto_cycle_tick` runs every
/// 100 ms while the AI handler above runs every ~2 s, so without this
/// gate the loop lands several more shots before the surrender is
/// processed.
///
/// Reverting `is_auto_cycle_target_valid` to a bare `!is_dead_state`
/// check fails this — the tick takes the re-fire branch, starts the
/// ability cooldown, and the surrendered NPC takes another hit.
#[tokio::test]
async fn auto_cycle_tick_refuses_to_re_fire_at_a_surrendered_npc() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    install_ability_def(&mut mgr);
    arm_loop_at(&mut mgr, PLAYER_A, NPC);
    content_sets_submit(&mut mgr, NPC);

    let (tx, _rx) = mpsc::channel(64);
    let npc_health_before = mgr.get_entity(NPC).unwrap().stats.get(HEALTH).unwrap().cur;

    crate::cell::service::ticks::auto_cycle_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert!(
        !player.abilities.auto_cycle,
        "a surrendered target must clear the loop, not merely skip a tick",
    );
    assert!(
        !player.abilities.is_on_cooldown(ABILITY),
        "and no shot may be fired — a started cooldown means the tick re-fired",
    );
    assert_eq!(
        mgr.get_entity(NPC).unwrap().stats.get(HEALTH).unwrap().cur,
        npc_health_before,
        "the surrendered NPC must take no further auto-fire damage",
    );
}

/// Control for the test above: the same fixture with the NPC left
/// Fighting DOES re-fire. Without this, a gate that rejected every target
/// would look identical to a correct one.
#[tokio::test]
async fn auto_cycle_tick_still_re_fires_at_a_fighting_npc() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    install_ability_def(&mut mgr);
    arm_loop_at(&mut mgr, PLAYER_A, NPC);

    let (tx, _rx) = mpsc::channel(64);
    crate::cell::service::ticks::auto_cycle_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert!(player.abilities.auto_cycle, "a live target keeps the loop");
    assert!(
        player.abilities.is_on_cooldown(ABILITY),
        "and the tick fires — proving the surrender case above is the gate, \
         not a fixture that never fires at all",
    );
}

// ── Non-hostility for the instance's life ──────────────────────────────

/// A hostile-on-sight NPC that is pushed into Submit without ever having
/// fought must be disarmed, and must stay disarmed after something else
/// flips it back to Idle.
///
/// Two reverts fail this. Dropping `npc.aggression = 0` fails the final
/// assertion: the Idle flip re-admits the NPC to the aggression branch of
/// the AI tick and it seeds threat on the player standing next to it.
/// Dropping the `aggression > 0` term from the cleanup probe fails it for
/// a subtler reason — this NPC never fought, so `last_movement_type` is
/// `None` and `threat_list` is empty, and the handler early-outs before
/// reaching the disarm at all.
#[tokio::test]
async fn a_submitted_npc_does_not_re_aggro_on_proximity() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 2.0);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.aggression = 3;
    }
    // The player has to be a witness for the idle-auto-aggro scan to see
    // them at all.
    let _ = mgr.compute_aoi_changes();

    content_sets_submit(&mut mgr, NPC);
    let (tx, _rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.ai_state, AiState::Submit, "the surrender holds");
    assert_eq!(npc.aggression, 0, "and the NPC is disarmed");
    assert!(
        mgr.get_entity(PLAYER_A).unwrap().threatened_mobs.is_empty(),
        "standing next to a surrendered NPC must not start a fight",
    );

    // Now the part the disarm exists for: something pushes the NPC back
    // to Idle (a content `set_npc_ai_state idle`, a follow-target that
    // resolves to nothing, the GM console, a respawn). A still-aggressive
    // NPC would immediately aggro the player standing beside it.
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.ai_state = AiState::Idle;
    }
    run_ai_tick(&tx, &mut mgr).await;

    assert!(
        mgr.get_entity(PLAYER_A).unwrap().threatened_mobs.is_empty(),
        "an un-submitted NPC must stay pacified — the surrender cleared the \
         hostile-on-sight switch, not just the current state",
    );
    assert!(mgr.get_entity(NPC).unwrap().threat_list.is_empty());
}

// ── Adjacent cleanup the surrender inherits from the death path ────────

/// A surrendered NPC stops pulsing. Without this the player who just
/// accepted the surrender keeps taking channel damage from the NPC that
/// gave up — the channel has no owner-death event to cancel it, because
/// the NPC never dies.
///
/// Reverting the `cancel_channels_from_attacker` call leaves the
/// instance on the player.
#[tokio::test]
async fn submit_cancels_channels_the_npc_was_running() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);

    // pulse_count == 0 is what marks an effect as channelled.
    let mut params = HashMap::new();
    params.insert("HealthDamage".to_string(), "10".to_string());
    let channel = EffectDef {
        effect_id: 9101,
        ability_id: 4242,
        pulse_count: 0,
        pulse_duration: 0.5,
        params,
        ..Default::default()
    };
    mgr.effect_defs.insert(9101, channel.clone());

    let (tx, _rx) = mpsc::channel(64);
    let registered =
        crate::cell::effects::register_active_effect(&mut mgr, PLAYER_A, NPC, &channel, Instant::now(), &tx)
            .await;
    assert!(
        registered,
        "fixture invariant: the channel must actually register"
    );
    // Give the handler a reason to run its cleanup pass.
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);

    content_sets_submit(&mut mgr, NPC);
    run_ai_tick(&tx, &mut mgr).await;

    assert!(
        mgr.get_entity(PLAYER_A).unwrap().active_effects.is_empty(),
        "the surrendered NPC's channel must be cancelled off the player",
    );
}

/// A surrendered NPC releases its cover slot. It never leaves Submit on
/// its own, so a held reservation would be unavailable to every other NPC
/// in that chunk for the rest of the instance — the same forever-leak the
/// death path releases for a corpse, and Submit was the only combat-exit
/// path missing it (death, threat-empty and leash all release).
#[tokio::test]
async fn submit_releases_the_npcs_cover_slot() {
    use cimmeria_common::EntityId;

    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    let slot = crate::cell::cover::CoverSlotKey::new(7, 3);
    mgr.cover
        .reservations
        .lock()
        .unwrap()
        .reserve_for_entity(EntityId(NPC as i32), slot)
        .expect("fixture invariant: the slot must reserve");
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);

    content_sets_submit(&mut mgr, NPC);
    let (tx, _rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let reservations = mgr.cover.reservations.lock().unwrap();
    assert_eq!(
        reservations.slot_for_entity(EntityId(NPC as i32)),
        None,
        "the surrendered NPC must not hold a cover slot for the instance's life",
    );
    assert!(
        !reservations.is_reserved(slot),
        "and the slot is free to reuse"
    );
}

// ── Handler shape ──────────────────────────────────────────────────────

/// The pre-existing quiesce still happens: movement-type cache dropped,
/// velocity zeroed, the NPC's own combat bit cleared. Not a regression
/// guard for this packet — it pins the behaviour the rewrite had to carry
/// forward unchanged, so a later refactor of the handler can't quietly
/// drop it.
#[tokio::test]
async fn submit_still_quiesces_the_npc_itself() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.state_field |= BSF_IN_COMBAT;
        npc.last_movement_type = Some(MobMovementType::CombatAdvance);
        npc.velocity = [1.0, 0.0, 1.0];
        npc.ai_retry_at = Some(Instant::now());
    }

    content_sets_submit(&mut mgr, NPC);
    let (tx, _rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.state_field & BSF_IN_COMBAT, 0);
    assert_eq!(npc.last_movement_type, None);
    assert_eq!(npc.velocity, [0.0; 3]);
    assert_eq!(npc.ai_retry_at, None);
}

/// Re-engaging a surrendered NPC re-runs the cleanup on the next AI pass.
///
/// This is the leak the probe's `!threat_list.is_empty()` term exists for:
/// `generate_threat` writes the attacker into a submitted NPC's
/// `threat_list` and calls `enter_player_combat` *outside* the
/// state-preemption guard, so every stray bullet at a surrendered NPC
/// puts the shooter back into permanent combat. Submit has no death or
/// leash exit to undo it — this handler is the only scrub. Replacing the
/// probe with a one-shot latch fails this test.
///
/// Explicit attacks themselves are unchanged: the shot lands, damage
/// applies, and the NPC can still be killed.
#[tokio::test]
async fn re_engaging_a_surrendered_npc_scrubs_the_attacker_again() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 5.0);
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0);

    content_sets_submit(&mut mgr, NPC);
    let (tx, _rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;
    assert!(mgr.get_entity(PLAYER_A).unwrap().threatened_mobs.is_empty());

    // Player shoots the surrendered NPC anyway — allowed, and it re-arms
    // both sides' combat state.
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 25.0);
    assert_ne!(
        mgr.get_entity(PLAYER_A).unwrap().state_field & BSF_IN_COMBAT,
        0,
        "fixture invariant: the stray shot must put the player back in combat"
    );
    assert_eq!(
        mgr.get_entity(NPC).unwrap().ai_state,
        AiState::Submit,
        "and the NPC must NOT be preempted back into Fighting",
    );

    run_ai_tick(&tx, &mut mgr).await;

    let player = mgr.get_entity(PLAYER_A).unwrap();
    assert!(
        player.threatened_mobs.is_empty(),
        "the next AI pass must scrub the attacker again — otherwise one stray \
         shot at a surrendered NPC denies that player regen for the session",
    );
    assert_eq!(player.state_field & BSF_IN_COMBAT, 0);
}
