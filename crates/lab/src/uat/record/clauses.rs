//! LR-04: the snapshot diff and the row clauses drafted from it (D-LR5,
//! D-LR7). A row's clauses are the `onDialogDisplay` messages the server
//! sent in it, then what changed in `sgw_mission`, the carried inventory and
//! `sgw_player` between the last snapshot before the row and the last one in
//! it. Server effects persist a tick or more after the call (README F5), so
//! the server clauses poll, with a timeout from the observed settle time.

use super::log::{DbSnapshot, RecordLog, SnapPlayer};
use super::segment::RecordedRow;

// Bag numbers, copied from `crates/entity/src/inventory.rs:13-26`:
// cimmeria-lab does not depend on cimmeria-entity.
/// The main bag.
pub const INV_MAIN: i32 = 1;
/// The mission-item bag.
// Inside the carried range (1 to 14); only the tests name it.
#[cfg_attr(not(test), allow(dead_code))]
pub const INV_MISSION: i32 = 2;
/// The first equipment bag (the bandolier).
pub const INV_BANDOLIER: i32 = 3;
/// The last equipment bag (the second artifact slot).
pub const INV_ARTIFACT2: i32 = 14;

/// Whether a container is carried by the player: the main and mission bags
/// and the equipment bags. The bank, buyback and the rest are not.
pub fn is_carried(container: i32) -> bool {
    (INV_MAIN..=INV_ARTIFACT2).contains(&container)
}

/// One change between two snapshots.
#[derive(Debug, Clone, PartialEq)]
pub enum Diff {
    /// A new mission (`from` None), a changed status, or a removed mission
    /// (`to` None).
    MissionStatus {
        /// `sgw_mission.mission_id`.
        mission: i32,
        /// The status before; None when the mission had no row.
        from: Option<i32>,
        /// The status after; None when the row is gone.
        to: Option<i32>,
    },
    /// Same status, changed step.
    MissionStep {
        /// `sgw_mission.mission_id`.
        mission: i32,
        /// `current_step_id` before; None when it was NULL.
        from: Option<i32>,
        /// `current_step_id` after; None when it is NULL.
        to: Option<i32>,
    },
    /// A carried stack's count changed; a stack that is not there counts 0.
    ItemCount {
        /// The item's design id.
        type_id: i32,
        /// The bag it is in ([`is_carried`]).
        container: i32,
        /// The count before.
        from: i32,
        /// The count after.
        to: i32,
        /// The item's name from the after snapshot, else the before one.
        name: Option<String>,
    },
    /// `level`, `exp`, `naquadah` or `world`.
    Player {
        /// Which of [`PLAYER_FIELDS`] changed.
        field: &'static str,
        /// The value before, as text.
        from: String,
        /// The value after, as text.
        to: String,
    },
}

/// What changed between two snapshots, in the contract's clause order:
/// missions by id, carried items by type then container, then the player's
/// level, exp, naquadah and world. Items outside the carried bags
/// ([`is_carried`]) are left out. The player is compared only when both
/// snapshots have the row.
pub fn diff_snapshots(before: &DbSnapshot, after: &DbSnapshot) -> Vec<Diff> {
    let mut out = Vec::new();

    let mut missions: Vec<i32> = before
        .missions
        .iter()
        .chain(&after.missions)
        .map(|m| m.mission_id)
        .collect();
    missions.sort_unstable();
    missions.dedup();
    for mission in missions {
        let find = |s: &DbSnapshot| s.missions.iter().find(|m| m.mission_id == mission).cloned();
        let (b, a) = (find(before), find(after));
        let (from, to) = (b.as_ref().map(|m| m.status), a.as_ref().map(|m| m.status));
        if from != to {
            out.push(Diff::MissionStatus { mission, from, to });
        } else if let (Some(b), Some(a)) = (b, a) {
            if b.step != a.step {
                out.push(Diff::MissionStep {
                    mission,
                    from: b.step,
                    to: a.step,
                });
            }
        }
    }

    let mut stacks: Vec<(i32, i32)> = before
        .items
        .iter()
        .chain(&after.items)
        .map(|i| (i.type_id, i.container_id))
        .filter(|&(_, c)| is_carried(c))
        .collect();
    stacks.sort_unstable();
    stacks.dedup();
    for (type_id, container) in stacks {
        let find = |s: &DbSnapshot| {
            s.items
                .iter()
                .find(|i| i.type_id == type_id && i.container_id == container)
                .cloned()
        };
        let (b, a) = (find(before), find(after));
        let from = b.as_ref().map_or(0, |i| i.count);
        let to = a.as_ref().map_or(0, |i| i.count);
        if from != to {
            let name = a.and_then(|i| i.name).or_else(|| b.and_then(|i| i.name));
            out.push(Diff::ItemCount {
                type_id,
                container,
                from,
                to,
                name,
            });
        }
    }

    if let (Some(b), Some(a)) = (&before.player, &after.player) {
        for field in PLAYER_FIELDS {
            let (from, to) = (player_field(b, field), player_field(a, field));
            if from != to {
                out.push(Diff::Player { field, from, to });
            }
        }
    }
    out
}

