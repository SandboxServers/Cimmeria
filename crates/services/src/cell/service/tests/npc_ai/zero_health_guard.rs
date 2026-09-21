//! A 0-HEALTH NPC gets no AI turn — the defence-in-depth half of the
//! effect-driven-death fix.
//!
//! Playtest 2026-09-19: a guard whose HEALTH was zeroed by an effect
//! script's Focus-pierce bleed never went through the death path, kept
//! `ai_state == Fighting`, and hit the player for 86 damage while sitting
//! at 0 HP. `abilities::death::resolve_death` stamping `ai_state = Dead`
//! is the primary fix; the health-stat filter in `npc_ai::dispatch` is
//! the backstop, so that a future kill path forgetting the stamp can
//! still not produce a shooting corpse.

use super::{make_ai_fixture, seed_default_ability, seed_target_with_threat};
use crate::cell::combat::NPC_DEFAULT_ABILITY;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

/// The backstop in isolation: an NPC at 0 HEALTH that never got
/// `BSF_DEAD` must be skipped by the tick entirely.
///
/// The discriminator is the empty-threat reset. `npc_ai_fight` turns a
/// `Fighting` NPC with an empty threat list into `Idle` (see
/// `npc_ai_fighting_with_empty_threat_resets_to_idle`), so "still
/// Fighting after a tick" can only mean the handler never ran. Remove the
/// `npc_is_incapacitated` filter in `dispatch.rs` and this NPC ticks,
/// flips to `Idle`, and the assertion fails.
#[tokio::test]
async fn zero_health_npc_without_dead_bit_gets_no_ai_turn() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    // The pre-fix corpse shape: HEALTH zeroed, but no BSF_DEAD and
    // ai_state left at Fighting because no kill path ever ran.
    if let Some(npc) = mgr.get_entity_mut(200) {
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 0, 100);
            h.clear_dirty();
        }
        npc.ai_state = AiState::Fighting;
        npc.threat_list.clear();
    }

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    assert!(
        matches!(mgr.get_entity(200).unwrap().ai_state, AiState::Fighting),
        "a 0-HEALTH NPC must be skipped before its handler runs — reaching \
         npc_ai_fight with an empty threat list would have reset it to Idle"
    );
    assert!(
        rx.try_recv().is_err(),
        "a skipped NPC must emit nothing on the wire"
    );
}

/// End-to-end: a player's Focus-pierce bleed kills an NPC that was
/// mid-fight, and the NPC does not get to shoot back on the next AI tick.
///
/// This is the playtest sequence in miniature. Ability 579 deals zero
/// direct damage and 26 bleed damage (see the `damage_apply` tests for
/// the arithmetic), so pre-fix the NPC finished the shot alive at 0 HP,
/// still `Fighting`, still holding the player on its threat list — and
/// the very next tick fired its own ability back. Post-fix the shot
/// resolves the death, so the tick has nothing to give a turn to.
#[tokio::test]
async fn npc_killed_by_an_effect_bleed_does_not_shoot_back() {
    use crate::cell::abilities::handle_use_ability;
    use cimmeria_entity::abilities::EffectDef;

    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    // Give the NPC a real, in-range, off-cooldown ability so "did it
    // attack?" is observable rather than vacuously false.
    seed_default_ability(&mut mgr, /* min */ 0, /* max */ 30);
    seed_target_with_threat(&mut mgr, 200, 100, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.abilities.add_ability(NPC_DEFAULT_ABILITY);
        npc.faction = crate::cell::combat::HOSTILE_FACTION;
        npc.level = 5;
        // 20 HP with an empty Focus pool: the direct damage of ability
        // 579 is 0 and its script bleed is 26.
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 20, 100);
            h.clear_dirty();
        }
        if let Some(f) = npc.stats.get_mut(cimmeria_entity::stats::FOCUS) {
            f.update(0, 0, 100);
            f.clear_dirty();
        }
    }

    // Player-side ability 579 → effect 641 (`RangedPhysicalDamage`,
    // FocusDamage only), mirroring the live pistol auto attack.
    let mut params = std::collections::HashMap::new();
    params.insert("FocusDamage".to_string(), "80".to_string());
    mgr.effect_defs.insert(
        641,
        EffectDef {
            effect_id: 641,
            script_name: Some("RangedPhysicalDamage".to_string()),
            params,
            ..Default::default()
        },
    );
    mgr.ability_defs.insert(
        579,
        cimmeria_entity::abilities::AbilityDef {
            ability_id: 579,
            name: "Pistol Auto Attack".to_string(),
            cooldown: 0.5,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![641],
            moniker_ids: vec![],
            required_ammo: 0,
            event_set_id: None,
            velocity: 0.0,
        },
    );
    if let Some(p) = mgr.get_entity_mut(100) {
        p.abilities.add_ability(579);
    }

    let (tx, _rx) = mpsc::channel(256);
    assert!(
        handle_use_ability(100, 579, 200, &tx, &mut mgr).await,
        "pre-condition: the player's shot must commit"
    );
    assert_eq!(
        mgr.get_entity(200).unwrap().stats.get(HEALTH).unwrap().cur,
        0,
        "pre-condition: the bleed must have zeroed the NPC's HEALTH"
    );

    let player_hp_before = mgr.get_entity(100).unwrap().stats.get(HEALTH).unwrap().cur;

    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        &mut mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;

    // Assertion order matters: check "did it shoot back" BEFORE the
    // ai_state pin. With both halves of the fix reverted the NPC is at
    // 0 HP, still Fighting, still holding the player on its threat list,
    // and this cooldown assertion is what fails — which is the proof
    // that the assertion is observable rather than vacuously true.
    let npc = mgr.get_entity(200).unwrap();
    assert!(
        !npc.abilities.is_on_cooldown(NPC_DEFAULT_ABILITY),
        "a killed NPC must not fire on the following tick — a started cooldown \
         means it entered the fire path (the playtest's 86-damage hit from a \
         0-HP guard)"
    );
    assert_eq!(
        mgr.get_entity(100).unwrap().stats.get(HEALTH).unwrap().cur,
        player_hp_before,
        "the player must take no damage from a corpse"
    );
    assert!(
        matches!(mgr.get_entity(200).unwrap().ai_state, AiState::Dead),
        "the bleed kill must have left the NPC in AiState::Dead; got {:?}",
        mgr.get_entity(200).unwrap().ai_state
    );
}

