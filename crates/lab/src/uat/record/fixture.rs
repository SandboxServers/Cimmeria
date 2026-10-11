//! LR-09: the row fixture built from a recording (D-LR11). A row's fixture
//! puts the profile character back in the state the row started from: the
//! missions its diffs touch and the item design ids whose count changed, at
//! their values in the snapshot before the row. Position and world are
//! never written (the row's `@stand` places the player). A start state the
//! lab-fixtures schema cannot express is a `# TODO` line, never a guess.

use crate::uat::spec_fixture::{FixtureItem, FixtureMission, FixtureMissionState, FixtureSpec};

use super::clauses::{diff_snapshots, row_snapshots, Diff, Inline, INV_MAIN, INV_MISSION};
use super::log::{DbSnapshot, RecordLog};
use super::segment::RecordedRow;

/// `sgw_mission.status` of an active mission.
const MISSION_ACTIVE: i32 = 1;
/// `sgw_mission.status` of a completed mission.
const MISSION_COMPLETED: i32 = 2;
/// The bags a fixture rewrites: staging deletes every instance of a listed
/// design id in containers 1 to 15 (D-FX7; 15 is `INV_CRAFTING`,
/// `crates/entity/src/inventory.rs:27`).
const FIXTURE_BAGS: std::ops::RangeInclusive<i32> = INV_MAIN..=15;
/// The most a fixture item may count (D-FX7, `check_fixture`).
const MAX_COUNT: i32 = 10;

/// A row's fixture and what it could not express.
#[derive(Debug, Clone, PartialEq)]
pub struct RowFixture {
    /// The fixture, `character` set to the profile.
    pub spec: FixtureSpec,
    /// `# TODO` texts, without the prefix: start states the schema cannot set.
    pub todos: Vec<String>,
}

/// The fixture that puts a character back where this row started: the
/// missions the row's diffs touch, at their start state, and the item
/// design ids whose count changed, at their start count. No position (the
/// row's `@stand` places the player) and no world.
pub fn row_fixture(profile: &str, before: &DbSnapshot, diffs: &[Diff]) -> RowFixture {
    let mut f = RowFixture {
        spec: FixtureSpec {
            character: profile.to_string(),
            world: None,
            position: None,
            missions: Vec::new(),
            items: Vec::new(),
            tutorials_unseen: Vec::new(),
        },
        todos: Vec::new(),
    };
    let mut missions: Vec<i32> = Vec::new();
    let mut items: Vec<i32> = Vec::new();
    for d in diffs {
        match d {
            Diff::MissionStatus { mission, .. } | Diff::MissionStep { mission, .. } => {
                missions.push(*mission)
            }
            Diff::ItemCount { type_id, .. } => items.push(*type_id),
            Diff::Player { .. } => {}
        }
    }
    missions.sort_unstable();
    missions.dedup();
    items.sort_unstable();
    items.dedup();

    for id in missions {
        let start = before.missions.iter().find(|m| m.mission_id == id);
        let (step, state) = match start.map(|m| (m.status, m.step)) {
            None => (None, Some(FixtureMissionState::NotActive)),
            Some((MISSION_ACTIVE, Some(step))) => (Some(step), None),
            Some((MISSION_COMPLETED, _)) => (None, Some(FixtureMissionState::Completed)),
            Some((MISSION_ACTIVE, None)) => {
                f.todos
                    .push(format!("mission {id} started active on no step"));
                continue;
            }
            Some((status, _)) => {
                f.todos
                    .push(format!("mission {id} started in status {status}"));
                continue;
            }
        };
        f.spec.missions.push(FixtureMission { id, step, state });
    }

    for id in items {
        if let Some(item) = start_item(id, before, &mut f.todos) {
            f.spec.items.push(item);
        }
    }
    f
}