/// The player fields a row can change, in clause order.
const PLAYER_FIELDS: [&str; 4] = ["level", "exp", "naquadah", "world"];

fn player_field(p: &SnapPlayer, field: &str) -> String {
    match field {
        "level" => p.level.to_string(),
        "exp" => p.exp.to_string(),
        "naquadah" => p.naquadah.to_string(),
        _ => p.world_location.clone(),
    }
}

/// The last snapshot strictly before the row's anchor tick, and the last at
/// or before `last_tick`. Strictly before, because the anchor's own tick can
/// already hold its effect's DB write; the FS-P3 fixture keeps the old
/// state in tick 32 so the rule is pinned either way. None when either is
/// missing.
pub fn row_snapshots<'a>(
    log: &'a RecordLog,
    row: &RecordedRow,
) -> Option<(&'a DbSnapshot, &'a DbSnapshot)> {
    let last_where = |keep: &dyn Fn(u32) -> bool| {
        log.ticks
            .iter()
            .rev()
            .filter(|t| keep(t.seq))
            .find_map(|t| t.snapshot.as_ref())
    };
    let before = last_where(&|seq| seq < row.anchor.tick)?;
    let after = last_where(&|seq| seq <= row.last_tick)?;
    Some((before, after))
}

/// Host ms from the anchor's tick to the first tick (from the anchor's tick
/// to `last_tick`) whose snapshot shows `d`. A diff no snapshot in the row
/// shows (not one of this row's) gets the row's whole span.
/// [`row_clauses`] never reaches that fallback: its diffs end in the row's
/// own last snapshot, so some tick in the row always shows them.
///
/// `host_ms` comes from the JSONL, so the subtraction saturates rather than
/// trusting it to be sane.
pub fn settle_ms(log: &RecordLog, row: &RecordedRow, d: &Diff) -> i64 {
    let in_row = || {
        log.ticks
            .iter()
            .filter(move |t| (row.anchor.tick..=row.last_tick).contains(&t.seq))
    };
    let Some(anchor_ms) = in_row().next().map(|t| t.host_ms) else {
        return 0;
    };
    let shown = in_row()
        .find(|t| t.snapshot.as_ref().is_some_and(|s| shows(s, d)))
        .or_else(|| in_row().next_back())
        .map_or(anchor_ms, |t| t.host_ms);
    shown.saturating_sub(anchor_ms).max(0)
}

/// Whether a snapshot holds the state a diff ends in.
fn shows(s: &DbSnapshot, d: &Diff) -> bool {
    match d {
        Diff::MissionStatus { mission, to, .. } => {
            s.missions
                .iter()
                .find(|m| m.mission_id == *mission)
                .map(|m| m.status)
                == *to
        }
        Diff::MissionStep { mission, to, .. } => {
            s.missions
                .iter()
                .find(|m| m.mission_id == *mission)
                .and_then(|m| m.step)
                == *to
        }
        Diff::ItemCount {
            type_id,
            container,
            to,
            ..
        } => {
            s.items
                .iter()
                .find(|i| i.type_id == *type_id && i.container_id == *container)
                .map_or(0, |i| i.count)
                == *to
        }
        Diff::Player { field, to, .. } => s
            .player
            .as_ref()
            .is_some_and(|p| player_field(p, field) == *to),
    }
}

/// `clamp(max(2t, t + floor), floor, ceiling)`, rounded up to 250 ms.
/// Saturating, since `t` is a recorded time span: a huge one clamps to the
/// ceiling.
fn margin(t: i64, floor: i64, ceiling: i64) -> u64 {
    let t = t.max(0);
    let ms = t
        .saturating_mul(2)
        .max(t.saturating_add(floor))
        .clamp(floor, ceiling);
    // The floors and ceilings are multiples of 250, so rounding up never
    // leaves the clamp's range.
    ((ms + 249) / 250 * 250) as u64
}

