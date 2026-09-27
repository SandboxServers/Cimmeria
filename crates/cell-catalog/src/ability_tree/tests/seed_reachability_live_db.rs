//! The FINAL v2 seed is trainable under the AT-03 gates (AT-05b).
//!
//! `seed_live_db` pins the seed's shape; this file checks that the shape
//! and the gates agree. A node whose `required_branch_points` or
//! prerequisites cannot be met by its unlock level under the v2 economy is
//! a button the trainer shows greyed forever.
//!
//! The economy (D-AT02, `db/sgw/Players/Seed/sgw_player.sql`): a character
//! holds 1 point at level 1 and earns 1 per level, so an unspent character
//! at level `L` has exactly `L` points. Only trainer purchases count as
//! spend (D-AT03), so the simulation starts with nothing known.

use std::collections::HashSet;

use super::super::*;
use crate::test_support::require_db_or_skip;

/// The seeded archetypes, as `EArchetype` positions (Soldier..Sholva).
const SEEDED_ARCHETYPES: std::ops::RangeInclusive<i32> = 1..=7;

fn ctx<'a>(
    catalog: &'a AbilityTreeCatalog,
    known: &'a HashSet<i32>,
    archetype_id: i32,
    ability_id: i32,
    level: i32,
    spent: i32,
    points: i32,
    offered: &'a [i32],
) -> TrainContext<'a> {
    TrainContext {
        catalog,
        ability_id,
        ability_exists: true,
        player_id: Some(0x7030_05b0),
        archetype_id: Some(archetype_id),
        level,
        known,
        tree_points_spent: spent,
        training_points: points,
        // At the seeded debug trainer (list 1 offers every node, D-AT06;
        // `seed_trainer_rows_and_tree_rows_match` proves it), in range, so
        // only the node and spend gates decide.
        trainer: TrainerPin::Trainer {
            offered,
            in_range: true,
        },
    }
}

/// Every node of `arch`, as the debug trainer offers it.
fn offered_by_debug_trainer(catalog: &AbilityTreeCatalog, arch: i32) -> Vec<i32> {
    catalog.tree(arch).iter().map(|n| n.ability_id).collect()
}

/// `target` plus every node it needs through its prerequisites.
fn prerequisite_closure(catalog: &AbilityTreeCatalog, arch: i32, target: i32) -> HashSet<i32> {
    let mut closure = HashSet::new();
    let mut stack = vec![target];
    while let Some(id) = stack.pop() {
        if closure.insert(id) {
            if let Some(node) = catalog.node(arch, id) {
                stack.extend(node.prerequisites.iter().copied());
            }
        }
    }
    closure
}

/// Can a fresh character reach `target` at exactly its unlock level, with
/// the points the economy gives by then? Buys greedily: the target's own
/// prerequisite path first, then the cheapest-to-open filler node to raise
/// the archetype-wide spend. Every seeded node costs what `evaluate_train`
/// says, so the simulation follows the gates exactly. Returns the rejection
/// that stopped it.
fn reach_at_unlock_level(
    catalog: &AbilityTreeCatalog,
    arch: i32,
    target: &TreeNode,
) -> Result<(), TrainReject> {
    let offered = offered_by_debug_trainer(catalog, arch);
    let level = target.level;
    let path = prerequisite_closure(catalog, arch, target.ability_id);
    let mut known = HashSet::new();
    let (mut spent, mut points) = (0, level);
    loop {
        match evaluate_train(&ctx(
            catalog,
            &known,
            arch,
            target.ability_id,
            level,
            spent,
            points,
            &offered,
        )) {
            Ok(_) => return Ok(()),
            Err(TrainReject::SpendGate { .. }) | Err(TrainReject::MissingPrerequisite { .. }) => {}
            Err(other) => return Err(other),
        }
        let buyable = catalog.tree(arch).iter().filter(|n| {
            n.ability_id != target.ability_id
                && evaluate_train(&ctx(
                    catalog,
                    &known,
                    arch,
                    n.ability_id,
                    level,
                    spent,
                    points,
                    &offered,
                ))
                .is_ok()
        });
        let next = buyable.min_by_key(|n| {
            (
                !path.contains(&n.ability_id),
                n.skill_point_cost,
                n.level,
                n.tree_index,
                n.node_order,
            )
        });
        let Some(next) = next else {
            return evaluate_train(&ctx(
                catalog,
                &known,
                arch,
                target.ability_id,
                level,
                spent,
                points,
                &offered,
            ))
            .map(|_| ());
        };
        known.insert(next.ability_id);
        spent += next.skill_point_cost;
        points -= next.skill_point_cost;
    }
}

/// Every seeded node can be trained by a character at its unlock level who
/// has spent that level's points on the way. Also proves no node is
/// orphaned by an impossible spend gate or an unreachable prerequisite.
#[tokio::test]
async fn seed_every_node_is_trainable_at_its_unlock_level_under_the_v2_economy() {
    let pool = require_db_or_skip!();
    let catalog = AbilityTreeCatalog::load(&pool).await.expect("catalog load");

    let mut checked = 0;
    let mut unreachable = Vec::new();
    for arch in SEEDED_ARCHETYPES {
        for node in catalog.tree(arch) {
            checked += 1;
            if let Err(reject) = reach_at_unlock_level(&catalog, arch, node) {
                unreachable.push((arch, node.ability_id, node.level, reject));
            }
        }
    }
    assert_eq!(checked, catalog.len(), "every seeded node was checked");
    assert!(
        unreachable.is_empty(),
        "nodes a character cannot train at their unlock level \
         (archetype, ability, level, rejection): {unreachable:?}"
    );
}

/// A new level-1 character with its single point can open every branch:
/// each root needs no spend, no prerequisite and at most one point.
#[tokio::test]
async fn seed_every_branch_root_is_trainable_by_a_new_character() {
    let pool = require_db_or_skip!();
    let catalog = AbilityTreeCatalog::load(&pool).await.expect("catalog load");
    let known = HashSet::new();

    let mut roots = 0;
    let mut locked = Vec::new();
    for arch in SEEDED_ARCHETYPES {
        let offered = offered_by_debug_trainer(&catalog, arch);
        for root in catalog.tree(arch).iter().filter(|n| n.is_branch_root) {
            roots += 1;
            if let Err(reject) = evaluate_train(&ctx(
                &catalog,
                &known,
                arch,
                root.ability_id,
                1,
                0,
                1,
                &offered,
            )) {
                locked.push((arch, root.ability_id, reject));
            }
        }
    }
    assert_eq!(roots, 21, "one root per branch");
    assert!(
        locked.is_empty(),
        "roots a level-1 character with 1 point cannot train \
         (archetype, ability, rejection): {locked:?}"
    );
}