/// The fixture item for design id `id` at its start count, or None with a
/// TODO per stack the schema cannot set. Staging rewrites every bag from 1
/// to 15, so the item is left out whole when any start stack is in another
/// bag, over the cap, or beside a second stack: writing the rest would
/// delete that stack.
fn start_item(id: i32, before: &DbSnapshot, todos: &mut Vec<String>) -> Option<FixtureItem> {
    let stacks: Vec<(i32, i32)> = before
        .items
        .iter()
        .filter(|i| i.type_id == id && FIXTURE_BAGS.contains(&i.container_id))
        .map(|i| (i.container_id, i.count))
        .collect();
    let mut faults = Vec::new();
    for &(bag, count) in &stacks {
        if container_name(bag).is_none() {
            faults.push(format!("item {id} started in bag {bag}"));
        } else if count > MAX_COUNT {
            faults.push(format!(
                "item {id} started at count {count} in bag {bag} (a fixture sets 0 to {MAX_COUNT})"
            ));
        }
    }
    if faults.is_empty() && stacks.len() > 1 {
        // Only bags 1 and 2 are left, one stack each per design id.
        faults.push(format!(
            "item {id} started in bags {INV_MAIN} and {INV_MISSION} (a fixture sets one bag)"
        ));
    }
    if !faults.is_empty() {
        todos.extend(faults);
        return None;
    }
    // No start stack: none carried, which `count = 0` sets in any bag.
    let (bag, count) = stacks.first().copied().unwrap_or((INV_MAIN, 0));
    Some(FixtureItem {
        id,
        count: u32::try_from(count).unwrap_or(0),
        container: container_name(bag).map(str::to_string),
    })
}

/// The fixture's name for a bag: never a number (FX-01 maps `mission` to
/// container 2, `INV_MISSION`).
fn container_name(bag: i32) -> Option<&'static str> {
    match bag {
        INV_MAIN => Some("main"),
        INV_MISSION => Some("mission"),
        _ => None,
    }
}

/// The `[row.fixture]` lines: the table, `character`, the missions and items
/// one per line, then the TODO lines.
pub fn fixture_toml(f: &RowFixture) -> Vec<String> {
    let s = |v: &str| Inline::Str(v.to_string());
    let int = |v: i32| Inline::Int(i64::from(v));
    let mut out = vec![
        "[row.fixture]".to_string(),
        format!("character = {}", s(&f.spec.character).render()),
    ];
    let missions = f.spec.missions.iter().map(|m| {
        let mut kv = vec![("id".to_string(), int(m.id))];
        if let Some(step) = m.step {
            kv.push(("step".into(), int(step)));
        }
        if let Some(state) = m.state {
            kv.push(("state".into(), s(state_name(state))));
        }
        Inline::Table(kv)
    });
    let missions: Vec<Inline> = missions.collect();
    let items: Vec<Inline> = f
        .spec
        .items
        .iter()
        .map(|i| {
            let mut kv = vec![
                ("id".to_string(), int(i.id)),
                ("count".into(), Inline::Int(i64::from(i.count))),
            ];
            if let Some(c) = &i.container {
                kv.push(("container".into(), s(c)));
            }
            Inline::Table(kv)
        })
        .collect();
    for (key, list) in [("missions", missions), ("items", items)] {
        if list.is_empty() {
            continue;
        }
        out.push(format!("{key} = ["));
        out.extend(list.iter().map(|v| format!("  {},", v.render())));
        out.push("]".into());
    }
    out.extend(f.todos.iter().map(|t| format!("# TODO {t}")));
    out
}

fn state_name(s: FixtureMissionState) -> &'static str {
    match s {
        FixtureMissionState::Active => "active",
        FixtureMissionState::NotActive => "not_active",
        FixtureMissionState::Completed => "completed",
    }
}

/// What the emitter writes after a row's clauses, and its `# TODO` count.
/// With a profile, the `[row.fixture]` block; without one, the same lines
/// commented under `# fixture (pass -Profile to enable):`, the character
/// left as `<profile>`. A row with no snapshot before it gets one TODO.
pub(super) fn row_block(
    log: &RecordLog,
    row: &RecordedRow,
    profile: Option<&str>,
) -> (Vec<String>, usize) {
    let Some((before, after)) = row_snapshots(log, row) else {
        return (
            vec!["# TODO fixture: no database snapshot before the row".into()],
            1,
        );
    };
    let f = row_fixture(
        profile.unwrap_or("<profile>"),
        before,
        &diff_snapshots(before, after),
    );
    let lines = fixture_toml(&f);
    let lines = match profile {
        Some(_) => lines,
        None => std::iter::once("# fixture (pass -Profile to enable):".to_string())
            .chain(lines.into_iter().map(|l| {
                if l.starts_with("# TODO") {
                    l
                } else {
                    format!("# {l}")
                }
            }))
            .collect(),
    };
    (lines, f.todos.len())
}

#[cfg(test)]
#[path = "fixture_tests.rs"]
mod tests;
