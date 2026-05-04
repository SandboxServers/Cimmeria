//! AoI churn smoke: a player walks past many NPCs and we assert the
//! per-pair enter/leave bookkeeping stays balanced, the witness set
//! never blows up, and disconnect cleans up the player without
//! orphaning entries in the global `entity_space` map.
//!
//! What this catches that the per-tick `tests.rs` cases don't:
//!
//! - A leak in the per-pair ledger (player witness set never decays
//!   even when the entity moves out of range): would surface as a
//!   witness set growing unbounded across the walk.
//! - A double-fire of `EnteredAoI` mid-walk if witness-set
//!   persistence is broken: would surface as `entered_count > 1`
//!   for the same (player, NPC) pair.
//! - NPC entries leaking out of `entity_space` on player disconnect:
//!   would surface as the post-disconnect entity_space size missing
//!   NPCs that should have stayed.

use super::SpaceManager;
use crate::cell::messages::CellToBaseMsg;
use std::collections::HashMap;

const TEST_SPACES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" Instanced="false" MinX="-2400" MaxX="2200" MinY="-3200" MaxY="2800" />
</Spaces>"#;

const TEST_CELL_SPACES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Spaces>
    <Space WorldName="Agnos" />
</Spaces>"#;

fn make_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(TEST_SPACES_XML).unwrap();
    mgr.create_startup_spaces(TEST_CELL_SPACES_XML).unwrap();
    mgr
}

