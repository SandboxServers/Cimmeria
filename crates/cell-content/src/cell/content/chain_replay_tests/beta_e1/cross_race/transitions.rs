//! Beta Site E1 cross-race guard: transitions keep login binds consistent
//! (UB-45b). A click that moves a player from one walk state to another must
//! leave the binds a relog at the new state would give, or a relog changes
//! what the NPC says (F14).

use std::collections::{BTreeMap, BTreeSet};

use super::super::*;
use super::login_binds::{fold_binds, in_chain_order, login_binds, stamp, step_objectives};
use super::walk::{archetypes, beta_trigger_keys, walk, Race, TriggerKeys, WalkState};
use crate::test_support::require_db_or_skip;

/// Binds the walk does not reach yet, as (state label, dialog set, slot). An
/// entry needs a comment with its reason and the packet that owns it. Empty:
/// the merged seed keeps every transition consistent.
const KNOWN_DIVERGENCE: &[(&str, i64, i64)] = &[];

/// A walk state in canonical form, so a state computed by a transition can be
/// matched to a walk state. Counters are part of the key.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    missions: BTreeMap<i64, String>,
    active: BTreeSet<(i64, i64)>,
    objectives: BTreeSet<(i64, i64)>,
    counters: BTreeMap<String, i64>,
}

/// The canonical form of a walk state.
fn key(s: &WalkState) -> Key {
    Key {
        missions: s
            .missions
            .iter()
            .map(|(m, status)| (*m, status.to_string()))
            .collect(),
        active: s.step.iter().chain(s.overlap.iter()).copied().collect(),
        objectives: s.objectives_done.iter().copied().collect(),
        counters: s.counters.iter().cloned().collect(),
    }
}

/// A walk state rebuilt from its canonical form, for `apply` and the login.
fn state_of(k: &Key, label: &str, archetype: i64) -> WalkState {
    WalkState {
        label: label.to_string(),
        archetype,
        missions: k
            .missions
            .iter()
            .map(|(m, status)| (*m, status_of(status)))
            .collect(),
        step: None,
        objectives_done: k.objectives.iter().copied().collect(),
        overlap: k.active.iter().copied().collect(),
        counters: k.counters.iter().map(|(n, v)| (n.clone(), *v)).collect(),
    }
}

/// The static status string a walk state uses for a mission status.
fn status_of(status: &str) -> &'static str {
    match status {
        "completed" => "completed",
        "active" => "active",
        _ => "not_active",
    }
}

/// Move `k` by one action. Only the actions that change the walk state do
/// anything: accept (the mission's first step, by index), advance (the
/// mission's one active step; its done objectives are cleared, as the walk
/// keeps only the active step's objectives), complete objective, complete
/// mission (its step and objectives clear) and increment counter.
fn move_state(k: &mut Key, action: &Action, first: &BTreeMap<i64, i64>) {
    match action {
        Action::AcceptMission { mission_id } => {
            let m = i64::from(*mission_id);
            k.missions.insert(m, "active".to_string());
            k.active.retain(|(mm, _)| *mm != m);
            if let Some(&step) = first.get(&m) {
                k.active.insert((m, step));
            }
        }
        Action::AdvanceStep {
            mission_id,
            step_id,
        } => {
            let m = i64::from(*mission_id);
            k.active.retain(|(mm, _)| *mm != m);
            k.objectives.retain(|(mm, _)| *mm != m);
            k.active.insert((m, i64::from(*step_id)));
        }
        Action::CompleteObjective {
            mission_id,
            objective_id,
        } => {
            k.objectives
                .insert((i64::from(*mission_id), i64::from(*objective_id)));
        }
        Action::CompleteMission { mission_id } => {
            let m = i64::from(*mission_id);
            k.missions.insert(m, "completed".to_string());
            k.active.retain(|(mm, _)| *mm != m);
            k.objectives.retain(|(mm, _)| *mm != m);
        }
        Action::IncrementCounter {
            counter_name,
            amount,
        } => {
            *k.counters.entry(counter_name.clone()).or_insert(0) += i64::from(*amount);
        }
        _ => {}
    }
}

/// The trigger events a click, a choice and an item use make at `archetype`.
fn trigger_events(keys: &TriggerKeys, archetype: i64) -> Vec<(TriggerType, ExecutionContext)> {
    keys.tags
        .iter()
        .map(|tag| (TriggerType::InteractTag, interact_ctx(tag, archetype)))
        .chain(
            keys.dialogs
                .iter()
                .map(|&dialog| (TriggerType::DialogChoice, choice_ctx(dialog, archetype))),
        )
        .chain(
            keys.items
                .iter()
                .map(|&item| (TriggerType::ItemUse, item_use_ctx(item, archetype))),
        )
        .collect()
}

