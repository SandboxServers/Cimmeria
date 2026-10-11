//! Row-boundary snapshots and fixture parity (lab-fixtures FX-10, D-FX4,
//! D-FX10). A row that continues a chain is snapshotted before its setup:
//! the chain character's missions, carried items and seen tutorials, read
//! with read-only SQL, and its world. Parity compares that with the row's
//! fixture, so a fixture that drifts from what the player route produces is
//! caught on every full run. Pure: the runner's half (`runner/snapshot.rs`)
//! makes the calls and writes the file.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::evidence::RowResult;
use super::fixture::FIXTURE_CARRIED_CONTAINERS;
use super::grade::Grade;
use super::spec::FixtureSpec;
use super::spec_fixture::FixtureMissionState;

/// `RowSnapshot::schema`.
pub const SNAPSHOT_SCHEMA: u32 = 1;
/// The longest parity reason or flag a row carries.
pub const PARITY_CAP: usize = 300;

/// `sgw_mission.status` of an active and a completed mission (FX-02
/// worknote, "Mission row shape").
const STATUS_ACTIVE: i64 = 1;
const STATUS_COMPLETED: i64 = 2;

/// The chain character's state at a row boundary, before the row's setup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RowSnapshot {
    pub schema: u32,
    /// The character's last name (ASCII letters only).
    pub character: String,
    /// The session's world, `None` when no session lists the character.
    pub world: Option<String>,
    pub missions: Vec<SnapMission>,
    /// Carried items (containers 1 to 15), one entry per design id and
    /// container.
    pub items: Vec<SnapItem>,
    pub tutorials_seen: Vec<i32>,
    /// Host clock when the snapshot was taken.
    pub host_ms: i64,
}

/// One `sgw_mission` row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapMission {
    pub id: i32,
    /// `active`, `completed`, or `status <n>` for any other value.
    pub status: String,
    /// `current_step_id` (null on a completed row).
    pub step: Option<i32>,
}

/// The carried count of one design id in one container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapItem {
    pub id: i32,
    pub container: i32,
    pub count: i64,
}

/// What parity found. Mismatches fail a passing row; `unspecified` (chain
/// missions the fixture does not name) is reported only.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Parity {
    #[serde(default)]
    pub mismatches: Vec<String>,
    #[serde(default)]
    pub unspecified: Vec<String>,
    /// Why no snapshot was taken; parity was not checked. Kept here, not
    /// only as a reason, so a re-grade after an attestation keeps it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot_error: Option<String>,
}

/// Three read-only statements (missions, carried items, tutorials) for the
/// character whose name ends in ` <name>` (end-anchored, as FX-05's verify
/// SQL: `p1`'s character must not match `p10`'s). `name` is formatted into
/// the SQL, so it must be ASCII letters.
pub fn snapshot_sql(name: &str) -> Result<[String; 3], String> {
    letters_only(name)?;
    let n = name.to_ascii_lowercase();
    let carried = FIXTURE_CARRIED_CONTAINERS
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    Ok([
        format!(
            "SELECT m.mission_id AS id, m.status AS status, m.current_step_id AS step \
             FROM sgw_mission m JOIN sgw_player p ON p.player_id = m.player_id \
             WHERE p.player_name ILIKE '% {n}' ORDER BY m.mission_id"
        ),
        format!(
            "SELECT i.type_id AS id, i.container_id AS container, \
             coalesce(sum(i.stack_size), 0) AS n FROM sgw_inventory i \
             JOIN sgw_player p ON p.player_id = i.character_id \
             WHERE p.player_name ILIKE '% {n}' AND i.container_id IN ({carried}) \
             GROUP BY i.type_id, i.container_id ORDER BY 1, 2"
        ),
        format!(
            "SELECT t.tutorial_id AS id FROM sgw_player_tutorials t \
             JOIN sgw_player p ON p.player_id = t.player_id \
             WHERE p.player_name ILIKE '% {n}' ORDER BY 1"
        ),
    ])
}

