//! Tests for `apply_damage_to_target` — extracted to a sibling file
//! to keep `damage_apply/mod.rs` under the 700-line hard cap.

use super::*;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;
use cimmeria_entity::abilities::EffectDef;

/// Fixture: player attacker (id=1) and NPC target (id=2) in the SAME
/// startup space, both at the same position so range checks pass.
/// We use the non-instanced "Castle" world — instanced worlds
/// allocate a fresh space on every `create_entity` call, which would
/// put player and NPC in different spaces and break AoI/witness
/// lookups.
///
/// The player is `connect_entity`'d AND an AoI tick is run so the
/// player ends up in the NPC's witness set. Without that, every
/// `send_entity_method` addressed to the NPC resolves to "no
/// witnesses, drop the message" and the wire-side assertions
/// silently fail.
fn make_mgr_player_vs_npc() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    mgr.create_entity(2, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
    }
    mgr.connect_entity(1);
    let _ = mgr.compute_aoi_changes();
    mgr
}

fn make_ability(id: i32, effect_ids: Vec<i32>) -> AbilityDef {
    AbilityDef {
        ability_id: id,
        name: "test".to_string(),
        cooldown: 0.5,
        warmup: 0.0,
        flags: 0,
        is_ranged: false,
        min_range: 0,
        max_range: 30,
        target_type_id: 0,
        effect_ids,
        moniker_ids: vec![],
        required_ammo: 0,
        event_set_id: None,
        velocity: 0.0,
    }
}

fn make_effect(id: i32, health_damage: i32) -> EffectDef {
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), health_damage.to_string());
    EffectDef {
        effect_id: id,
        ability_id: 0,
        delay: 0,
        effect_sequence: 0,
        event_set_id: None,
        script_name: None,
        params,
        ..Default::default()
    }
}

fn drain(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<CellToBaseMsg> {
    let mut out = Vec::new();
    while let Ok(m) = rx.try_recv() {
        out.push(m);
    }
    out
}

fn has_method(msgs: &[CellToBaseMsg], target: u32, method: u16) -> bool {
    msgs.iter().any(|m| match m {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        }
        | CellToBaseMsg::WitnessEntityMethod {
            entity_id,
            method_index,
            ..
        } => *entity_id == target && *method_index == method,
        _ => false,
    })
}

/// Happy path: player hits NPC for non-lethal damage. Target survives,
/// effect-results + stat-update fire, no GrantXP, no death cascade.
#[tokio::test]
async fn non_lethal_hit_emits_effect_results_and_stat_update_and_no_xp_grant() {
    let mut mgr = make_mgr_player_vs_npc();
    let ability = make_ability(7, vec![100]);
    mgr.ability_defs.insert(7, ability.clone());
    mgr.effect_defs.insert(100, make_effect(100, 5));
    // Give the NPC plenty of health so the hit can't kill it.
    if let Some(npc) = mgr.get_entity_mut(2) {
        if let Some(stat) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            stat.update(0, 1000, 1000);
            stat.clear_dirty();
        }
    }
    let (tx, mut rx) = mpsc::channel(64);

    apply_damage_to_target(1, 2, 7, &Some(ability), 1, false, &tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    // ON_EFFECT_RESULTS fires to the attacker (or its witness routing).
    assert!(
        has_method(&msgs, 1, method_idx::ON_EFFECT_RESULTS),
        "attacker should receive onEffectResults; got {msgs:?}"
    );
    // Target gets a stat update so its health bar moves.
    assert!(
        has_method(&msgs, 2, method_idx::ON_STAT_UPDATE),
        "target should receive onStatUpdate; got {msgs:?}"
    );
    // No GrantXP — target survived.
    assert!(
        !msgs
            .iter()
            .any(|m| matches!(m, CellToBaseMsg::GrantXP { .. })),
        "non-lethal hit must not emit GrantXP; got {msgs:?}"
    );
}

