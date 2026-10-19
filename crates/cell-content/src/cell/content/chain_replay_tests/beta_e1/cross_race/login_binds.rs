//! Beta Site E1 cross-race guard: login binds (UB-45b). Binds are per player
//! and not persisted (F14), so every in-session bind change needs a login
//! twin, or a relog changes what the NPC says. F21 makes this sharper: a bind
//! no chain claims is opened by the interact fallback, and two binds on one
//! slot open the first one. These tests fire the login on every walk state.

use std::collections::{BTreeMap, BTreeSet};

use super::super::*;
use super::walk::{apply, archetypes, walk, Race, WalkState};
use crate::test_support::require_db_or_skip;

/// The Beta bind slots: a login in another world must bind nothing here.
const BETA_SLOTS: std::ops::RangeInclusive<i64> = 2300..=2349;
/// Worlds other than Beta that a login can name (`entities/spaces.xml`).
const OTHER_WORLDS: [&str; 2] = ["Omega_Site", "SGC_W1"];

/// The actions of a resolution in chain-id order (the order the engine runs
/// them in), so a bind and its clear are read in the order they fire.
pub(super) fn in_chain_order(resolved: &ResolvedActions) -> Vec<&Action> {
    let mut items: Vec<&(i64, Action)> = resolved.actions.iter().collect();
    items.sort_by_key(|(chain, _)| *chain);
    items.into_iter().map(|(_, action)| action).collect()
}

/// Apply the bind actions in `actions` to `binds`, as (dialog set, slot).
/// An add binds its slot; a remove clears the slot (the seed names only the
/// slot in `remove_dialog_set`).
pub(super) fn fold_binds(binds: &mut BTreeSet<(i64, i64)>, actions: &[&Action]) {
    for action in actions {
        match action {
            Action::AddDialogSet {
                dialog_set_id,
                slot,
                ..
            } => {
                binds.insert((i64::from(*dialog_set_id), i64::from(*slot)));
            }
            Action::RemoveDialogSet { slot, .. } => {
                let slot = i64::from(*slot);
                binds.retain(|(_, s)| *s != slot);
            }
            _ => {}
        }
    }
}

/// The objectives of each step, from `mission_objectives`, read once per test.
pub(super) async fn step_objectives(pool: &PgPool) -> BTreeMap<i64, Vec<i64>> {
    let rows: Vec<(i32, i32)> = sqlx::query_as(
        "SELECT step_id, objective_id FROM resources.mission_objectives \
         ORDER BY step_id, objective_id",
    )
    .fetch_all(pool)
    .await
    .expect("read the objectives of each step");
    let mut by_step: BTreeMap<i64, Vec<i64>> = BTreeMap::new();
    for (step, objective) in rows {
        by_step
            .entry(i64::from(step))
            .or_default()
            .push(i64::from(objective));
    }
    by_step
}

/// `apply` plus the objective statuses the engine sets
/// (`populate_mission_context`): an objective of an active step that is not
/// done reads `active`. Login chains gate on it (chains 23454-23457).
pub(super) fn stamp(
    ctx: ExecutionContext,
    s: &WalkState,
    objectives: &BTreeMap<i64, Vec<i64>>,
) -> ExecutionContext {
    let mut ctx = apply(ctx, s);
    for &(mission, step) in s.step.iter().chain(s.overlap.iter()) {
        for &objective in objectives.get(&step).into_iter().flatten() {
            if !s.objectives_done.contains(&(mission, objective)) {
                with_obj(&mut ctx, mission as i32, objective as i32, "active");
            }
        }
    }
    ctx
}

/// The binds a login into Beta at `state`, as `archetype`, leaves: the full
/// login resolution, receiver chains included (their step advance and bind
/// both fire on the login).
pub(super) fn login_binds(
    engine: &ChainEngine,
    state: &WalkState,
    archetype: i64,
    objectives: &BTreeMap<i64, Vec<i64>>,
) -> BTreeSet<(i64, i64)> {
    let ctx = stamp(login_ctx(BETA, Some(archetype)), state, objectives);
    let resolved = fire(engine, TriggerType::PlayerLoaded, &ctx);
    let mut binds = BTreeSet::new();
    fold_binds(&mut binds, &in_chain_order(&resolved));
    binds
}

/// Every `add_dialog_set` a matched chain of `resolved` makes, as
/// (dialog set, slot), in chain-id order.
fn add_binds(resolved: &ResolvedActions) -> Vec<(i64, i64)> {
    in_chain_order(resolved)
        .into_iter()
        .filter_map(|action| match action {
            Action::AddDialogSet {
                dialog_set_id,
                slot,
                ..
            } => Some((i64::from(*dialog_set_id), i64::from(*slot))),
            _ => None,
        })
        .collect()
}

/// On every walk state of every race, for every archetype the race admits,
/// a login into Beta binds no slot twice. Two binds on one slot is the F21
/// hazard: the interact fallback opens the first one, not the one the
/// session showed.
#[tokio::test]
async fn login_binds_at_most_one_dialog_set_per_slot() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let objectives = step_objectives(&pool).await;
    for race in Race::ALL {
        for state in walk(&pool, race).await {
            for archetype in archetypes(&state) {
                let ctx = stamp(login_ctx(BETA, Some(archetype)), &state, &objectives);
                let binds = add_binds(&fire(&engine, TriggerType::PlayerLoaded, &ctx));
                let mut slots: Vec<i64> = binds.iter().map(|(_, slot)| *slot).collect();
                slots.sort_unstable();
                slots.dedup();
                assert_eq!(
                    slots.len(),
                    binds.len(),
                    "{} archetype {archetype}: login binds {binds:?} put two dialog sets on one slot",
                    state.label
                );
            }
        }
    }
}

/// On every walk state of every race, for every archetype the race admits, a
/// login into Omega_Site or SGC_W1 binds nothing on the Beta slots. The
/// Beta binds are for Beta's world only.
#[tokio::test]
async fn no_beta_bind_outside_beta() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let objectives = step_objectives(&pool).await;
    for world in OTHER_WORLDS {
        for race in Race::ALL {
            for state in walk(&pool, race).await {
                for archetype in archetypes(&state) {
                    let ctx = stamp(login_ctx(world, Some(archetype)), &state, &objectives);
                    let resolved = fire(&engine, TriggerType::PlayerLoaded, &ctx);
                    let stray: Vec<(i64, i64)> = add_binds(&resolved)
                        .into_iter()
                        .filter(|(_, slot)| BETA_SLOTS.contains(slot))
                        .collect();
                    assert!(
                        stray.is_empty(),
                        "{} archetype {archetype} logging into {world}: binds {stray:?} on Beta slots",
                        state.label
                    );
                }
            }
        }
    }
}