/// The first step of every mission, by `mission_steps.index`.
async fn first_steps(pool: &PgPool) -> BTreeMap<i64, i64> {
    sqlx::query_as::<_, (i32, i32)>(
        "SELECT DISTINCT ON (mission_id) mission_id, step_id \
         FROM resources.mission_steps ORDER BY mission_id, index",
    )
    .fetch_all(pool)
    .await
    .expect("read each mission's first step")
    .into_iter()
    .map(|(m, step)| (i64::from(m), i64::from(step)))
    .collect()
}

/// Whether any Beta chain listens for `mission_accepted`. None does today, so
/// the follow-up is skipped; the check keeps it honest when one does.
async fn mission_accepted_used(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM resources.content_triggers \
         WHERE event_type = 'mission_accepted' AND chain_id BETWEEN 23000 AND 23499)",
    )
    .fetch_one(pool)
    .await
    .expect("read the mission_accepted triggers")
}

/// For each walk state `s`, each trigger `e` and each archetype of the race:
/// the binds `s` gives, plus what the click and its follow-ups bind, must be
/// the binds a login at `s'` gives (as (dialog set, slot) sets). `s'` is the
/// state the click leaves; a transition to a state that is not in the walk
/// (a counter at 3, say) is skipped and counted.
/// Revert: delete restore chain 23313 (481 S1762 login bind). The Carter click
/// at S1761 (chain 23312) then expects (2467, 2305) at S1762, where login has
/// none, and this test must fail.
#[tokio::test]
async fn transitions_keep_login_binds_consistent() {
    let pool = require_db_or_skip!();
    let engine = whole_seed(&pool).await;
    let keys = beta_trigger_keys(&pool).await;
    let first = first_steps(&pool).await;
    let accepted_trigger = mission_accepted_used(&pool).await;
    let objectives = step_objectives(&pool).await;
    let (mut checked, mut skipped) = (0usize, 0usize);
    for race in Race::ALL {
        let states = walk(&pool, race).await;
        let known: BTreeSet<Key> = states.iter().map(key).collect();
        for state in &states {
            for archetype in archetypes(state) {
                for (trigger, ctx) in trigger_events(&keys, archetype) {
                    let resolved = fire(&engine, trigger.clone(), &stamp(ctx, state, &objectives));
                    let clicked = in_chain_order(&resolved);
                    let mut next = key(state);
                    for action in &clicked {
                        move_state(&mut next, action, &first);
                    }
                    if !known.contains(&next) {
                        skipped += 1;
                        continue;
                    }
                    checked += 1;
                    let after = state_of(&next, &state.label, archetype);

                    let mut expected = login_binds(&engine, state, archetype, &objectives);
                    fold_binds(&mut expected, &clicked);
                    for action in &clicked {
                        let follow_up = match action {
                            Action::CompleteMission { mission_id } => Some((
                                TriggerType::MissionCompleted,
                                completed_ctx(i64::from(*mission_id), archetype),
                            )),
                            Action::AcceptMission { mission_id } if accepted_trigger => {
                                let mut ctx = ExecutionContext::new();
                                ctx.set_param(
                                    "mission_id".to_string(),
                                    serde_json::json!(mission_id),
                                );
                                Some((TriggerType::MissionAccepted, ctx))
                            }
                            _ => None,
                        };
                        if let Some((kind, ctx)) = follow_up {
                            let followed = fire(&engine, kind, &stamp(ctx, &after, &objectives));
                            fold_binds(&mut expected, &in_chain_order(&followed));
                        }
                    }

                    let actual = login_binds(&engine, &after, archetype, &objectives);
                    let diff: Vec<(i64, i64)> = expected
                        .symmetric_difference(&actual)
                        .copied()
                        .filter(|&(dsm, slot)| {
                            !KNOWN_DIVERGENCE
                                .iter()
                                .any(|&(l, d, s)| l == state.label && d == dsm && s == slot)
                        })
                        .collect();
                    assert!(
                        diff.is_empty(),
                        "{} archetype {archetype}, {trigger:?}: login binds differ by {diff:?} \
                         (click and follow-ups expect {expected:?}, login at the next state gives {actual:?})",
                        state.label
                    );
                }
            }
        }
    }
    println!("transitions: {checked} checked, {skipped} skipped (next state not in the walk)");
    assert!(checked > 0, "no transition reached a walk state");
}
