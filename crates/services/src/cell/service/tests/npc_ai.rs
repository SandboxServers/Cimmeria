//! `npc_ai_tick` state-machine transitions: Fighting → Idle on empty
//! threat, Fighting → Leashing past `LEASH_DISTANCE`, dead-target prune,
//! stationary no-pathfind, leash snap-to-spawn / heal / cooldown clear.
//!
//! Uses a non-instanced `Castle` fixture rather than the parent
//! `make_test_space_mgr` (Castle_CellBlock, instanced) so the NPC and
//! its threat targets co-locate in the same space — otherwise
//! `has_line_of_sight` falls back to true and `find_path` to None,
//! masking what the in-range / LOS branches of `npc_ai_fight` actually do.

use crate::cell::space_manager::SpaceManager;
use cimmeria_common::Vector3;
use cimmeria_entity::cell_entity::AiState;
use cimmeria_entity::stats::HEALTH;
use tokio::sync::mpsc;

/// Build a non-instanced "Castle" space and seed an NPC at id=200 in
/// AiState::Fighting with the given spawn position. Returns the
/// SpaceManager. Caller layers in the threat list and ability defs.
fn make_ai_fixture(npc_spawn: [f32; 3], npc_pos: [f32; 3]) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(200, "Castle", npc_pos, [0.0; 3]).unwrap();
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.is_player = false;
        npc.class_id = 0x04; // SGWMob — required for all_npc_entity_ids()
        npc.ai_state = AiState::Fighting;
        npc.spawn_position = Some(Vector3::new(npc_spawn[0], npc_spawn[1], npc_spawn[2]));
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    mgr
}

/// Fighting NPC with an empty threat list resets to Idle. The
/// regression guard for the early-return that re-enables this NPC
/// to be re-aggrod by the next attacker.
#[tokio::test]
async fn npc_ai_fighting_with_empty_threat_resets_to_idle() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;
    assert!(matches!(
        mgr.get_entity(200).unwrap().ai_state,
        AiState::Idle
    ));
}

/// Target sitting past `LEASH_DISTANCE` (50.0) from the NPC's spawn
/// triggers AiState::Leashing and clears the threat list. Pin the
/// transition so a refactor that drops the leash branch can't
/// silently let mobs path across the whole zone.
#[tokio::test]
async fn npc_ai_target_beyond_leash_distance_triggers_leashing() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    // Target player at distance 100 from spawn (LEASH_DISTANCE=50).
    mgr.create_entity(100, "Castle", [100.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        p.player_id = Some(1);
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
    }
    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;
    let npc = mgr.get_entity(200).unwrap();
    assert!(matches!(npc.ai_state, AiState::Leashing));
    assert!(
        npc.threat_list.is_empty(),
        "leashing must clear the threat list"
    );
}

/// A dead target is removed from the threat list while the OTHER
/// live threats remain — so the next tick has a target to pick. Pin
/// both the surgical removal of just the dead one AND the AI staying
/// Fighting. An implementation that scans through and bulk-clears
/// the whole list on first dead encounter would silently break
/// multi-target combat.
#[tokio::test]
async fn npc_ai_dead_target_is_removed_but_other_threats_remain() {
    let mut mgr = make_ai_fixture([0.0; 3], [10.0, 0.0, 0.0]);
    // Dead target close to NPC.
    mgr.create_entity(100, "Castle", [11.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    // Live secondary target, also near the NPC.
    mgr.create_entity(101, "Castle", [12.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 0, 100);
            h.clear_dirty();
        }
    }
    if let Some(p) = mgr.get_entity_mut(101) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        // 100 has the highest threat (so it's selected first → dead → removed),
        // 101 stays as the next-tick target.
        npc.threat_list.insert(100, 5.0);
        npc.threat_list.insert(101, 2.0);
    }
    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;
    let npc = mgr.get_entity(200).unwrap();
    assert!(
        !npc.threat_list.contains_key(&100),
        "dead target must be removed from threat list"
    );
    assert_eq!(
        npc.threat_list.get(&101),
        Some(&2.0),
        "live secondary threat must survive the dead-target prune"
    );
    assert!(
        matches!(npc.ai_state, AiState::Fighting),
        "AI stays Fighting so next tick picks up another target"
    );
}

/// Stationary NPC out of attack range / LOS does NOT pathfind. Pin
/// so a refactor that runs the pathfinder unconditionally doesn't
/// turn turrets into chasers.
#[tokio::test]
async fn npc_ai_stationary_does_not_pathfind_when_out_of_range() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    // Target at distance 40 (within LEASH=50 but past NPC_ATTACK_RANGE=30).
    mgr.create_entity(100, "Castle", [40.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, 1.0);
        npc.is_stationary = true;
    }
    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;
    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.nav_path.is_empty(),
        "stationary NPC must not populate a nav path; got {:?}",
        npc.nav_path
    );
    assert!(
        matches!(npc.ai_state, AiState::Fighting),
        "stationary NPC stays Fighting; only the leash branch transitions"
    );
}

/// Leashing tick: NPC snaps back to spawn, health restores to max,
/// AI resets to Idle, threat list clears, cooldowns clear. The
/// canary for the "leash never returns to Idle" hang.
#[tokio::test]
async fn npc_ai_leashing_snaps_to_spawn_restores_health_and_idles() {
    let mut mgr = make_ai_fixture([0.0; 3], [40.0, 0.0, 40.0]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Leashing;
        // Seed pre-leash state: a damaged NPC with stale threat targets
        // and an active cooldown carried over from the Fighting phase.
        // The leash tick must wipe ALL of these — pin each one so a
        // regression that only handles ai_state but leaves threat or
        // cooldowns dangling gets caught.
        npc.threat_list.insert(100, 5.0);
        npc.threat_list.insert(101, 2.0);
        npc.abilities
            .start_ability_cooldown(592, std::time::Duration::from_secs(60));
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 5, 100); // damaged
            h.clear_dirty();
        }
    }
    let (tx, _rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;
    let npc = mgr.get_entity(200).unwrap();
    assert!(matches!(npc.ai_state, AiState::Idle));
    assert_eq!(
        npc.position,
        Vector3::new(0.0, 0.0, 0.0),
        "leash must snap NPC back to spawn"
    );
    assert_eq!(
        npc.stats.get(HEALTH).unwrap().cur,
        100,
        "leash must restore health to max"
    );
    assert!(
        npc.threat_list.is_empty(),
        "leash must wipe pre-seeded threat targets"
    );
    assert!(
        !npc.abilities.is_on_cooldown(592),
        "leash must clear pre-seeded cooldown"
    );
}
