//! Effect-driven death guards for `apply_damage_to_target` — a HEALTH bleed
//! written by an effect script must produce a death in the same ability
//! resolution.
//!
//! Split from `tests.rs` along the one seam that file has: everything here
//! shares `make_bleed_fixture`, and nothing in `tests.rs` uses it.

use super::single_damage_path_tests::seq_rolling;
use super::tests::{drain, has_method, make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;
use cimmeria_entity::abilities::EffectDef;

// ──────────────────────────────────────────────────────────────────────
// Effect-driven death (playtest 2026-09-19, MessHall_Guard1/2)
//
// Bug shape: the ability's NVP damage leaves the NPC standing, but an
// effect script's HEALTH bleed takes it to zero. Pre-fix nothing noticed:
// the NPC sat at 0 HP with `ai_state` still `Fighting`, kept shooting
// back, dropped no loot and granted no XP.
//
// `make_bleed_fixture` reproduces the live shape (pistol auto attack 579
// / effect 641 = `RangedPhysicalDamage`): a `FocusDamage` NVP and NO
// `HealthDamage` NVP, so every point of health loss comes from the
// script's Focus-pierce spillover. With `FocusDamage = 80` against an
// empty Focus pool the script's two-step truncation gives
// `(80*100/80) * 80 / 300 = 26` health damage — see
// `scripts::RangedPhysicalDamage`. An NPC at 20 HP dies to the bleed.
//
// **AB-06 (D-AB07) changed where the bleed runs.** A damage script is now
// its effect's only damage path and runs with the direct damage, before
// the hit's death check, so these kills resolve through the direct
// `target_died` arm; the post-script sweep still covers after-hit
// scripts (`Suppression`, the last test). Two changes to the fixtures:
// - each shot uses an `effect_seq` whose roll is a hit ([`seq_rolling`]).
//   Before AB-06 a missed roll still ran the script, so the player-victim
//   guard passed on seq 1, which is a miss for NPC 2 with ability 579; a
//   miss now runs no script and deals nothing;
// - the guards fail without either death path, not only the sweep: they
//   pin "a script bleed to zero kills in the same resolution" whichever
//   arm resolves it.
// ──────────────────────────────────────────────────────────────────────

/// Kismet sequence id the fixture registers for `Entity_Death` so the
/// death-animation broadcast is observable.
const DEATH_SEQ_ID: i32 = 4242;

/// Focus-only damage effect bound to a HEALTH-bleeding script.
fn make_bleed_effect(id: i32, script: &str, focus_damage: i32) -> EffectDef {
    let mut params = std::collections::HashMap::new();
    params.insert("FocusDamage".to_string(), focus_damage.to_string());
    EffectDef {
        effect_id: id,
        ability_id: 0,
        delay: 0,
        effect_sequence: 0,
        event_set_id: None,
        script_name: Some(script.to_string()),
        params,
        ..Default::default()
    }
}

/// Fixture: player 1 vs NPC 2, where ability 579 carries only a
/// Focus-gated bleed script. The NPC is left at 20 HP with an empty Focus
/// pool so the bleed (26) is lethal and the direct damage (0) is not.
fn make_bleed_fixture(script: &str) -> (SpaceManager, AbilityDef) {
    let mut mgr = make_mgr_player_vs_npc();
    let ability = make_ability(579, vec![641]);
    mgr.ability_defs.insert(579, ability.clone());
    mgr.effect_defs
        .insert(641, make_bleed_effect(641, script, 80));
    // Event set 1025 (Mob) → Entity_Death (5001) — lets the death
    // animation broadcast be asserted rather than silently skipped.
    mgr.sequence_map.insert((1025, 5001), DEATH_SEQ_ID);
    if let Some(npc) = mgr.get_entity_mut(2) {
        npc.level = 5;
        if let Some(h) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            h.update(0, 20, 100);
            h.clear_dirty();
        }
        // Empty Focus pool: the whole 80 FocusDamage overflows into the
        // spillover formula. A full pool would absorb it and the legacy
        // gate would suppress the HEALTH bleed entirely.
        if let Some(f) = npc.stats.get_mut(cimmeria_entity::stats::FOCUS) {
            f.update(0, 0, 100);
            f.clear_dirty();
        }
    }
    (mgr, ability)
}

