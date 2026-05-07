//! `npc_ai_tick` state-machine transitions: Fighting → Idle on empty
//! threat, Fighting → Leashing past `LEASH_DISTANCE`, dead-target prune,
//! stationary no-pathfind, leash snap-to-spawn / heal / cooldown clear.
//!
//! Uses a non-instanced `Castle` fixture rather than the parent
//! `make_test_space_mgr` (Castle_CellBlock, instanced) so the NPC and
//! its threat targets co-locate in the same space — otherwise
//! `has_line_of_sight` falls back to true and `find_path` to None,
//! masking what the in-range / LOS branches of `npc_ai_fight` actually do.

use crate::cell::messages::CellToBaseMsg;
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

/// Fighting NPC with three live threats must pick the highest-threat
/// target for attack. The dead-target sibling test only seeds two
/// threats and the dead one is pruned, so the survivor is selected by
/// default — it never exercises the `max_by` branch. This test pins
/// that the top threat is chosen by varying the geometry: only the
/// top-threat target (101) is in NPC_ATTACK_RANGE; 100 and 102 are
/// past 30. With pre-seeded `nav_path`, the in-range attack branch
/// clears it, and the out-of-range pathing branch leaves it set. So:
/// max_by correct → 101 picked → in range → nav_path empty.
/// max_by flipped → 100 or 102 picked → out of range → nav_path stays.
#[tokio::test]
async fn npc_ai_fight_picks_top_threat_among_multiple_live_targets() {
    use cimmeria_common::Vector3;

    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);

    // Position so only the top-threat target is in NPC_ATTACK_RANGE (30):
    //   100 (threat 2.0)  → distance 40 (out of range, within LEASH=50)
    //   102 (threat 5.0)  → distance 35 (out of range, within LEASH=50)
    //   101 (threat 10.0) → distance 10 (in range)
    for &(eid, pos) in &[
        (100, [40.0, 0.0, 0.0]),
        (101, [10.0, 0.0, 0.0]),
        (102, [-35.0, 0.0, 0.0]),
    ] {
        mgr.create_entity(eid, "Castle", pos, [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(eid) {
            p.is_player = true;
            if let Some(h) = p.stats.get_mut(HEALTH) {
                h.update(0, 100, 100);
                h.clear_dirty();
            }
        }
    }

    if let Some(npc) = mgr.get_entity_mut(200) {
        // Insert threats so the highest is NOT first in HashMap order.
        // max_by must still return 101 (threat 10.0).
        npc.threat_list.insert(100, 2.0);
        npc.threat_list.insert(102, 5.0);
        npc.threat_list.insert(101, 10.0);
        // Pre-seed nav_path so the assertion can distinguish:
        //  - in-range branch explicitly calls `nav_path.clear()`
        //  - out-of-range pathing branch leaves it set if find_path
        //    returns None (no navmesh in the test fixture).
        npc.nav_path.push_back(Vector3::new(99.0, 0.0, 99.0));
    }

    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        matches!(npc.ai_state, AiState::Fighting),
        "NPC must stay Fighting with live threats"
    );
    assert!(
        npc.nav_path.is_empty(),
        "max_by must select the in-range top-threat target (101); reverting \
         to min_by picks an out-of-range target and leaves the pre-seeded \
         nav_path in place"
    );
}

/// Single-target NaN: `max_by` over a 1-element iterator never compares,
/// so this only proves the iterator path doesn't panic on a NaN entry.
/// The actual `partial_cmp(NaN, _).unwrap_or(Equal)` fallback is pinned
/// by the multi-target sibling below.
#[tokio::test]
async fn npc_ai_fight_single_nan_target_does_not_panic() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);

    mgr.create_entity(100, "Castle", [5.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(100) {
        p.is_player = true;
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }

    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.threat_list.insert(100, f32::NAN);
    }

    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        matches!(npc.ai_state, AiState::Fighting),
        "single NaN-threat target must not panic and stays Fighting"
    );
}

/// Multi-target NaN: with NaN AND a finite threat in the list, `max_by`
/// actually invokes `partial_cmp(NaN, finite)` which returns `None`.
/// The fallback `unwrap_or(Ordering::Equal)` keeps the iterator going.
/// A refactor that swapped the unwrap_or for `unwrap()` would panic on
/// this comparison and fail the test.
#[tokio::test]
async fn npc_ai_fight_nan_in_threat_list_with_other_targets_does_not_panic() {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);

    for &(eid, pos) in &[(100, [5.0, 0.0, 0.0]), (101, [10.0, 0.0, 0.0])] {
        mgr.create_entity(eid, "Castle", pos, [0.0; 3]).unwrap();
        if let Some(p) = mgr.get_entity_mut(eid) {
            p.is_player = true;
            if let Some(h) = p.stats.get_mut(HEALTH) {
                h.update(0, 100, 100);
                h.clear_dirty();
            }
        }
    }

    if let Some(npc) = mgr.get_entity_mut(200) {
        // Mixed: one NaN, one finite. max_by must invoke partial_cmp on
        // the NaN; the unwrap_or(Equal) fallback prevents the panic.
        npc.threat_list.insert(100, f32::NAN);
        npc.threat_list.insert(101, 5.0);
    }

    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        matches!(npc.ai_state, AiState::Fighting),
        "NaN-vs-finite comparison must fall through Equal without panicking"
    );
}

/// Leashing tick emits onStatUpdate (method 20) and onStateFieldUpdate
/// (method 19) to witnesses. Without a witness in the NPC's space the
/// calls hit the empty-witness branch and silently drop. This test adds
/// a player witness, drains rx, and asserts presence + ordering.
#[tokio::test]
async fn npc_ai_leash_emits_stat_update_then_state_field_to_witnesses() {
    let mut mgr = make_ai_fixture([0.0; 3], [40.0, 0.0, 40.0]);

    // Add a player witness in the same space.
    mgr.create_entity(1, "Castle", [5.0, 0.0, 5.0], [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
        if let Some(h) = p.stats.get_mut(HEALTH) {
            h.update(0, 100, 100);
            h.clear_dirty();
        }
    }
    mgr.connect_entity(1);
    // Compute AoI so the player witnesses NPC 200.
    let _ = mgr.compute_aoi_changes();

    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.ai_state = AiState::Leashing;
        if let Some(h) = npc.stats.get_mut(HEALTH) {
            h.update(0, 5, 100); // damaged
            h.clear_dirty();
        }
    }

    let (tx, mut rx) = mpsc::channel(16);
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    // Collect WitnessEntityMethod packets for NPC 200.
    let mut witness_methods: Vec<u16> = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::WitnessEntityMethod {
            entity_id,
            method_index,
            ..
        } = msg
        {
            if entity_id == 200 {
                witness_methods.push(method_index);
            }
        }
    }

    assert_eq!(
        witness_methods.len(),
        2,
        "leash tick must emit exactly 2 witness methods for NPC 200"
    );
    assert_eq!(
        witness_methods[0], 20,
        "first witness method must be onStatUpdate (20)"
    );
    assert_eq!(
        witness_methods[1], 19,
        "second witness method must be onStateFieldUpdate (19)"
    );
}
