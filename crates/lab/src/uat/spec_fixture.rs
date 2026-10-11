//! Row fixture types for UAT specs (lab-fixtures FX-01): the state a row
//! starts from. Re-exported from [`super::spec`].

use std::collections::HashSet;

use serde::Deserialize;

use super::spec::{RowSpec, SectionSpec};

// ── Row fixtures (lab-fixtures FX-01) ──

/// The state a row starts from: a profile character, where it stands, and
/// the missions, items and tutorials the runner sets before the row's setup.
#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureSpec {
    /// A `[section.profiles]` key.
    pub character: String,
    #[serde(default)]
    pub world: Option<String>,
    /// Server metres, as `/gmgotoxyz` takes them.
    #[serde(default)]
    pub position: Option<[f32; 3]>,
    #[serde(default)]
    pub missions: Vec<FixtureMission>,
    #[serde(default)]
    pub items: Vec<FixtureItem>,
    #[serde(default)]
    pub tutorials_unseen: Vec<i32>,
}

/// One mission the fixture sets: active on a step, or in a given state.
#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureMission {
    pub id: i32,
    /// Active on this step. Implies `state = "active"`.
    #[serde(default)]
    pub step: Option<i32>,
    #[serde(default)]
    pub state: Option<FixtureMissionState>,
}

/// A mission's state in a fixture, as the spec writes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureMissionState {
    Active,
    NotActive,
    Completed,
}

/// One item count the fixture sets.
#[derive(Debug, Clone, PartialEq, Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct FixtureItem {
    /// Item design id (`sgw_inventory.type_id`).
    pub id: i32,
    /// Exact carried count after the fixture, 0 to 10 (D-FX7).
    pub count: u32,
    /// `main` (default, container 1, `INV_MAIN`) or `mission` (container 2, `INV_MISSION`, crates/entity/src/inventory.rs:14).
    #[serde(default)]
    pub container: Option<String>,
}

impl FixtureMission {
    /// `step` set means Active; no `step` and no `state` is a spec error.
    pub fn resolved_state(&self) -> Result<FixtureMissionState, String> {
        match (self.step, self.state) {
            (Some(_), None | Some(FixtureMissionState::Active)) => Ok(FixtureMissionState::Active),
            (Some(_), Some(s)) => Err(format!(
                "mission {}: a step makes it active, not {s:?}",
                self.id
            )),
            (None, Some(s)) => Ok(s),
            (None, None) => Err(format!("mission {}: needs a step or a state", self.id)),
        }
    }
}

impl FixtureItem {
    /// 1 for `main`/unset, 2 for `mission` (`INV_MISSION`), Err otherwise.
    pub fn container_id(&self) -> Result<i32, String> {
        match self.container.as_deref() {
            None | Some("main") => Ok(1),
            Some("mission") => Ok(2),
            Some(other) => Err(format!(
                "item {}: container {other:?} is not main or mission",
                self.id
            )),
        }
    }
}

impl FixtureSpec {
    /// True when the fixture writes server state (missions, items or
    /// tutorials), so the runner stages it before the session's one world
    /// entry (D-FX5).
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn writes_state(&self) -> bool {
        !self.missions.is_empty() || !self.items.is_empty() || !self.tutorials_unseen.is_empty()
    }
}

/// Fixture rules: the profile exists, the row can take a fixture (one
/// player, in world or any state), and each mission, item, tutorial, world
/// and position is well formed and unique.
pub(super) fn check_fixture(
    r: &str,
    f: &FixtureSpec,
    spec: &SectionSpec,
    row: &RowSpec,
    errs: &mut Vec<String>,
) {
    if !spec.section.profiles.contains_key(&f.character) {
        errs.push(format!(
            "{r}/fixture: no [section.profiles.{}]",
            f.character
        ));
    }
    if !matches!(row.state.as_str(), "in_world" | "any") {
        errs.push(format!(
            "{r}/fixture: state {:?} (a fixture needs in_world or any)",
            row.state
        ));
    }
    if row.players != 1 {
        errs.push(format!(
            "{r}/fixture: players = {} (fixtures are one-player only)",
            row.players
        ));
    }
    let mut mission_ids = HashSet::new();
    for m in &f.missions {
        if m.id <= 0 {
            errs.push(format!("{r}/fixture: mission id {} must be > 0", m.id));
        }
        if let Err(e) = m.resolved_state() {
            errs.push(format!("{r}/fixture: {e}"));
        }
        if m.step.is_some_and(|s| s <= 0) {
            errs.push(format!("{r}/fixture: mission {} step must be > 0", m.id));
        }
        if !mission_ids.insert(m.id) {
            errs.push(format!("{r}/fixture: duplicate mission {}", m.id));
        }
    }
    let mut item_ids = HashSet::new();
    for i in &f.items {
        if i.id <= 0 {
            errs.push(format!("{r}/fixture: item id {} must be > 0", i.id));
        }
        if i.count > 10 {
            errs.push(format!(
                "{r}/fixture: item {} count {} exceeds 10",
                i.id, i.count
            ));
        }
        if let Err(e) = i.container_id() {
            errs.push(format!("{r}/fixture: {e}"));
        }
        if !item_ids.insert(i.id) {
            errs.push(format!("{r}/fixture: duplicate item {}", i.id));
        }
    }
    let mut tutorial_ids = HashSet::new();
    for t in &f.tutorials_unseen {
        if *t <= 0 {
            errs.push(format!("{r}/fixture: tutorial id {t} must be > 0"));
        }
        if !tutorial_ids.insert(*t) {
            errs.push(format!("{r}/fixture: duplicate tutorial {t}"));
        }
    }
    if let Some(w) = &f.world {
        if w.is_empty() || w.chars().any(char::is_whitespace) {
            errs.push(format!(
                "{r}/fixture: world {w:?} must be non-empty with no whitespace"
            ));
        }
    }
    if let Some(p) = f.position {
        if !p.iter().all(|v| v.is_finite()) {
            errs.push(format!(
                "{r}/fixture: position {p:?} must be three finite numbers"
            ));
        }
    }
}