/// **The regression guard.** One `RangedPhysicalDamage` shot whose direct
/// damage is zero and whose bleed is lethal must produce a complete
/// corpse inside that single ability resolution — not on the next shot.
///
/// Without a death path for script damage the NPC is left alive at 0 HP
/// with `ai_state == Fighting`, no `BSF_DEAD`, no XP, no loot flip and no
/// death animation: every assertion below fails.
#[tokio::test]
async fn effect_script_bleed_to_zero_runs_death_transition_in_same_resolution() {
    use crate::cell::combat::is_dead_state;
    use cimmeria_entity::cell_entity::AiState;

    let (mut mgr, ability) = make_bleed_fixture("RangedPhysicalDamage");
    let (tx, mut rx) = mpsc::channel(64);

    let seq = seq_rolling(&mgr, (1, 2), 579, false);
    apply_damage_to_target(1, 2, 579, &Some(ability), seq, false, &tx, &mut mgr).await;

    let npc = mgr.get_entity(2).unwrap();
    assert_eq!(
        npc.stats.get(cimmeria_entity::stats::HEALTH).unwrap().cur,
        0,
        "pre-condition: the Focus-pierce bleed must have zeroed HEALTH"
    );
    assert!(
        is_dead_state(npc.state_field),
        "BSF_DEAD must be set by the same ability resolution that zeroed HEALTH — \
         a 0-HP NPC without the bit is the playtest corpse that kept fighting"
    );
    assert!(
        matches!(npc.ai_state(), AiState::Dead),
        "ai_state must be Dead so the AI tick stops giving the corpse turns; got {:?}",
        npc.ai_state()
    );

    let msgs = drain(&mut rx);
    assert_eq!(
        msgs.iter().find_map(|m| match m {
            CellToBaseMsg::GrantXP {
                entity_id,
                xp_amount,
                ..
            } => Some((*entity_id, *xp_amount)),
            _ => None,
        }),
        Some((1, 50)),
        "the bleed kill must pay kill XP (level-5 mob = 50) to the attacker; got {msgs:?}"
    );
    assert!(
        has_method(&msgs, 2, method_idx::INTERACTION_TYPE),
        "the corpse must get its InteractionType flip (loot becomes clickable); got {msgs:?}"
    );
    assert!(
        has_method(&msgs, 2, method_idx::ON_STATE_FIELD_UPDATE),
        "the corpse must get the dead-state flip; got {msgs:?}"
    );
    assert!(
        has_method(&msgs, 2, method_idx::ON_SEQUENCE),
        "the corpse must play the Entity_Death animation; got {msgs:?}"
    );
}

/// `MeleePhysicalDamage` shares the Focus-pierce shape and therefore the
/// same gap. Pinned separately so a fix applied only to the ranged script
/// can't pass.
#[tokio::test]
async fn melee_script_bleed_to_zero_runs_death_transition_in_same_resolution() {
    use crate::cell::combat::is_dead_state;
    use cimmeria_entity::cell_entity::AiState;

    let (mut mgr, ability) = make_bleed_fixture("MeleePhysicalDamage");
    let (tx, mut rx) = mpsc::channel(64);

    let seq = seq_rolling(&mgr, (1, 2), 579, false);
    apply_damage_to_target(1, 2, 579, &Some(ability), seq, false, &tx, &mut mgr).await;

    let npc = mgr.get_entity(2).unwrap();
    assert!(
        is_dead_state(npc.state_field) && matches!(npc.ai_state(), AiState::Dead),
        "MeleePhysicalDamage bleed to zero must also resolve the death; \
         state_field={:#x} ai_state={:?}",
        npc.state_field,
        npc.ai_state()
    );
    assert!(
        drain(&mut rx)
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::GrantXP { .. })),
        "melee bleed kill must pay kill XP"
    );
}

/// Exactly-once: a follow-up shot at the corpse must not re-run the death
/// transition. The bleed still lands (HEALTH clamps at 0) but
/// `resolve_death`'s `BSF_DEAD` probe short-circuits, so no second XP
/// grant, loot flip or death animation goes out.
///
/// This is also what keeps mission kill credit single: the health probe
/// in `handle_use_ability_with_kill_credit` sees `cur <= 0` *before* the
/// follow-up shot and skips — the fix adds no second credit window.
#[tokio::test]
async fn effect_script_bleed_kill_does_not_re_kill_on_a_follow_up_shot() {
    let (mut mgr, ability) = make_bleed_fixture("RangedPhysicalDamage");
    let (tx, mut rx) = mpsc::channel(64);

    let seq = seq_rolling(&mgr, (1, 2), 579, false);
    apply_damage_to_target(1, 2, 579, &Some(ability.clone()), seq, false, &tx, &mut mgr).await;
    let first = drain(&mut rx);
    assert_eq!(
        first
            .iter()
            .filter(|m| matches!(m, CellToBaseMsg::GrantXP { .. }))
            .count(),
        1,
        "pre-condition: the killing shot pays XP exactly once; got {first:?}"
    );

    apply_damage_to_target(1, 2, 579, &Some(ability), seq + 1, false, &tx, &mut mgr).await;
    let second = drain(&mut rx);

    assert!(
        !second
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::GrantXP { .. })),
        "shooting a corpse must not re-grant kill XP; got {second:?}"
    );
    assert!(
        !has_method(&second, 2, method_idx::INTERACTION_TYPE),
        "shooting a corpse must not re-roll loot / re-flip InteractionType; got {second:?}"
    );
    assert!(
        !has_method(&second, 2, method_idx::ON_SEQUENCE),
        "shooting a corpse must not replay the death animation; got {second:?}"
    );
}

