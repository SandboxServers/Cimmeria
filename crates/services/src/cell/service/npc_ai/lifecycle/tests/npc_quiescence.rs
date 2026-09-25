//! What the surrendered NPC itself ends up looking like: non-hostile for
//! the instance's life, channels cancelled, cover slot released,
//! stopped, and facing the player it gave up to. The cleanup this half
//! inherits from `apply_death_transition` is a deliberate *subset* --
//! see the `npc_ai_submit` doc comment for what is left out and why.

use super::*;
use crate::cell::combat::AggroCause;

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
    assert_eq!(npc.ai_state(), AiState::Submit, "the surrender holds");
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
        crate::cell::service::npc_ai::force_ai_state(npc, AiState::Idle);
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
    let registered = crate::cell::effects::register_active_effect(
        &mut mgr,
        PLAYER_A,
        NPC,
        &channel,
        Instant::now(),
        &tx,
    )
    .await;
    assert!(
        registered,
        "fixture invariant: the channel must actually register"
    );
    // Give the handler a reason to run its cleanup pass.
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0, AggroCause::Damage);

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
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0, AggroCause::Damage);

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
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0, AggroCause::Damage);
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

/// The NPC must end up facing the player it surrendered to.
///
/// 2026-09-18 Castle playtest, findings H4b and H6: the AI writes
/// `direction` only as a side effect of translation, and the movement
/// tick skips NPCs with an empty `nav_path` — which the surrender itself
/// clears. Without an explicit re-face the yaw freezes wherever the last
/// chase step left it, and a duelist who circled the NPC watches it
/// kneel at right angles to them.
///
/// Costs no wire traffic: the AoI tick's `EntityMoved` carries
/// `direction` for every entity in view on every pass.
///
/// The NPC is placed on -Z and the player on +X so the expected yaw is a
/// value no other code path would produce by accident — the spawn yaw is
/// 0 and `atan2(0, dz)` for a pure-Z bearing is 0 or PI, so a stuck or
/// axis-swapped facing cannot pass.
#[tokio::test]
async fn submit_turns_the_npc_to_face_the_player_it_surrendered_to() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_npc(&mut mgr, NPC, 0.0);
    // Player at +X, NPC at -Z: bearing from NPC to player is
    // atan2(dx=10, dz=20) — first quadrant, and distinctly not 0.
    if let Some(p) = mgr.get_entity_mut(PLAYER_A) {
        p.position = cimmeria_common::Vector3::new(10.0, 0.0, 0.0);
    }
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.position = cimmeria_common::Vector3::new(0.0, 0.0, -20.0);
        // Frozen mid-chase, facing away from the player.
        npc.direction = cimmeria_common::Vector3::new(0.0, std::f32::consts::PI, 0.0);
    }
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 50.0, AggroCause::Damage);

    content_sets_submit(&mut mgr, NPC);
    let (tx, _rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(NPC).unwrap();
    let expected = 10.0_f32.atan2(20.0);
    assert!(
        (npc.direction.y - expected).abs() < 1e-4,
        "the surrendered NPC must face its attacker: expected yaw \
         {expected} rad, got {} rad",
        npc.direction.y,
    );
    assert_eq!(
        npc.direction.x, 0.0,
        "pitch stays flat — NPCs do not aim up or down",
    );
}

/// The top-threat attacker is the one the NPC turns to, not whichever
/// player happens to hash first in `threat_list`. In the 1v1 duel this
/// is the same player either way, which is exactly why it needs a
/// two-player test: a naive `.next()` on the map passes the duel case
/// and picks a bystander in a group fight.
#[tokio::test]
async fn submit_faces_the_highest_threat_attacker() {
    let mut mgr = make_mgr();
    add_player(&mut mgr, PLAYER_A, 0.0);
    add_player(&mut mgr, PLAYER_B, 0.0);
    add_npc(&mut mgr, NPC, 0.0);
    // A due north of the NPC, B due south. The two bearings are PI apart,
    // so picking the wrong player is unmissable.
    if let Some(p) = mgr.get_entity_mut(PLAYER_A) {
        p.position = cimmeria_common::Vector3::new(0.0, 0.0, 30.0);
    }
    if let Some(p) = mgr.get_entity_mut(PLAYER_B) {
        p.position = cimmeria_common::Vector3::new(0.0, 0.0, -30.0);
    }
    if let Some(npc) = mgr.get_entity_mut(NPC) {
        npc.position = cimmeria_common::Vector3::new(0.0, 0.0, 0.0);
    }
    // B is the chip-damage bystander; A is the duelist.
    let _ = generate_threat(&mut mgr, PLAYER_B, NPC, 5.0, AggroCause::Damage);
    let _ = generate_threat(&mut mgr, PLAYER_A, NPC, 500.0, AggroCause::Damage);

    content_sets_submit(&mut mgr, NPC);
    let (tx, _rx) = mpsc::channel(64);
    run_ai_tick(&tx, &mut mgr).await;

    let yaw = mgr.get_entity(NPC).unwrap().direction.y;
    assert!(
        yaw.abs() < 1e-4,
        "the NPC must face PLAYER_A (+Z, yaw 0), the highest-threat \
         attacker — got {yaw} rad, which is PLAYER_B's bearing",
    );
}
