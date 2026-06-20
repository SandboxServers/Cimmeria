//! Respawn-tick fan-out tests: loot-UI close for still-looting players,
//! the zero-witness silent path, the full damage_apply → death → respawn
//! integration loop, and the player-target guard on `broadcast_movement_type`.
//!
//! Split out of the monolithic `npc_respawn/tests.rs` (issue #529) — every
//! test body and assertion is byte-identical to the original.

use super::super::*;
use super::fixtures::{drain, make_mgr_with_dead_npc};
use crate::cell::combat::{BSF_DEAD, BSF_MOVEMENT_LOCK};
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;

/// **A3 fix pin**: a player whose `looting_entity` points at the
/// respawning corpse must have their loot UI closed (server sends
/// onLootDisplay with an empty list, count = 0) and `looting_entity`
/// cleared. Without this, the player would have a stale UI showing
/// items that no longer exist on the respawned-alive NPC.
#[tokio::test]
async fn respawn_closes_loot_ui_for_still_looting_players() {
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
    // Player 1 was mid-loot on the corpse when respawn fires.
    if let Some(p) = mgr.get_entity_mut(1) {
        p.looting_entity = Some(50);
    }

    let (tx, mut rx) = mpsc::channel(64);
    npc_respawn_tick(&tx, &mut mgr).await;

    let p = mgr.get_entity(1).unwrap();
    assert!(
            p.looting_entity.is_none(),
            "looting_entity must be cleared by respawn so subsequent take-item calls don't reference a dead corpse",
        );
    // Verify the wire packet that closes the UI: onLootDisplay
    // (method 114) with entity_id = NPC id, count = 0, initial = 0.
    let msgs = drain(&mut rx);
    let close_pkt = msgs.iter().find_map(|m| match m {
        CellToBaseMsg::EntityMethodCall {
            entity_id: 1,
            method_index,
            args,
        } if *method_index == method_idx::ON_LOOT_DISPLAY => Some(args.clone()),
        _ => None,
    });
    let args = close_pkt.expect("player must receive onLootDisplay close packet");
    // Payload layout: i32 entity_id, u32 count, u8 initial — 9 bytes.
    assert_eq!(
        args.len(),
        9,
        "close packet payload pin (entity_id + count + initial)"
    );
    assert_eq!(
        i32::from_le_bytes(args[0..4].try_into().unwrap()),
        50,
        "entity_id must be the respawning NPC's id",
    );
    assert_eq!(
        u32::from_le_bytes(args[4..8].try_into().unwrap()),
        0,
        "count must be 0 — Loot.lua hides window on count==0",
    );
    assert_eq!(args[8], 0, "initial flag must be 0 (not a fresh display)");
}

/// **A3 sibling**: when a player is NOT looting the respawning
/// corpse, no onLootDisplay packet is sent to them. Catches a
/// regression that would send the close packet to every player
/// in the space.
#[tokio::test]
async fn respawn_does_not_close_loot_ui_for_unrelated_players() {
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = make_mgr_with_dead_npc(Some(30), Some(past));
    // Player 1 is in the space but NOT looting the corpse.
    assert!(mgr.get_entity(1).unwrap().looting_entity.is_none());

    let (tx, mut rx) = mpsc::channel(64);
    npc_respawn_tick(&tx, &mut mgr).await;

    let any_loot_close = drain(&mut rx).iter().any(|m| {
        matches!(
            m,
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                ..
            } if *method_index == method_idx::ON_LOOT_DISPLAY
        )
    });
    assert!(
        !any_loot_close,
        "non-looting player must not receive an onLootDisplay close packet",
    );
}

/// **B3 fix pin**: when the dying NPC has zero witnesses, the
/// respawn tick must complete the state mutation but emit zero
/// wire packets and zero warn-level logs. Pre-fix: each
/// `send_entity_method` on a zero-witness NPC emitted a warn
/// ("NPC has no witnesses, method dropped"), producing 3 warns
/// per respawn cycle per NPC at scale.
#[tokio::test]
async fn respawn_with_no_witnesses_is_silent_and_state_correct() {
    // Fresh manager with ONLY an NPC — no player witness in the
    // space, so the NPC has zero observers.
    let past = std::time::Instant::now() - std::time::Duration::from_millis(1);
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.spawn_npc(50, "Castle", [10.0, 0.0, 0.0], [0.0, 1.57, 0.0])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(50) {
        npc.set_state_flag(BSF_DEAD);
        npc.set_state_flag(BSF_MOVEMENT_LOCK);
        npc.ai_state = AiState::Dead;
        if let Some(hp) = npc.stats.get_mut(HEALTH) {
            hp.set_current(0);
        }
        npc.respawn_secs = Some(30);
        npc.respawn_at = Some(past);
    }
    let _ = mgr.compute_aoi_changes();
    assert!(
        mgr.get_witnesses_of(50).is_empty(),
        "fixture invariant: zero-witness NPC",
    );

    let (tx, mut rx) = mpsc::channel(64);
    npc_respawn_tick(&tx, &mut mgr).await;

    // Server state correct.
    let npc = mgr.get_entity(50).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle);
    assert_eq!(npc.state_field & BSF_DEAD, 0);
    let hp = npc.stats.get(HEALTH).unwrap();
    assert_eq!(hp.cur, hp.max);

    // No wire packets emitted. (The wire helpers we use are the
    // no-warn variants, so the empty-witness branch is a clean
    // no-op rather than a per-method warn.)
    assert!(
        drain(&mut rx).is_empty(),
        "zero-witness respawn must produce zero wire messages",
    );
}

