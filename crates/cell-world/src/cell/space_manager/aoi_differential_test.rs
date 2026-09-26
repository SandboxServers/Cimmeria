//! Differential property test for the AoI diff machinery.
//!
//! For every player, every tick, the contract is:
//!
//!     after_witnesses == apply_diff(before_witnesses, events)
//!
//! where `events` is the slice of `EnteredAoI` / `LeftAoI` events
//! emitted by `compute_aoi_changes` for that player on that tick.
//! If a `LeftAoI` is dropped or an `EnteredAoI` is double-fired,
//! the diff-accumulated set drifts from the actual one and this
//! test fails — pointing the reviewer at the exact tick + entity
//! pair where the divergence first surfaces.
//!
//! The walk uses proptest to drive random entity placement and
//! per-tick movement vectors, so a regression that only triggers
//! on a specific spatial pattern (e.g. an entity exiting AoI on the
//! same tick it crosses a grid-cell boundary) gets shrunk to a
//! minimal counterexample rather than buried in an opaque scripted
//! scenario.

use super::SpaceManager;
use crate::cell::messages::CellToBaseMsg;
use cimmeria_common::EntityId;
use proptest::prelude::*;
use std::collections::{HashMap, HashSet};

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

const WORLD_HALF_WIDTH: f32 = 800.0;

/// Initial entity placement: position drawn from a uniform square
/// around the origin, well inside the Agnos world bounds. Strict
/// f32 ranges keep proptest from picking NaN / infinity inputs that
/// the AoI code isn't expected to handle.
fn arb_initial_pos() -> impl Strategy<Value = [f32; 3]> {
    let r = -WORLD_HALF_WIDTH..=WORLD_HALF_WIDTH;
    (r.clone(), r).prop_map(|(x, z)| [x, 0.0, z])
}

/// Per-tick displacement vector. Bounded at ±80 per axis so an entity
/// can move up to ~113 units of euclidean distance in one tick —
/// comfortably above the 100-unit AoI radius, so a single tick can
/// move an entity from outside-AoI to inside or vice versa, exercising
/// both transition arms of the diff. The previous ±50 cap maxed out
/// at √(50²+50²) ≈ 70.7 which couldn't cross the radius in one tick;
/// the comment claimed it could, and the sibling assertion needs
/// outside↔inside crossings to actually exercise the fast-mover path.
fn arb_step() -> impl Strategy<Value = [f32; 3]> {
    let r = -80.0f32..=80.0f32;
    (r.clone(), r).prop_map(|(dx, dz)| [dx, 0.0, dz])
}

proptest! {
    #![proptest_config(ProptestConfig {
        // 16 cases each at 12 entities × 25 ticks = ~4800 ticks of
        // simulation per run. That's enough to surface a missed-
        // LeftAoI bug while keeping the workspace `cargo test`
        // runtime well under one second for this property.
        cases: 16,
        ..ProptestConfig::default()
    })]

    /// For every player, on every tick, the diff-accumulated witness
    /// set (prior + EnteredAoI - LeftAoI) must exactly equal the
    /// post-tick witness set on the entity. A regression that drops
    /// a LeftAoI when an entity exits AoI would surface here as a
    /// stale entry in the diff-accumulated set; a phantom EnteredAoI
    /// would surface as an extra entry.
    #[test]
    fn witness_set_equals_diff_accumulated_set_after_every_tick(
        // 4 players, 8 NPCs — enough to get meaningful AoI churn
        // without runaway runtime.
        initial_positions in proptest::collection::vec(arb_initial_pos(), 12),
        // 25 ticks × 12 entities of step vectors. Pre-generated so
        // the proptest shrinker can shrink the *sequence* — when a
        // failure happens, proptest minimises the trajectory until
        // it finds the smallest one that still triggers the divergence.
        per_tick_steps in proptest::collection::vec(
            proptest::collection::vec(arb_step(), 12),
            25,
        ),
    ) {
        const PLAYER_COUNT: u32 = 4;
        const ENTITY_COUNT: u32 = 12;
        const ID_BASE: u32 = 100;

        let mut mgr = make_manager();

        // Seed all entities. The first PLAYER_COUNT are connected
        // players (have witness sets and are observers); the rest are
        // NPCs (observable but don't observe).
        for i in 0..ENTITY_COUNT {
            let id = ID_BASE + i;
            let pos = initial_positions[i as usize];
            if i < PLAYER_COUNT {
                mgr.create_entity(id, "Agnos", pos, [0.0; 3]).unwrap();
                mgr.connect_entity(id);
            } else {
                mgr.spawn_npc(id, "Agnos", pos, [0.0; 3]).unwrap();
            }
        }

        // Initial tick to populate witness sets — without this, the
        // first per-tick step would observe before/after sets that
        // both equal "everything in range from the seed positions",
        // which is fine but skips the EnteredAoI events from the
        // initial population.
        let _ = mgr.compute_aoi_changes();

        // For each subsequent tick, capture per-player before/after
        // witness sets + the events emitted, and verify the diff
        // contract.
        for (tick_idx, step_set) in per_tick_steps.iter().enumerate() {
            // Snapshot every player's witness set BEFORE this tick.
            let before: HashMap<u32, HashSet<u32>> = (0..PLAYER_COUNT)
                .map(|p| {
                    let id = ID_BASE + p;
                    let wit: HashSet<u32> = mgr
                        .get_entity(id)
                        .map(|e| e.witnesses.iter().map(|EntityId(i)| *i as u32).collect())
                        .unwrap_or_default();
                    (id, wit)
                })
                .collect();

            // Apply the step to every entity.
            for i in 0..ENTITY_COUNT {
                let id = ID_BASE + i;
                let step = step_set[i as usize];
                let cur_pos = mgr.get_entity(id).map(|e| e.position).unwrap();
                let new_pos = [
                    (cur_pos.x + step[0]).clamp(-WORLD_HALF_WIDTH, WORLD_HALF_WIDTH),
                    0.0,
                    (cur_pos.z + step[2]).clamp(-WORLD_HALF_WIDTH, WORLD_HALF_WIDTH),
                ];
                mgr.update_entity_position(id, new_pos, [0, 0, 0], [0.0; 3]);
            }

            let events = mgr.compute_aoi_changes();

            // Apply the per-player events to the before-snapshots and
            // compare to the actual after-tick witness sets.
            for p in 0..PLAYER_COUNT {
                let player_id = ID_BASE + p;
                let mut diffed = before.get(&player_id).cloned().unwrap_or_default();
                for ev in &events {
                    match ev {
                        CellToBaseMsg::EnteredAoI {
                            witness_id,
                            entity_id,
                            ..
                        } if *witness_id == player_id => {
                            diffed.insert(*entity_id);
                        }
                        CellToBaseMsg::LeftAoI {
                            witness_id,
                            entity_id,
                        } if *witness_id == player_id => {
                            diffed.remove(entity_id);
                        }
                        _ => {}
                    }
                }

                let actual: HashSet<u32> = mgr
                    .get_entity(player_id)
                    .map(|e| e.witnesses.iter().map(|EntityId(i)| *i as u32).collect())
                    .unwrap_or_default();

                prop_assert_eq!(
                    &diffed,
                    &actual,
                    "tick {}, player {}: witness set must equal diff-accumulated set. \
                     diff-accumulated had {:?}, actual had {:?}; \
                     missing-from-actual = {:?}, extra-in-actual = {:?}",
                    tick_idx,
                    player_id,
                    diffed,
                    actual,
                    diffed.difference(&actual).collect::<Vec<_>>(),
                    actual.difference(&diffed).collect::<Vec<_>>()
                );
            }
        }
    }
}