/// A polled server clause: `clamp(max(2t, t + 3000), 3000, 60000)`.
pub fn clause_timeout_ms(t: i64) -> u64 {
    margin(t, 3000, 60_000)
}

/// A dialog wait: `clamp(max(2t, t + 2000), 2000, 30000)`.
pub fn dialog_timeout_ms(t: i64) -> u64 {
    margin(t, 2000, 30_000)
}

/// A `wait_server` step: `clamp(max(2t, t + 5000), 5000, 120000)`.
pub fn server_wait_timeout_ms(t: i64) -> u64 {
    margin(t, 5000, 120_000)
}

/// One clause, ready for the emitter: id, text and the key/value pairs in
/// write order (`source`, `tool`, `args`, `pointer`, `op`, `value`,
/// `timeout_ms`, ...).
#[derive(Debug, Clone, PartialEq)]
pub struct ClauseOut {
    /// The clause id, unique in the row (`dialog-3995`, `mission-622-step`).
    pub id: String,
    /// The one-line description a reader sees.
    pub text: String,
    /// The clause's keys and values, in the order they are written.
    pub fields: Vec<(&'static str, Inline)>,
}

/// The row's clauses, in the contract's order: one `dialog-<d>` per distinct
/// `onDialogDisplay` dialog id in the order sent, then one per
/// [`diff_snapshots`] entry.
///
/// `character_sql` is pasted between the SQL quotes as it is (`${character}`
/// or a `-CharacterExpr` expression). The caller owns making it safe; it is
/// neither escaped nor checked here.
pub fn row_clauses(log: &RecordLog, row: &RecordedRow, character_sql: &str) -> Vec<ClauseOut> {
    let mut out = Vec::new();

    let mut dialogs: Vec<i64> = Vec::new();
    for (_, name, decoded) in &row.server {
        if name != "onDialogDisplay" {
            continue;
        }
        if let Some(d) = decoded["dialog_id"].as_i64() {
            if !dialogs.contains(&d) {
                dialogs.push(d);
            }
        }
    }
    for d in dialogs {
        out.push(ClauseOut {
            id: format!("dialog-{d}"),
            text: format!("onDialogDisplay {d} to the player"),
            fields: vec![
                ("source", string("packet")),
                ("message", string("onDialogDisplay")),
                ("direction", string("to_client")),
                (
                    "match_fields",
                    Inline::Table(vec![("dialog_id".into(), Inline::Int(d))]),
                ),
                ("min_rows", Inline::Int(1)),
            ],
        });
    }

    let Some((before, after)) = row_snapshots(log, row) else {
        return out;
    };
    for d in diff_snapshots(before, after) {
        let timeout = clause_timeout_ms(settle_ms(log, row, &d));
        out.push(server_clause(&d, character_sql, timeout));
    }
    out
}

/// The polled `server_db_query` clause for one diff.
fn server_clause(d: &Diff, character: &str, timeout_ms: u64) -> ClauseOut {
    let mission_sql = |column: &str, mission: i32| {
        format!(
            "SELECT m.{column} FROM sgw_mission m JOIN sgw_player p ON p.player_id = m.player_id \
             WHERE p.player_name = '{character}' AND m.mission_id = {mission}"
        )
    };
    // (id, text, sql, pointer, expected value; None is `op = "absent"`)
    let (id, text, sql, pointer, value) = match d {
        Diff::MissionStatus { mission, to, .. } => (
            format!("mission-{mission}-status"),
            match to {
                Some(s) => format!("sgw_mission: {mission} status {s}"),
                None => format!("sgw_mission: {mission} has no row"),
            },
            mission_sql("status", *mission),
            // A removed mission has no row at all.
            if to.is_some() { "/rows/0/status" } else { "/rows/0" },
            to.map(|s| Inline::Int(i64::from(s))),
        ),
        Diff::MissionStep { mission, to, .. } => (
            format!("mission-{mission}-step"),
            match to {
                Some(s) => format!("sgw_mission: {mission} on step {s}"),
                None => format!("sgw_mission: {mission} on no step"),
            },
            mission_sql("current_step_id", *mission),
            "/rows/0/current_step_id",
            to.map(|s| Inline::Int(i64::from(s))),
        ),
        Diff::ItemCount {
            type_id,
            container,
            to,
            name,
            ..
        } => (
            format!("item-{type_id}-{container}"),
            match name {
                Some(n) => format!("carried {n} ({type_id}) in bag {container}: {to}"),
                None => format!("carried item {type_id} in bag {container}: {to}"),
            },
            format!(
                "SELECT count(*)::int AS n FROM sgw_inventory i JOIN sgw_player p ON p.player_id = i.character_id \
                 WHERE p.player_name = '{character}' AND i.type_id = {type_id} AND i.container_id = {container}"
            ),
            "/rows/0/n",
            Some(Inline::Int(i64::from(*to))),
        ),
        Diff::Player { field, to, .. } => {
            let column = if *field == "world" { "world_location" } else { field };
            let value = if *field == "world" {
                string(to)
            } else {
                // The snapshot wrote these from integer columns.
                to.parse().map_or_else(|_| string(to), Inline::Int)
            };
            (
                format!("player-{field}"),
                format!("sgw_player: {field} {to}"),
                format!("SELECT p.{column} FROM sgw_player p WHERE p.player_name = '{character}'"),
                match *field {
                    "level" => "/rows/0/level",
                    "exp" => "/rows/0/exp",
                    "naquadah" => "/rows/0/naquadah",
                    _ => "/rows/0/world_location",
                },
                Some(value),
            )
        }
    };
    let mut fields = vec![
        ("source", string("server")),
        ("tool", string("server_db_query")),
        ("args", Inline::Table(vec![("sql".into(), string(&sql))])),
        ("pointer", string(pointer)),
    ];
    match value {
        Some(v) => {
            fields.push(("op", string("eq")));
            fields.push(("value", v));
        }
        None => fields.push(("op", string("absent"))),
    }
    fields.push((
        "timeout_ms",
        Inline::Int(i64::try_from(timeout_ms).unwrap_or(i64::MAX)),
    ));
    ClauseOut { id, text, fields }
}

fn string(s: &str) -> Inline {
    Inline::Str(s.to_string())
}

/// An inline TOML value, as the emitter writes it on one line.
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    /// A string, written quoted and escaped.
    Str(String),
    /// An integer.
    Int(i64),
    /// The value and its fixed number of decimals (at least one is written,
    /// so the value stays a TOML float).
    Float(f64, usize),
    /// `true` or `false`.
    Bool(bool),
    /// An inline table; its pairs are written in this order.
    Table(Vec<(String, Inline)>),
    /// An array, in this order.
    // No clause writes one yet; LR-09's fixture lists will.
    #[cfg_attr(not(test), allow(dead_code))]
    Array(Vec<Inline>),
}

