//! Walk states for the cross-race guard (UB-45a; UB-45b reuses them). Every
//! state is derived from the seed: the steps and objectives from
//! `mission_steps` and `mission_objectives`, the counter names from the
//! `increment_counter` rows of chains 23000-23499, and the trigger keys from
//! `content_triggers`. The race order is the one hand-listed input, and it is
//! D-UB15's.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use super::super::*;

/// 1425 sits at S4244 while 1426 is offered (UB-15), so the two overlap. The
/// cross states pair the two, as UB-14-16 describes.
const OVERLAP_MISSION: i64 = 1425;
const OVERLAP_STEP: i64 = 4244;
const OVERLAP_NEXT: i64 = 1426;
/// Most objectives one step may have. Its done-subsets number `2^n`.
const MAX_OBJECTIVES: usize = 5;

/// A Beta race, with the offer order D-UB15 fixes for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Race {
    Human,
    FreeJaffa,
    Asgard,
}

impl Race {
    /// Every race, in test order.
    pub(crate) const ALL: [Race; 3] = [Race::Human, Race::FreeJaffa, Race::Asgard];

    /// The tag a walk-state label starts with.
    fn tag(self) -> &'static str {
        match self {
            Race::Human => "hum",
            Race::FreeJaffa => "fj",
            Race::Asgard => "asg",
        }
    }

    /// The offer order (D-UB15). 1503 is left out: `beta_e1_m1503_chains.sql`
    /// is not in `database.sql`, so its chains are not seeded (UB-22).
    fn missions(self) -> &'static [i64] {
        match self {
            Race::Human => &[1423, 381, 473, 478, 1425, 1426, 1427],
            Race::FreeJaffa => &[1500, 1502],
            Race::Asgard => &[481, 420, 421, 1147, 443, 477, 479],
        }
    }

    /// The archetype the race's own chains gate on (Human uses 1, the first
    /// of the Human range; [`archetypes`] expands it).
    fn archetype(self) -> i64 {
        match self {
            Race::Human => HUMANS[0],
            Race::FreeJaffa => FREE_JAFFA,
            Race::Asgard => ASGARD,
        }
    }
}