/// **C2 / B2 pin**: drive the full kill → respawn → re-kill loop
/// THROUGH `damage_apply::apply_damage_to_target` (rather than
/// mimicking the death-side mutations inline). Ensures that the
/// `combat::mark_npc_dead` helper extraction stays correctly
/// wired — if a future refactor stops calling it from
/// `damage_apply`, the loop test catches the missing respawn
/// stamp because the second cycle's `respawn_at` would never be
/// set and the tick wouldn't promote the NPC back.
///
/// Goes through the public ability dispatch path so the test
/// shape catches breakage along the full damage → death →
/// respawn pipeline, not just the respawn tick in isolation.
#[tokio::test]
async fn kill_via_damage_apply_then_respawn_then_kill_again() {
    use cimmeria_entity::abilities::{AbilityDef, EffectDef};

    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    // Player attacker.
    mgr.create_entity(1, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(100);
        p.abilities.add_ability(7);
    }
    // NPC target with respawn_secs = 3 (the minimum, so the
    // assertion that respawn_at is stamped is meaningful).
    mgr.spawn_npc(50, "Castle", [3.0, 0.0, 0.0], [0.0, 1.57, 0.0])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(50) {
        // Hostile so the #444 single-target target-validity gate
        // allows the attack through to the kill path this test drives.
        npc.faction = crate::cell::combat::HOSTILE_FACTION;
        npc.respawn_secs = Some(3);
        npc.original_interaction_type_flags = 1 << 5;
        npc.interaction_type_flags = 1 << 5;
        if let Some(hp) = npc.stats.get_mut(HEALTH) {
            hp.update(0, 1, 100); // one-shottable
            hp.clear_dirty();
        }
    }
    // Lethal ability fixture (mirrors the auto_cycle / pending_attack
    // tests' lethal ability setup).
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), "9999".to_string());
    mgr.effect_defs.insert(
        100,
        EffectDef {
            effect_id: 100,
            ability_id: 7,
            params,
            ..Default::default()
        },
    );
    mgr.ability_defs.insert(
        7,
        AbilityDef {
            ability_id: 7,
            name: "test".to_string(),
            cooldown: 0.0,
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
    mgr.connect_entity(1);
    let _ = mgr.compute_aoi_changes();

    let (tx, _rx) = mpsc::channel(128);

    // First kill — drive through the real ability dispatch path.
    let _ = crate::cell::abilities::handle_use_ability(1, 7, 50, &tx, &mut mgr).await;

    // damage_apply → mark_npc_dead must have stamped respawn_at.
    let npc = mgr.get_entity(50).unwrap();
    assert_eq!(npc.ai_state, AiState::Dead, "cycle 1 → Dead");
    assert!(
        npc.respawn_at.is_some(),
        "cycle 1 → respawn_at must be stamped by mark_npc_dead",
    );
    let hp = npc.stats.get(HEALTH).unwrap();
    assert_eq!(hp.cur, 0, "cycle 1 → HP=0");

    // Verify the death-side mutations that mark_npc_dead is
    // responsible for. The pin point is the integration —
    // damage_apply must call mark_npc_dead, which must stamp
    // respawn_at + transition ai_state + clear nav state.
    // Re-running the loop through damage_apply for cycle 2 is
    // covered by `kill_respawn_rekill_loop_is_idempotent`, which
    // mimics the kill directly to keep that test's ability /
    // cooldown plumbing scope-limited.
    assert!(npc.nav_path.is_empty(), "mark_npc_dead must clear nav_path",);
    assert_eq!(npc.last_movement_type, None);
    assert_eq!(
        npc.state_flag_counts.get(&BSF_DEAD).copied(),
        Some(1),
        "mark_npc_dead must counted-set BSF_DEAD (not raw bit op)",
    );

    // Advance the deadline to the past so the respawn tick consumes it.
    if let Some(npc) = mgr.get_entity_mut(50) {
        npc.respawn_at = Some(std::time::Instant::now() - std::time::Duration::from_millis(1));
    }
    npc_respawn_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(50).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle, "post-respawn → Idle");
    assert_eq!(npc.state_field & BSF_DEAD, 0);
    assert!(
        !npc.state_flag_counts.contains_key(&BSF_DEAD),
        "post-respawn → BSF_DEAD counter drained (clear_all_state_flags)",
    );
    let hp = npc.stats.get(HEALTH).unwrap();
    assert_eq!(hp.cur, hp.max, "post-respawn → HP at max");
}

/// `broadcast_movement_type` is NPC-only. Calling it on a player
/// must short-circuit with a warn and emit nothing — without this
/// guard the helper would send a meaningless EMobMovementType byte
/// to the player's client (and to nearby observers via the
/// self+witnesses fanout).
#[tokio::test]
async fn broadcast_movement_type_on_player_is_a_no_op() {
    use cimmeria_entity::cell_entity::MobMovementType;

    let mut mgr = make_mgr_with_dead_npc(None, None);
    // Promote entity 1 (the player witness in the fixture) to a
    // movement-type call recipient. Players normally don't have
    // `last_movement_type`; this test pins that the guard fires
    // BEFORE the cache mutation.
    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::abilities::broadcast_movement_type(
        1,
        Some(MobMovementType::CombatAdvance),
        &tx,
        &mut mgr,
    )
    .await;

    let p = mgr.get_entity(1).unwrap();
    assert!(
        p.last_movement_type.is_none(),
        "player guard must short-circuit before touching the cache",
    );
    assert!(
        drain(&mut rx).is_empty(),
        "no wire messages must be emitted for a player target",
    );
}
