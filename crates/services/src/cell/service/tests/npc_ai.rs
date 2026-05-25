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

// ── set_aggression auto-aggro tests ──────────────────────────────────────

/// Castle test space + one NPC at the origin and one player. NPC defaults
/// to `class_id = 0x04` so `npc_ai_tick` sees it; the caller flips
/// faction / aggression as needed. The NPC's default ability bucket is
/// left intact (Pistol Shot 592) so the fight path doesn't wedge on an
/// empty selector.
fn make_aggression_fixture(
    npc_id: u32,
    npc_faction: u8,
    player_id: u32,
    player_pos: [f32; 3],
) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.spawn_npc(npc_id, "Castle", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(npc_id) {
        npc.faction = npc_faction;
        npc.ai_state = AiState::Idle;
    }
    mgr.create_entity(player_id, "Castle", player_pos, [0.0; 3])
        .unwrap();
    if let Some(p) = mgr.get_entity_mut(player_id) {
        p.is_player = true;
        p.player_id = Some(player_id as i32);
        p.faction = 0;
    }
    mgr.connect_entity(player_id);
    let _ = mgr.compute_aoi_changes();
    mgr
}

/// `aggression > 0` on an Idle NPC with an opposing-faction player in AoI
/// transitions the NPC to Fighting on the next tick with the player on
/// the threat list. Bug shape: the previous `set_aggression` wrote a
/// property nothing read, so the drone sat idle forever.
#[tokio::test]
async fn idle_npc_with_aggression_aggros_opposing_player() {
    let mut mgr = make_aggression_fixture(200_001, 10, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_001) {
        npc.aggression = 1;
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200_001).unwrap();
    assert_eq!(
        npc.ai_state,
        AiState::Fighting,
        "aggression=1 with opposing-faction witness must transition to Fighting",
    );
    assert!(
        npc.threat_list.contains_key(&1),
        "player must be on NPC threat list after auto-aggro",
    );
}

/// `aggression == 0` (default) keeps the NPC Idle even with a hostile
/// player in AoI. The outer-tick filter — not an inner defensive check —
/// is what enforces this baseline; the test goes through `npc_ai_tick`
/// to exercise the actual production path.
#[tokio::test]
async fn idle_npc_without_aggression_stays_idle() {
    let mut mgr = make_aggression_fixture(200_002, 10, 1, [5.0, 0.0, 0.0]);
    assert_eq!(mgr.get_entity(200_002).unwrap().aggression, 0);
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200_002).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle);
    assert!(npc.threat_list.is_empty());
}

/// `aggression > 0` but the witness shares the NPC's faction → no aggro.
/// Pins that the faction-equality check is in the right direction (skip
/// same-faction, target opposing).
#[tokio::test]
async fn aggression_skips_same_faction_witnesses() {
    let mut mgr = make_aggression_fixture(200_003, 0, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_003) {
        npc.aggression = 1;
    }
    let (tx, _rx) = mpsc::channel(16);

    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    let npc = mgr.get_entity(200_003).unwrap();
    assert_eq!(npc.ai_state, AiState::Idle, "same faction must not aggro");
    assert!(npc.threat_list.is_empty());
}

// ── choose_npc_ability selector tests ────────────────────────────────────

/// Selector: NPC with two abilities, the lower-id one on cooldown →
/// returns the higher-id one. Pins that the cooldown gate participates
/// in selection (without it, the selector always returns the first
/// sorted ability and the test would still pass for the wrong reason).
#[tokio::test]
async fn selector_skips_cooling_ability() {
    let mut mgr = make_aggression_fixture(200_004, 10, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_004) {
        npc.abilities
            .remove_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
        npc.abilities.add_ability(221);
        npc.abilities.add_ability(592);
        // Sort order is ascending — put the first one on cooldown so the
        // selector must skip past it.
        npc.abilities
            .start_ability_cooldown(221, std::time::Duration::from_secs(10));
    }

    let chosen = crate::cell::service::npc_ai::choose_npc_ability(200_004, &mgr);
    assert_eq!(chosen, Some(592), "cooling 221 → selector must pick 592");
}

/// Selector: NPC with three abilities, the two lowest-id ones on
/// cooldown → returns the third. Pins selection across a non-trivial
/// bucket (the single-cooling-of-two case is the minimal partition; a
/// multi-cooling case verifies the `find` walks past every cooling
/// ability rather than stopping at the first).
#[tokio::test]
async fn selector_skips_multiple_cooling_abilities() {
    let mut mgr = make_aggression_fixture(200_008, 10, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_008) {
        npc.abilities
            .remove_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
        npc.abilities.add_ability(100);
        npc.abilities.add_ability(200);
        npc.abilities.add_ability(300);
        // Sort order is ascending — cool 100 and 200, leave 300 fresh.
        npc.abilities
            .start_ability_cooldown(100, std::time::Duration::from_secs(10));
        npc.abilities
            .start_ability_cooldown(200, std::time::Duration::from_secs(10));
    }

    let chosen = crate::cell::service::npc_ai::choose_npc_ability(200_008, &mgr);
    assert_eq!(
        chosen,
        Some(300),
        "selector must walk past 100 and 200 (both cooling) to reach 300",
    );
}