fn letters_only(name: &str) -> Result<(), String> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(format!(
            "chain character {name:?} must be ASCII letters only"
        ));
    }
    Ok(())
}

/// A snapshot from the three `server_db_query` results of [`snapshot_sql`]
/// (`{row_count, truncated, row_cap, rows: [{alias: value}]}`). `world` and
/// `host_ms` are left for the caller. A truncated result is an error: a
/// partial list would report missing state that is there.
pub fn from_rows(
    name: &str,
    missions: &Value,
    items: &Value,
    tutorials: &Value,
) -> Result<RowSnapshot, String> {
    letters_only(name)?;
    let mut ms = rows_of("missions", missions)?
        .iter()
        .map(|r| {
            let status = match int("missions", r, "status")? {
                STATUS_ACTIVE => "active".to_string(),
                STATUS_COMPLETED => "completed".to_string(),
                s => format!("status {s}"),
            };
            let step = match r.get("step") {
                None | Some(Value::Null) => None,
                Some(_) => Some(int32("missions", r, "step")?),
            };
            Ok(SnapMission {
                id: int32("missions", r, "id")?,
                status,
                step,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    ms.sort_by_key(|m| m.id);
    let mut is = rows_of("items", items)?
        .iter()
        .map(|r| {
            Ok(SnapItem {
                id: int32("items", r, "id")?,
                container: int32("items", r, "container")?,
                count: int("items", r, "n")?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    is.sort_by_key(|i| (i.id, i.container));
    let mut ts = rows_of("tutorials", tutorials)?
        .iter()
        .map(|r| int32("tutorials", r, "id"))
        .collect::<Result<Vec<_>, String>>()?;
    ts.sort_unstable();
    ts.dedup();
    Ok(RowSnapshot {
        schema: SNAPSHOT_SCHEMA,
        character: name.to_string(),
        world: None,
        missions: ms,
        items: is,
        tutorials_seen: ts,
        host_ms: 0,
    })
}

fn rows_of<'a>(what: &str, reply: &'a Value) -> Result<&'a Vec<Value>, String> {
    if reply.get("truncated").and_then(Value::as_bool) == Some(true) {
        return Err(format!(
            "{what}: result truncated at {} rows",
            reply.get("row_cap").unwrap_or(&Value::Null)
        ));
    }
    reply
        .get("rows")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{what}: reply has no rows list"))
}

/// A whole number at `key` (1 and 1.0 both read as 1).
fn int(what: &str, row: &Value, key: &str) -> Result<i64, String> {
    let v = row.get(key).unwrap_or(&Value::Null);
    v.as_i64()
        .or_else(|| v.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64))
        .ok_or_else(|| format!("{what}: {key} is {v}, not a whole number"))
}

fn int32(what: &str, row: &Value, key: &str) -> Result<i32, String> {
    let v = int(what, row, key)?;
    i32::try_from(v).map_err(|_| format!("{what}: {key} {v} is out of range"))
}

/// How the chain holds a mission, for a mismatch or an `unspecified` line.
fn chain_has(m: Option<&SnapMission>) -> String {
    match m {
        None => "chain does not have it".into(),
        Some(m) if m.status == "active" => match m.step {
            Some(s) => format!("chain has step {s}"),
            None => "chain has it active with no step".into(),
        },
        Some(m) if m.status == "completed" => "chain has it completed".into(),
        Some(m) => format!("chain has it in {}", m.status),
    }
}

/// Every mission, item count, tutorial and the world the fixture names
/// must match `snap`; missions in `snap` the fixture does not name are
/// `unspecified`. A completed mission is compared by status only (its
/// objective arrays are not in the snapshot; FX-02 leaves
/// `active_objective_ids` empty where the route leaves the last roster).
pub fn parity(f: &FixtureSpec, snap: &RowSnapshot) -> Parity {
    let mut p = Parity::default();
    if let Some(want) = &f.world {
        match &snap.world {
            Some(w) if w == want => {}
            Some(w) => p
                .mismatches
                .push(format!("world: chain is in {w}, fixture wants {want}")),
            None => p
                .mismatches
                .push(format!("world: chain has no session, fixture wants {want}")),
        }
    }
    for fm in &f.missions {
        let have = snap.missions.iter().find(|m| m.id == fm.id);
        let (holds, want) = match fm.resolved_state() {
            Ok(FixtureMissionState::Active) => {
                let step = fm.step.map_or_else(|| "a step".into(), |s| s.to_string());
                let holds = have.is_some_and(|m| m.status == "active" && m.step == fm.step);
                (holds, step)
            }
            Ok(FixtureMissionState::Completed) => (
                have.is_some_and(|m| m.status == "completed"),
                "it completed".into(),
            ),
            Ok(FixtureMissionState::NotActive) => (have.is_none(), "it not active".into()),
            Err(e) => {
                p.mismatches.push(e);
                continue;
            }
        };
        if !holds {
            p.mismatches.push(format!(
                "mission {}: {}, fixture wants {want}",
                fm.id,
                chain_has(have)
            ));
        }
    }
    for m in &snap.missions {
        if !f.missions.iter().any(|fm| fm.id == m.id) {
            let has = chain_has(Some(m));
            let has = has.strip_prefix("chain has ").unwrap_or(&has);
            p.unspecified.push(format!("mission {}: {has}", m.id));
        }
    }
    for fi in &f.items {
        let carried = |c: Option<i32>| -> i64 {
            snap.items
                .iter()
                .filter(|i| i.id == fi.id && c.is_none_or(|c| i.container == c))
                .map(|i| i.count)
                .sum()
        };
        let want = i64::from(fi.count);
        let total = carried(None);
        if total != want {
            p.mismatches.push(format!(
                "item {}: chain carries {total}, fixture wants {want}",
                fi.id
            ));
            // The container line would only repeat the count.
            continue;
        }
        if let Ok(cid) = fi.container_id() {
            let here = carried(Some(cid));
            if here != want {
                p.mismatches.push(format!(
                    "item {}: chain carries {here} in container {cid}, fixture wants {want}",
                    fi.id
                ));
            }
        }
    }
    for t in &f.tutorials_unseen {
        if snap.tutorials_seen.contains(t) {
            p.mismatches.push(format!(
                "tutorial {t}: chain has seen it, fixture wants it unseen"
            ));
        }
    }
    p
}

/// Apply a chained row's parity to its grade: mismatches turn a PASS into
/// a FAIL (`fixture parity: ...`) and are a flag on any other result; a
/// snapshot error is a reason that changes nothing; unnamed missions are a
/// flag. The runner and `lab_uat_attest` both call this after `grade`, so a
/// re-grade keeps the parity result.
pub fn apply_parity(g: &mut Grade, p: Option<&Parity>) {
    let Some(p) = p else {
        return;
    };
    if let Some(e) = &p.snapshot_error {
        g.reasons.push(cap_chars(
            &format!("fixture parity not checked: {e}"),
            PARITY_CAP,
        ));
    }
    if !p.mismatches.is_empty() {
        let text = cap_chars(
            &format!("fixture parity: {}", p.mismatches.join("; ")),
            PARITY_CAP,
        );
        if g.result == RowResult::Pass {
            g.result = RowResult::Fail;
            g.reasons.push(text);
        } else {
            g.flags.push(text);
        }
    }
    if !p.unspecified.is_empty() {
        g.flags.push(cap_chars(
            &format!(
                "fixture parity: the fixture does not name {}",
                p.unspecified.join("; ")
            ),
            PARITY_CAP,
        ));
    }
}

/// `s` cut to `cap` characters (not bytes) ending in `...`.
pub fn cap_chars(s: &str, cap: usize) -> String {
    if s.chars().count() <= cap {
        return s.to_string();
    }
    let head: String = s.chars().take(cap.saturating_sub(3)).collect();
    format!("{head}...")
}

#[cfg(test)]
#[path = "snapshot_tests.rs"]
mod tests;