/// Death path: player kills an NPC. Expect GrantXP, dead-state flip,
/// and the death-sequence onSequence (id=1).
#[tokio::test]
async fn lethal_hit_against_npc_emits_grant_xp_and_state_flip() {
    let mut mgr = make_mgr_player_vs_npc();
    let ability = make_ability(7, vec![100]);
    mgr.ability_defs.insert(7, ability.clone());
    // Use a high base damage and low NPC HP to guarantee death.
    // The post-QR/defense-multiplier dampening varies with the
    // RNG seed; pinning a specific damage number is brittle, but
    // pinning "enough damage to kill an entity at HP=1" isn't.
    mgr.effect_defs.insert(100, make_effect(100, 9999));
    if let Some(npc) = mgr.get_entity_mut(2) {
        npc.level = 5;
        if let Some(stat) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            stat.update(0, 1, 100);
            stat.clear_dirty();
        }
    }
    let (tx, mut rx) = mpsc::channel(64);

    apply_damage_to_target(1, 2, 7, &Some(ability), 1, false, &tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    let xp = msgs.iter().find_map(|m| match m {
        CellToBaseMsg::GrantXP {
            entity_id,
            xp_amount,
        } => Some((*entity_id, *xp_amount)),
        _ => None,
    });
    assert_eq!(
        xp,
        Some((1, 50)),
        "kill XP for level-5 mob is 10*5=50, attributed to the attacker"
    );
    // Dead-state update fires on the corpse.
    assert!(
        has_method(&msgs, 2, method_idx::ON_STATE_FIELD_UPDATE),
        "corpse must receive onStateFieldUpdate; got {msgs:?}"
    );
    // NPC AI transitioned to Dead.
    assert!(matches!(
        mgr.get_entity(2).unwrap().ai_state,
        cimmeria_entity::cell_entity::AiState::Dead
    ));
}

/// On a kill, the dying NPC's `threat_list` must survive long enough
/// for `apply_death_transition` → `clear_dead_npc_from_all_player_threat`
/// to walk it and drain each aggroed player's `threatened_mobs`. A
/// previous version cleared `threat_list` at the kill site (before
/// death.rs ran), which left every player permanently in combat —
/// out-of-combat regen could never trigger because `threatened_mobs`
/// was the gate's source of truth.
///
/// Pinning all three end-states here makes the ordering load-bearing:
/// 1. `threatened_mobs` is empty (the gate the regen tick reads).
/// 2. `BSF_InCombat` is cleared (the wire signal the client reads).
/// 3. The corpse's `threat_list` is drained (no stale aggro on the body).
#[tokio::test]
async fn lethal_hit_drains_threatened_mobs_and_clears_bsf_in_combat() {
    use crate::cell::combat::state::BSF_IN_COMBAT;

    let mut mgr = make_mgr_player_vs_npc();
    let ability = make_ability(7, vec![100]);
    mgr.ability_defs.insert(7, ability.clone());
    mgr.effect_defs.insert(100, make_effect(100, 9999));

    // Pre-seed the state a non-killing first shot would have produced:
    // player has the NPC in `threatened_mobs`, NPC has the player in its
    // `threat_list`, and BSF_InCombat is set on the player. The lethal
    // hit below must drain all three.
    if let Some(player) = mgr.get_entity_mut(1) {
        player.threatened_mobs.insert(2);
        player.state_field |= BSF_IN_COMBAT;
    }
    if let Some(npc) = mgr.get_entity_mut(2) {
        npc.threat_list.insert(1, 100.0);
        npc.level = 5;
        if let Some(stat) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            stat.update(0, 1, 100);
            stat.clear_dirty();
        }
    }

    let (tx, _rx) = mpsc::channel(64);
    apply_damage_to_target(1, 2, 7, &Some(ability), 1, false, &tx, &mut mgr).await;

    let player = mgr.get_entity(1).unwrap();
    assert!(
        player.threatened_mobs.is_empty(),
        "player's threatened_mobs must drain on kill — this is the gate \
         the out-of-combat regen tick reads. Stuck non-empty set means \
         regen never fires in production."
    );
    assert_eq!(
        player.state_field & BSF_IN_COMBAT,
        0,
        "BSF_InCombat must clear on the kill that empties \
         threatened_mobs. Stuck bit means HUD/cursor stay in \
         combat-ready forever."
    );

    let corpse = mgr.get_entity(2).unwrap();
    assert!(
        corpse.threat_list.is_empty(),
        "corpse's threat_list drains AFTER apply_death_transition has \
         consumed it — leaving stale aggro on the body would mask future \
         re-spawn aggro state."
    );
}