// ── Warn throttle ────────────────────────────────────────────────────────
//
// Nothing clears a 0-HEALTH-without-BSF_DEAD NPC, so the natural tick reaches
// it every ~2 s for as long as the zone is up. Unthrottled that is one WARN
// row per NPC per tick, forever, in the console and in SigNoz.

const ZERO_HEALTH_WARN: &str = "skipping NPC at 0 HEALTH with no BSF_DEAD";

/// Put `npc_id` into the invariant-violating shape: HEALTH 0, no `BSF_DEAD`.
fn zero_health_without_death(mgr: &mut crate::cell::space_manager::SpaceManager, npc_id: u32) {
    let npc = mgr.get_entity_mut(npc_id).expect("fixture NPC");
    let h = npc.stats.get_mut(HEALTH).expect("HEALTH stat");
    h.update(0, 0, 100);
    h.clear_dirty();
}

fn zero_health_warns(
    capture: &crate::test_support::LogCaptureGuard,
) -> Vec<crate::test_support::Captured> {
    capture
        .all()
        .into_iter()
        .filter(|e| e.level == tracing::Level::WARN && e.message_contains(ZERO_HEALTH_WARN))
        .collect()
}

/// A burst of ticks inside the window writes one row; the next row after
/// the window reports how many were elided. Every tick still skips the NPC —
/// the throttle governs the log line, never the filter. Removing the
/// `admit` gate writes five rows here.
#[test]
fn zero_health_warn_is_throttled_per_npc_and_reports_suppressed() {
    use crate::cell::service::npc_ai::{npc_is_incapacitated, ZERO_HEALTH_WARN_MIN_INTERVAL};
    use std::time::{Duration, Instant};

    let capture = crate::test_support::LogCapture::install();
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    zero_health_without_death(&mut mgr, 200);

    let t0 = Instant::now();
    for tick in 0..4 {
        let now = t0 + Duration::from_secs(2 * tick);
        assert!(
            npc_is_incapacitated(&mut mgr, 200, now),
            "suppressed or not, the NPC is still skipped"
        );
    }
    assert_eq!(
        zero_health_warns(&capture).len(),
        1,
        "four ticks inside the window write one row"
    );

    assert!(npc_is_incapacitated(
        &mut mgr,
        200,
        t0 + ZERO_HEALTH_WARN_MIN_INTERVAL
    ));
    let rows = zero_health_warns(&capture);
    assert_eq!(rows.len(), 2, "the window elapsed, so the next tick writes");
    assert!(rows[0].has_field("suppressed", "0"), "{:#?}", rows[0]);
    assert!(
        rows[1].has_field("suppressed", "3"),
        "the row after the window counts the three elided ticks: {:#?}",
        rows[1]
    );
}

/// One stuck NPC must not silence another: the slot is per NPC id.
#[test]
fn zero_health_warn_throttle_is_independent_per_npc() {
    use crate::cell::service::npc_ai::npc_is_incapacitated;

    let capture = crate::test_support::LogCapture::install();
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    mgr.create_entity(201, "Castle", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(201) {
        npc.is_player = false;
        npc.class_id = 0x04;
    }
    zero_health_without_death(&mut mgr, 200);
    zero_health_without_death(&mut mgr, 201);

    let now = std::time::Instant::now();
    assert!(npc_is_incapacitated(&mut mgr, 200, now));
    assert!(npc_is_incapacitated(&mut mgr, 201, now));

    let rows = zero_health_warns(&capture);
    assert_eq!(rows.len(), 2, "one row each: {rows:#?}");
    assert!(rows.iter().any(|r| r.has_field("npc_id", "200")));
    assert!(rows.iter().any(|r| r.has_field("npc_id", "201")));
}

/// `destroy_entity` releases the slot, so the table cannot outgrow the live
/// NPC population and a respawn on a recycled id warns on its first bad tick
/// instead of inheriting its predecessor's window.
#[test]
fn destroy_entity_releases_the_zero_health_warn_slot() {
    use crate::cell::service::npc_ai::npc_is_incapacitated;

    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    zero_health_without_death(&mut mgr, 200);
    assert!(npc_is_incapacitated(
        &mut mgr,
        200,
        std::time::Instant::now()
    ));
    assert_eq!(mgr.zero_health_npc_log.tracked(), 1);

    mgr.destroy_entity(200);
    assert_eq!(
        mgr.zero_health_npc_log.tracked(),
        0,
        "the throttle slot must not outlive the NPC"
    );
}