impl Inline {
    /// The value as inline TOML: strings escaped, floats with their fixed
    /// decimals, tables as `{ k = v, k = v }` (`{}` when empty), arrays as
    /// `[a, b]`.
    pub fn render(&self) -> String {
        match self {
            Inline::Str(s) => toml::Value::String(s.clone()).to_string(),
            Inline::Int(n) => n.to_string(),
            Inline::Float(f, _) if f.is_nan() => "nan".into(),
            Inline::Float(f, _) if f.is_infinite() => if *f > 0.0 { "inf" } else { "-inf" }.into(),
            Inline::Float(f, decimals) => format!("{:.*}", (*decimals).max(1), f),
            Inline::Bool(b) => b.to_string(),
            Inline::Table(kv) if kv.is_empty() => "{}".into(),
            Inline::Table(kv) => {
                let pairs: Vec<String> = kv
                    .iter()
                    .map(|(k, v)| format!("{} = {}", key(k), v.render()))
                    .collect();
                format!("{{ {} }}", pairs.join(", "))
            }
            Inline::Array(items) => {
                let items: Vec<String> = items.iter().map(Inline::render).collect();
                format!("[{}]", items.join(", "))
            }
        }
    }
}

/// A bare key when TOML allows one, else a quoted one.
fn key(k: &str) -> String {
    let bare = !k.is_empty()
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if bare {
        k.to_string()
    } else {
        toml::Value::String(k.to_string()).to_string()
    }
}

#[cfg(test)]
#[path = "clauses_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "clauses_row_tests.rs"]
mod row_tests;