/// One-shot kill regression guard: a single lethal hit against an NPC
/// must NOT leave the attacker stranded with BSF_InCombat set. The
/// previous bug: `use_ability` set the bit raw before damage_apply
/// ran, but `generate_threat` (which is what calls
/// `enter_player_combat` to populate `threatened_mobs` and broadcast
/// the bit) is gated on `!target_died` (mod.rs:413). So a one-shot
/// kill set the bit and never cleared it — the corpse's `threat_list`
/// stayed empty, `clear_dead_npc_from_all_player_threat` found
/// nothing to drain, and the bit stuck forever.
///
/// After the funnel fix, use_ability doesn't set the bit raw; the
/// kill path never sets the bit; the assertion below pins that
/// outcome.
#[tokio::test]
async fn one_shot_kill_does_not_strand_attacker_bsf_in_combat() {
    use crate::cell::abilities::handle_use_ability;
    use crate::cell::combat::state::BSF_IN_COMBAT;

    let mut mgr = make_mgr_player_vs_npc();
    let ability = make_ability(7, vec![100]);
    mgr.ability_defs.insert(7, ability);
    // Overkill damage on a 1-HP NPC — guarantees target_died = true
    // and `generate_threat` is skipped entirely on this hit.
    mgr.effect_defs.insert(100, make_effect(100, 9999));
    if let Some(player) = mgr.get_entity_mut(1) {
        // `handle_use_ability` validates the attacker owns the ability
        // before the damage path runs; the previous `apply_damage_to_target`
        // shortcut bypassed this check.
        player.abilities.add_ability(7);
    }
    if let Some(npc) = mgr.get_entity_mut(2) {
        npc.level = 5;
        if let Some(stat) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            stat.update(0, 1, 100);
            stat.clear_dirty();
        }
    }
    // Pre-condition: attacker is NOT in combat (fresh).
    assert_eq!(mgr.get_entity(1).unwrap().state_field & BSF_IN_COMBAT, 0);

    let (tx, _rx) = mpsc::channel(64);
    // Drive the full public entry point — the previous regression
    // path was a raw `state_field |= BSF_IN_COMBAT` inside
    // `handle_use_ability` *before* `apply_damage_to_target`
    // decided to skip `generate_threat` on `target_died=true`.
    // Bypassing into `apply_damage_to_target` directly would have
    // hidden the regression. Use the public entrypoint instead.
    let committed = handle_use_ability(1, 7, 2, &tx, &mut mgr).await;
    assert!(committed, "use_ability should commit on a valid one-shot");

    let attacker = mgr.get_entity(1).unwrap();
    assert_eq!(
        attacker.state_field & BSF_IN_COMBAT,
        0,
        "one-shot kill must not strand BSF_InCombat on the attacker — \
         the previous raw setter in use_ability set the bit before \
         generate_threat decided to skip on target_died=true, and no \
         clear path ever ran"
    );
    assert!(
        attacker.threatened_mobs.is_empty(),
        "one-shot kill must leave threatened_mobs empty — the gate the \
         out-of-combat regen tick reads"
    );
}