/// One state a player can be in on the Beta walk.
#[derive(Clone, Debug)]
pub(crate) struct WalkState {
    /// Short label for failure messages, e.g. `asg 477 S1744 O2060+O2062 c5`.
    pub label: String,
    /// The archetype the race's chains gate on. Human states use `HUMANS[0]`.
    pub archetype: i64,
    /// Every race mission's status: `completed`, `active` or `not_active`.
    pub missions: Vec<(i64, &'static str)>,
    /// The active step, as (mission, step).
    pub step: Option<(i64, i64)>,
    /// The objectives done, as (mission, objective).
    pub objectives_done: Vec<(i64, i64)>,
    /// Extra active steps of other missions (the 1425/1426 overlap only).
    pub overlap: Vec<(i64, i64)>,
    /// Counter values, by counter name.
    pub counters: Vec<(String, i64)>,
}

/// Every walk state of `race`, derived from the seed. Per mission: the state
/// before it is offered, one state per step (per subset of that step's done
/// objectives), and the state after it completes. Each state appears twice,
/// with every counter at 0 and at 5.
pub(crate) async fn walk(pool: &PgPool, race: Race) -> Vec<WalkState> {
    let order = race.missions();
    let mut core = Vec::new();
    for (i, &mission) in order.iter().enumerate() {
        let start = core.len();
        core.push(state(
            format!("{} {mission} pre", race.tag()),
            race,
            statuses(order, i, "not_active"),
            None,
            Vec::new(),
        ));
        for step in step_ids(pool, mission).await {
            for done in objective_subsets(pool, step).await {
                let mut label = format!("{} {mission} S{step}", race.tag());
                if !done.is_empty() {
                    let names: Vec<String> = done.iter().map(|o| format!("O{o}")).collect();
                    label.push_str(&format!(" {}", names.join("+")));
                }
                core.push(state(
                    label,
                    race,
                    statuses(order, i, "active"),
                    Some((mission, step)),
                    done.iter().map(|&o| (mission, o)).collect(),
                ));
            }
        }
        core.push(state(
            format!("{} {mission} done", race.tag()),
            race,
            statuses(order, i, "completed"),
            None,
            Vec::new(),
        ));
        if race == Race::Human && mission == OVERLAP_NEXT {
            let cross: Vec<WalkState> = core[start..].iter().map(overlap_with).collect();
            core.extend(cross);
        }
    }
    let counters = counter_names(pool).await;
    core.iter()
        .flat_map(|s| [0, 5].map(|level| with_counters(s, &counters, level)))
        .collect()
}

/// The archetypes a state is tested with: all four Human archetypes for a
/// Human state (the Union Human gate is `gte 1` and `lte 4`), else its own.
pub(crate) fn archetypes(s: &WalkState) -> Vec<i64> {
    if HUMANS.contains(&s.archetype) {
        HUMANS.to_vec()
    } else {
        vec![s.archetype]
    }
}

/// The trigger keys the Beta chains answer to, per event kind.
pub(crate) struct TriggerKeys {
    /// `interact_tag` spawn tags.
    pub tags: Vec<String>,
    /// `dialog_choice` dialog ids.
    pub dialogs: Vec<i64>,
    /// `item_use` item ids.
    pub items: Vec<i64>,
    /// `mission_completed` mission ids.
    pub missions: Vec<i64>,
}

/// The trigger keys of every chain in the Beta block (23000-23499).
pub(crate) async fn beta_trigger_keys(pool: &PgPool) -> TriggerKeys {
    let rows: Vec<(String, Option<String>)> = sqlx::query_as(
        "SELECT event_type, event_key FROM resources.content_triggers \
         WHERE chain_id BETWEEN 23000 AND 23499",
    )
    .fetch_all(pool)
    .await
    .expect("read the Beta trigger keys");
    let keys = |kind: &str| -> BTreeSet<String> {
        rows.iter()
            .filter(|(t, _)| t == kind)
            .filter_map(|(_, k)| k.clone())
            .collect()
    };
    let ids = |kind: &str| -> Vec<i64> {
        keys(kind)
            .iter()
            .map(|k| {
                k.parse()
                    .unwrap_or_else(|_| panic!("{kind} key {k} must be an id"))
            })
            .collect()
    };
    TriggerKeys {
        tags: keys("interact_tag").into_iter().collect(),
        dialogs: ids("dialog_choice"),
        items: ids("item_use"),
        missions: ids("mission_completed"),
    }
}

/// Stamp `s` onto `ctx`: mission and step statuses, done objectives, counters.
/// The archetype is the caller's, so one state serves every race it is tried with.
pub(crate) fn apply(mut ctx: ExecutionContext, s: &WalkState) -> ExecutionContext {
    for &(mission, status) in &s.missions {
        with_mission(&mut ctx, mission as i32, status);
    }
    for &(mission, step) in s.step.iter().chain(s.overlap.iter()) {
        with_step(&mut ctx, mission as i32, step as i32, "active");
    }
    for &(mission, obj) in &s.objectives_done {
        with_obj(&mut ctx, mission as i32, obj as i32, "completed");
    }
    for (name, value) in &s.counters {
        with_counter(&mut ctx, name, *value);
    }
    ctx
}

/// The step ids of `mission` in playing order (`mission_steps.index`).
async fn step_ids(pool: &PgPool, mission: i64) -> Vec<i64> {
    sqlx::query_scalar::<_, i32>(
        "SELECT step_id FROM resources.mission_steps WHERE mission_id = $1 ORDER BY index",
    )
    .bind(mission as i32)
    .fetch_all(pool)
    .await
    .expect("read the mission's steps")
    .into_iter()
    .map(i64::from)
    .collect()
}

/// Every subset of the step's objectives, as the list of objectives done.
/// Panics past [`MAX_OBJECTIVES`], since the subsets would explode.
async fn objective_subsets(pool: &PgPool, step: i64) -> Vec<Vec<i64>> {
    let objectives: Vec<i64> = sqlx::query_scalar::<_, i32>(
        "SELECT objective_id FROM resources.mission_objectives \
         WHERE step_id = $1 ORDER BY objective_id",
    )
    .bind(step as i32)
    .fetch_all(pool)
    .await
    .expect("read the step's objectives")
    .into_iter()
    .map(i64::from)
    .collect();
    assert!(
        objectives.len() <= MAX_OBJECTIVES,
        "step {step} has {} objectives; the done-subsets would explode",
        objectives.len()
    );
    (0..(1usize << objectives.len()))
        .map(|mask| {
            objectives
                .iter()
                .enumerate()
                .filter(|(bit, _)| mask & (1 << bit) != 0)
                .map(|(_, &o)| o)
                .collect()
        })
        .collect()
}

/// The `increment_counter` keys of the Beta chains (one query).
async fn counter_names(pool: &PgPool) -> Vec<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT target_key FROM resources.content_actions \
         WHERE action_type = 'increment_counter' AND chain_id BETWEEN 23000 AND 23499 \
         AND target_key IS NOT NULL ORDER BY target_key",
    )
    .fetch_all(pool)
    .await
    .expect("read the increment_counter keys")
}

/// The race's missions with the one at `i` at `status`: earlier ones are
/// completed, later ones not active.
fn statuses(order: &[i64], i: usize, status: &'static str) -> Vec<(i64, &'static str)> {
    order
        .iter()
        .enumerate()
        .map(|(j, &m)| {
            let s = match j.cmp(&i) {
                Ordering::Less => "completed",
                Ordering::Equal => status,
                Ordering::Greater => "not_active",
            };
            (m, s)
        })
        .collect()
}

fn state(
    label: String,
    race: Race,
    missions: Vec<(i64, &'static str)>,
    step: Option<(i64, i64)>,
    objectives_done: Vec<(i64, i64)>,
) -> WalkState {
    WalkState {
        label,
        archetype: race.archetype(),
        missions,
        step,
        objectives_done,
        overlap: Vec::new(),
        counters: Vec::new(),
    }
}

/// The 1426 state with 1425 also active at S4244 (UB-14-16).
fn overlap_with(s: &WalkState) -> WalkState {
    let mut cross = s.clone();
    cross.label = format!("{} +{OVERLAP_MISSION} S{OVERLAP_STEP}", s.label);
    for (mission, status) in cross.missions.iter_mut() {
        if *mission == OVERLAP_MISSION {
            *status = "active";
        }
    }
    cross.overlap.push((OVERLAP_MISSION, OVERLAP_STEP));
    cross
}

/// The same state with every counter at `level`.
fn with_counters(s: &WalkState, counters: &[String], level: i64) -> WalkState {
    WalkState {
        label: format!("{} c{level}", s.label),
        counters: counters.iter().map(|name| (name.clone(), level)).collect(),
        ..s.clone()
    }
}