/// **The player-victim guard** — the 2026-09-21 colo `.bug` report "why am i
/// not dead". A Cellblock Guard's pistol (effect 641) bled the tester to
/// 0 / 760 HEALTH and nothing followed: `state_field` stayed `8` (in combat
/// only), so the client was never told, the player could still walk, and the
/// guard spent the next 68 s looping "target is dead → drop threat →
/// re-aggro" every six seconds.
///
/// Same gap as the NPC guards above, with the roles reversed: NPC attacker,
/// player victim, lethal bleed. Without a death path for script damage the
/// player is left at 0 HP with no `BSF_DEAD`, no movement lock and no
/// Defeat Window.
#[tokio::test]
async fn effect_script_bleed_to_zero_kills_a_player_victim_in_same_resolution() {
    use crate::cell::combat::{is_dead_state, BSF_MOVEMENT_LOCK};

    let (mut mgr, ability) = make_bleed_fixture("RangedPhysicalDamage");
    if let Some(player) = mgr.get_entity_mut(1) {
        if let Some(h) = player.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            h.update(0, 20, 760);
            h.clear_dirty();
        }
        if let Some(f) = player.stats.get_mut(cimmeria_entity::stats::FOCUS) {
            f.update(0, 0, 1570);
            f.clear_dirty();
        }
    }
    let (tx, mut rx) = mpsc::channel(64);

    // NPC 2 shoots player 1, on a roll that hits.
    let seq = seq_rolling(&mgr, (2, 1), 579, false);
    apply_damage_to_target(2, 1, 579, &Some(ability), seq, false, &tx, &mut mgr).await;

    let player = mgr.get_entity(1).unwrap();
    assert_eq!(
        player
            .stats
            .get(cimmeria_entity::stats::HEALTH)
            .unwrap()
            .cur,
        0,
        "pre-condition: the Focus-pierce bleed must have zeroed the player's HEALTH"
    );
    assert!(
        is_dead_state(player.state_field),
        "a player bled to 0 HEALTH must get BSF_DEAD in the same resolution; \
         state_field={:#x} is the live report's walking corpse",
        player.state_field
    );
    assert_ne!(
        player.state_field & BSF_MOVEMENT_LOCK,
        0,
        "the dead player must be movement-locked; state_field={:#x}",
        player.state_field
    );

    let msgs = drain(&mut rx);
    assert!(
        msgs.iter().any(|m| matches!(
            m,
            CellToBaseMsg::EntityMethodCall { entity_id: 1, method_index, .. }
                if *method_index == method_idx::ON_BEGIN_AID_WAIT
        )),
        "the player must be shown the Defeat Window (onBeginAidWait); got {msgs:?}"
    );
    assert!(
        has_method(&msgs, 1, method_idx::ON_STATE_FIELD_UPDATE),
        "the dead-state flip must reach the wire, or the client keeps walking; got {msgs:?}"
    );
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::GrantXP { .. })),
        "an NPC killing a player pays nobody XP; got {msgs:?}"
    );
}

/// **The sweep guard.** An after-hit script (`Suppression`, not a damage
/// script, so it runs after the hit's death check) chips the last 5
/// HEALTH: the post-script `resolve_death` sweep must still make the
/// corpse in the same resolution. Without the sweep the NPC is left at
/// 0 HP without `BSF_DEAD`.
#[tokio::test]
async fn after_hit_script_to_zero_is_swept_into_a_death() {
    use crate::cell::combat::is_dead_state;

    let (mut mgr, ability) = make_bleed_fixture("Suppression");
    if let Some(h) = mgr
        .get_entity_mut(2)
        .and_then(|npc| npc.stats.get_mut(cimmeria_entity::stats::HEALTH))
    {
        // Suppression's default chip is 5 (the fixture has no HealthDamage).
        h.update(0, 5, 100);
        h.clear_dirty();
    }
    let (tx, _rx) = mpsc::channel(64);
    let seq = seq_rolling(&mgr, (1, 2), 579, false);

    apply_damage_to_target(1, 2, 579, &Some(ability), seq, false, &tx, &mut mgr).await;

    let npc = mgr.get_entity(2).unwrap();
    assert_eq!(
        npc.stats.get(cimmeria_entity::stats::HEALTH).unwrap().cur,
        0,
        "pre-condition: the Suppression chip zeroed HEALTH"
    );
    assert!(
        is_dead_state(npc.state_field),
        "the sweep must resolve a death an after-hit script caused"
    );
}