/// Player target dying: onBeginAidWait must fire so the client shows
/// the Defeat Window. Carries a TimeToAid prefix and a respawner array.
#[tokio::test]
async fn lethal_hit_against_player_target_emits_on_begin_aid_wait() {
    let mut mgr = make_mgr_player_vs_npc();
    // Promote the NPC target to a player so the death path takes the
    // player-target branch.
    if let Some(t) = mgr.get_entity_mut(2) {
        t.is_player = true;
        t.player_id = Some(200);
        if let Some(stat) = t.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            stat.update(0, 1, 100);
            stat.clear_dirty();
        }
    }
    let ability = make_ability(7, vec![100]);
    mgr.ability_defs.insert(7, ability.clone());
    mgr.effect_defs.insert(100, make_effect(100, 50));
    let (tx, mut rx) = mpsc::channel(64);

    apply_damage_to_target(1, 2, 7, &Some(ability), 1, false, &tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    let aid = msgs.iter().find_map(|m| match m {
        CellToBaseMsg::EntityMethodCall {
            entity_id: 2,
            method_index,
            args,
        } if *method_index == method_idx::ON_BEGIN_AID_WAIT => Some(args.clone()),
        _ => None,
    });
    let args = aid.expect("player target must receive onBeginAidWait");
    // Payload begins with TimeToAid as INT32. The exact value (30s) is
    // module policy, not just byte layout — pin it so a regression
    // can't quietly change the defeat window.
    assert!(args.len() >= 4);
    let time_to_aid = i32::from_le_bytes([args[0], args[1], args[2], args[3]]);
    assert_eq!(time_to_aid, 30);
}

/// Defensive bail: attacker vanished between the consume mutation
/// and the snapshot. With `needs_ammo_stat_send = false` the helper
/// must return cleanly without emitting any target packets. The
/// twin scenario — `needs_ammo_stat_send = true` causing the
/// dirty-ammo-stat flush even on bail — is covered by the
/// AoE-secondary tests on `handle_use_ability_on_ground` (those
/// pass `false` for secondaries but `true` for the primary, which
/// exercises the flush branch). Pin the false-branch here so a
/// refactor that always flushes regardless of the flag gets caught.
#[tokio::test]
async fn missing_attacker_with_no_pending_flush_bails_without_target_packets() {
    let mut mgr = make_mgr_player_vs_npc();
    let ability = make_ability(7, vec![100]);
    mgr.effect_defs.insert(100, make_effect(100, 5));
    // Remove the attacker BEFORE calling.
    mgr.destroy_entity(1);
    let (tx, mut rx) = mpsc::channel(64);

    apply_damage_to_target(1, 2, 7, &Some(ability), 1, false, &tx, &mut mgr).await;

    let msgs = drain(&mut rx);
    assert!(
        !has_method(&msgs, 2, method_idx::ON_STAT_UPDATE),
        "missing attacker must not emit a target onStatUpdate; got {msgs:?}"
    );
}

/// Player-attacker damage doubling is a known temporary balance hack
/// (`// Temp: 2x player damage so players can kill NPCs before dying`).
/// Pin "player deals strictly more damage than an NPC with the same
/// ability" so the comment-and-the-multiplier stay coupled — when
/// the hack is removed, this test is the canary that says so. The
/// exact ratio isn't 2x because `combat::calculate_damage` applies
/// QR + defense reductions on top, and integer truncation skews the
/// post-reduction ratio away from the input ratio.
#[tokio::test]
async fn player_attacker_does_more_damage_than_npc_attacker() {
    let mut mgr = make_mgr_player_vs_npc();
    let ability = make_ability(7, vec![100]);
    mgr.ability_defs.insert(7, ability.clone());
    // Use a large base so post-reduction damage is well above the
    // integer-truncation noise floor.
    mgr.effect_defs.insert(100, make_effect(100, 100));
    if let Some(npc) = mgr.get_entity_mut(2) {
        if let Some(stat) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            stat.update(0, 10_000, 10_000);
            stat.clear_dirty();
        }
    }
    let (tx, _rx) = mpsc::channel(64);
    apply_damage_to_target(1, 2, 7, &Some(ability.clone()), 1, false, &tx, &mut mgr).await;
    let after_player = mgr
        .get_entity(2)
        .unwrap()
        .stats
        .get(cimmeria_entity::stats::HEALTH)
        .unwrap()
        .cur;

    // Reset and run again with NPC attacker.
    let mut mgr2 = make_mgr_player_vs_npc();
    if let Some(p) = mgr2.get_entity_mut(1) {
        p.is_player = false;
        p.player_id = None;
    }
    mgr2.ability_defs.insert(7, ability.clone());
    mgr2.effect_defs.insert(100, make_effect(100, 100));
    if let Some(npc) = mgr2.get_entity_mut(2) {
        if let Some(stat) = npc.stats.get_mut(cimmeria_entity::stats::HEALTH) {
            stat.update(0, 10_000, 10_000);
            stat.clear_dirty();
        }
    }
    let (tx2, _rx2) = mpsc::channel(64);
    apply_damage_to_target(1, 2, 7, &Some(ability), 1, false, &tx2, &mut mgr2).await;
    let after_npc = mgr2
        .get_entity(2)
        .unwrap()
        .stats
        .get(cimmeria_entity::stats::HEALTH)
        .unwrap()
        .cur;

    let player_dmg = 10_000 - after_player;
    let npc_dmg = 10_000 - after_npc;
    assert!(
            player_dmg > npc_dmg,
            "player attacker must deal strictly more damage than NPC attacker; player={player_dmg}, npc={npc_dmg}"
        );
}