/// Selector: every known ability is on cooldown → `None` so the AI tick
/// holds fire rather than misfiring against an unfireable ability.
#[tokio::test]
async fn selector_returns_none_when_all_cooling() {
    let mut mgr = make_aggression_fixture(200_005, 10, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_005) {
        npc.abilities
            .remove_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
        npc.abilities.add_ability(221);
        npc.abilities
            .start_ability_cooldown(221, std::time::Duration::from_secs(10));
    }

    let chosen = crate::cell::service::npc_ai::choose_npc_ability(200_005, &mgr);
    assert_eq!(chosen, None, "all cooling → hold fire");
}

/// Selector: NPC with no abilities at all (misconfigured template) falls
/// back to `NPC_DEFAULT_ABILITY` so the tick never wedges silently.
#[tokio::test]
async fn selector_falls_back_to_default_when_no_abilities() {
    let mut mgr = make_aggression_fixture(200_006, 10, 1, [5.0, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200_006) {
        npc.abilities
            .remove_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
    }

    let chosen = crate::cell::service::npc_ai::choose_npc_ability(200_006, &mgr);
    assert_eq!(
        chosen,
        Some(crate::cell::combat::NPC_DEFAULT_ABILITY),
        "empty bucket → fallback to NPC_DEFAULT_ABILITY",
    );
}

/// Selector: ability with `required_ammo > 0` is still picked — NPCs have
/// infinite ammo (the ammo gate at the dispatch site is player-only).
/// Pinning this guards against re-introducing the regression where every
/// NPC carrying Pistol Shot 592 (`required_ammo = 1`) silently held
/// fire forever. The test seeds the ability def with `required_ammo = 1`
/// directly — without this seed, `space_mgr.ability_defs.get(&592)`
/// would return `None` and a re-introduced ammo gate would silently
/// fall through to `required_ammo = 0` and pass.
#[tokio::test]
async fn selector_picks_ammo_bearing_ability_for_npc() {
    use cimmeria_entity::abilities::AbilityDef;

    let mut mgr = make_aggression_fixture(200_007, 10, 1, [5.0, 0.0, 0.0]);
    mgr.ability_defs.insert(
        crate::cell::combat::NPC_DEFAULT_ABILITY,
        AbilityDef {
            ability_id: crate::cell::combat::NPC_DEFAULT_ABILITY,
            name: "Pistol Shot".to_string(),
            cooldown: 1.0,
            warmup: 0.0,
            flags: 0,
            is_ranged: true,
            min_range: 0,
            max_range: 30,
            target_type_id: 0,
            effect_ids: vec![],
            moniker_ids: vec![],
            // Mirror the seeded DB value — Pistol Shot is required_ammo=1.
            required_ammo: 1,
            event_set_id: None,
            velocity: 0.0,
        },
    );

    let chosen = crate::cell::service::npc_ai::choose_npc_ability(200_007, &mgr);
    assert_eq!(
        chosen,
        Some(crate::cell::combat::NPC_DEFAULT_ABILITY),
        "selector must pick 592 even with required_ammo > 0 — NPCs have \
         infinite ammo and the dispatch-site ammo check is player-only",
    );
}

// ──────────────────────────────────────────────────────────────────────
// Issue #304 negative-logging regression guard: handle_use_ability
// returning false.
//
// The bug shape: NPC AI ticks but the ability never fires (mob picked
// an ability that the pre-consume guard then rejected on cooldown, no
// ammo, or any other reason). Pre-#304 this was silent — the mob would
// appear stuck. The guard pins WARN level + reason field so removing
// either trips the test.
// ──────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn npc_ai_fight_warns_when_handle_use_ability_returns_false() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    // Stage: NPC in Fighting with a live in-range player on the threat
    // list. The NPC has NO ability_defs loaded at all, so
    // `handle_use_ability` will reject the call ("ability missing")
    // and return false — exactly the rejection shape the WARN guards.
    let mut mgr = make_ai_fixture([0.0; 3], [10.0, 0.0, 0.0]);
    mgr.create_entity(100, "Castle", [11.0, 0.0, 0.0], [0.0; 3])
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
        // Stage the rejection shape that triggers the WARN:
        // - `npc.abilities` is empty (no `add_ability` call).
        // - `choose_npc_ability` documents an empty-bucket fallback to
        //   NPC_DEFAULT_ABILITY ("so a misconfigured template doesn't
        //   wedge silently") — so the selector returns Some.
        // - `handle_use_ability` then rejects on the
        //   `entity.abilities.has_ability(ability_id)` check (the
        //   entity doesn't know the fallback ability) and returns
        //   false. That false-return is the seam the new WARN guards.
    }

    let capture = LogCapture::install();
    let (tx, _rx) = mpsc::channel(8);
    crate::cell::service::npc_ai::npc_ai_tick(&tx, &mut mgr).await;

    assert!(
        capture
            .find_event(
                Level::WARN,
                "NPC AI: attack tick produced no ability fire",
                "handle_use_ability_returned_false"
            )
            .is_some(),
        "issue #304: NPC AI must WARN when handle_use_ability returns false; \
         reverting to bare `.await` (ignoring the bool) breaks mob-stuck \
         diagnosability. Captured: {:#?}",
        capture.all()
    );
}