/// Drive a 50-NPC + 1-player AoI scenario across a 100-tick walk.
///
/// Layout: NPCs are seeded at (X = i*20, 0, 0) for i in 0..50, so the
/// line spans X = 0..980. The player starts at X = -200 (outside every
/// NPC's 100-unit AoI radius) and walks to X = 1200 across 100 ticks
/// of 14 units each — passing every NPC and exiting their range on
/// the far side. With aoi_radius = 100, each NPC enters the player's
/// witness set when the player crosses X = NPC_X - 100 and leaves
/// when the player crosses X = NPC_X + 100.
///
/// Assertions:
///
/// - Every (player, NPC) pair enters AoI exactly once and leaves
///   exactly once. A double-enter would mean the witness-set diff
///   is broken; a double-leave would mean the leaver bookkeeping
///   is broken; missing either side means a tick was dropped.
/// - Peak witness set never exceeds 11 NPCs (the spatial bound is
///   approximately `aoi_diameter / spacing`, plus one for the
///   inclusive radius edge) — a regression that fails to expire
///   entries from the witness set would let it grow unbounded.
/// - Per-tick witness set monotonically tracks the spatial reality:
///   after the walk completes (player at X = 1200), witness set is
///   empty (every NPC behind us beyond aoi_radius).
#[test]
fn aoi_churn_walk_balances_enters_and_leaves_with_bounded_witness_set() {
    const NPC_COUNT: u32 = 50;
    const NPC_SPACING: f32 = 20.0;
    const TICK_COUNT: u32 = 100;
    const PLAYER_ID: u32 = 1;
    const NPC_ID_BASE: u32 = 100;
    const PLAYER_START_X: f32 = -200.0;
    const PLAYER_END_X: f32 = 1200.0;
    // aoi_radius is the CellEntity default of 100.0 — see
    // crates/entity/src/cell_entity/mod.rs:329.

    let mut mgr = make_manager();

    // Seed NPCs along the line. spawn_npc creates them as non-player
    // entities with class_id=0x04 (SGWMob) and the default aoi_radius.
    for i in 0..NPC_COUNT {
        let pos = [i as f32 * NPC_SPACING, 0.0, 0.0];
        mgr.spawn_npc(NPC_ID_BASE + i, "Agnos", pos, [0.0; 3])
            .unwrap();
    }
    let space_id_at_start = *mgr.entity_space.get(&NPC_ID_BASE).unwrap();
    assert_eq!(
        mgr.entity_space.len(),
        NPC_COUNT as usize,
        "fixture sanity: 50 NPCs registered in entity_space"
    );

    // Spawn + connect the player at the starting X.
    mgr.create_entity(PLAYER_ID, "Agnos", [PLAYER_START_X, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(PLAYER_ID);

    // Per-NPC enter/leave counters across the walk.
    let mut enters: HashMap<u32, u32> = HashMap::new();
    let mut leaves: HashMap<u32, u32> = HashMap::new();
    let mut peak_witnesses: usize = 0;

    let step = (PLAYER_END_X - PLAYER_START_X) / TICK_COUNT as f32;
    for tick in 1..=TICK_COUNT {
        let x = PLAYER_START_X + step * tick as f32;
        mgr.update_entity_position(PLAYER_ID, [x, 0.0, 0.0], [0, 0, 0], [0.0; 3]);
        let events = mgr.compute_aoi_changes();
        for ev in &events {
            match ev {
                CellToBaseMsg::EnteredAoI {
                    witness_id,
                    entity_id,
                    ..
                } if *witness_id == PLAYER_ID => {
                    *enters.entry(*entity_id).or_insert(0) += 1;
                }
                CellToBaseMsg::LeftAoI {
                    witness_id,
                    entity_id,
                } if *witness_id == PLAYER_ID => {
                    *leaves.entry(*entity_id).or_insert(0) += 1;
                }
                _ => {}
            }
        }

        let witness_count = mgr
            .get_entity(PLAYER_ID)
            .map(|e| e.witnesses.len())
            .unwrap_or(0);
        peak_witnesses = peak_witnesses.max(witness_count);
        // Bounded witness-set guard: with NPCs every 20 units and a
        // 100-unit radius, at most ~10 NPCs are in range at any time.
        // Allow a small margin (11) for the inclusive boundary.
        assert!(
            witness_count <= 11,
            "tick {tick}: witness set blew past the spatial bound (got {witness_count}, expected <= 11) — likely a leaked entry"
        );
    }

    // Every NPC should have entered and left the player's AoI exactly
    // once over the full traversal. Anything > 1 means the diff is
    // double-firing; anything < 1 means a transition was dropped.
    for i in 0..NPC_COUNT {
        let npc_id = NPC_ID_BASE + i;
        assert_eq!(
            enters.get(&npc_id).copied().unwrap_or(0),
            1,
            "NPC {npc_id} should have entered AoI exactly once across the walk"
        );
        assert_eq!(
            leaves.get(&npc_id).copied().unwrap_or(0),
            1,
            "NPC {npc_id} should have left AoI exactly once across the walk"
        );
    }
    assert!(
        peak_witnesses > 0,
        "fixture sanity: the player must have observed at least one NPC mid-walk"
    );

    // After the full traversal the player is at X = 1200, beyond
    // every NPC's range — witness set must be empty. A regression that
    // never expires entries would surface as a non-empty set here.
    let final_witnesses = mgr
        .get_entity(PLAYER_ID)
        .map(|e| e.witnesses.len())
        .unwrap_or(usize::MAX);
    assert_eq!(
        final_witnesses, 0,
        "player at X = 1200 is past every NPC; witness set must be empty after final tick"
    );

    // ── Disconnect cleanup ──────────────────────────────────────────
    // Player exits the world. The async disconnect path emits LeftAoI
    // packets to observers (none, in this fixture — only the player
    // is connected), then destroys the player entity. NPCs persist in
    // the non-instanced space and remain in entity_space.
    let (tx, _rx) = tokio::sync::mpsc::channel(64);
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(mgr.disconnect_entity(PLAYER_ID, &tx));

    assert!(
        !mgr.entity_space.contains_key(&PLAYER_ID),
        "player must be removed from entity_space on disconnect"
    );
    // All 50 NPCs must still be present — the non-instanced space
    // does NOT destroy on player exit. A regression that nuked NPC
    // mappings on disconnect would shrink entity_space here.
    assert_eq!(
        mgr.entity_space.len(),
        NPC_COUNT as usize,
        "NPC entity_space mappings must persist after the last player disconnects from a non-instanced space"
    );
    for i in 0..NPC_COUNT {
        let npc_id = NPC_ID_BASE + i;
        assert_eq!(
            mgr.entity_space.get(&npc_id).copied(),
            Some(space_id_at_start),
            "NPC {npc_id} must still map to its original space"
        );
    }
}