/// Player dying mid-reload / mid-draw / mid-slot-swap must clear
/// every pending weapon-action timer. Same-world respawn
/// (`ReanchorPlayer`) reuses the cell entity, so any timer that
/// survives death will fire its tick during the Defeat Window or
/// post-respawn — phantom reload animations, queued attacks
/// against stale targets, slot-swap choreography on a corpse.
///
/// This guard stages a player with EVERY pending weapon field set
/// to `Some`, lethally damages them, and asserts each field is
/// `None` afterward.
#[tokio::test]
async fn player_death_clears_pending_weapon_action_state() {
    use std::time::{Duration, Instant};

    let mut mgr = make_mgr_player_vs_npc();
    // Make entity 2 the attacker (NPC) and entity 1 the target (player).
    // The fixture sets player at id=1 already; we just give the NPC an
    // ability so its damage_apply call has effects to resolve.
    let lethal = make_ability(99, vec![777]);
    mgr.effect_defs.insert(777, make_effect(777, 9_999));
    mgr.ability_defs.insert(99, lethal.clone());
    if let Some(p) = mgr.get_entity_mut(1) {
        // Pre-stage every pending timer with arbitrary future deadlines
        // so the post-death assertion is observable.
        let future = Some(Instant::now() + Duration::from_secs(60));
        p.pending_reload_at = future;
        p.reload_complete_at = future;
        p.reload_slot_id = Some(0);
        p.pending_attack_at = future;
        p.pending_attack_ability_id = Some(7);
        p.pending_attack_target_id = Some(42);
        p.pending_slot_swap_at = future;
        p.pending_slot_swap_target = Some(3);
        p.holster_animation_complete_at = future;
        p.combat_exit_at = Some(Instant::now());
        // Set HP to a single point so the test's 9999 damage kills.
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 1, 100);
            h.clear_dirty();
        }
    }

    let (tx, _rx) = mpsc::channel(64);
    // NPC attacker (id=2) hits player target (id=1). The effect chain
    // delivers 9999 damage, dropping HP from 1 to <0 → target_died = true.
    apply_damage_to_target(2, 1, 99, &Some(lethal), 0, false, &tx, &mut mgr).await;

    let player = mgr
        .get_entity(1)
        .expect("player entity must survive death — same-world respawn reuses it");
    assert_eq!(
        player.pending_reload_at, None,
        "pending_reload_at must clear on death — otherwise the next \
         pending_reload_tick fires Phase B mid-Defeat-Window, starting \
         a phantom reload"
    );
    assert_eq!(player.reload_complete_at, None);
    assert_eq!(player.reload_slot_id, None);
    assert_eq!(
        player.pending_attack_at, None,
        "pending_attack_at must clear on death — otherwise \
         pending_attack_tick re-fires `useAbility` against the cached \
         target post-respawn, producing a queued shot the player \
         never asked for"
    );
    assert_eq!(player.pending_attack_ability_id, None);
    assert_eq!(player.pending_attack_target_id, None);
    assert_eq!(player.pending_slot_swap_at, None);
    assert_eq!(player.pending_slot_swap_target, None);
    assert_eq!(player.holster_animation_complete_at, None);
    assert_eq!(player.combat_exit_at, None);
}
