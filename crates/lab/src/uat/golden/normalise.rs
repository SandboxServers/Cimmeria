//! A row's packet tap as the events a golden compares: the variance model
//! applied (README D-GD5), entity ids as labels, run variables as
//! `${name}`, and the mission transitions read on the way (F11, F12).

use std::collections::BTreeSet;

use serde_json::Value;

use super::event::{Dir, Event, Variance};
use super::key_args::{decode_inbound, render_in, render_json, ENTITY_FIELDS, OUTBOUND_KEYS};
use super::labels::EntityLabels;
use crate::uat::spec::GoldenSpec;

/// Messages whose count depends on time spent, not on what the server
/// answered (F7). Always ignored, whatever the spec says.
pub const DEFAULT_IGNORE: &[&str] = &["avatarUpdate*", "perfStats", "requestEntityUpdate"];

/// How long after its first message a late-bound tap is still settling.
pub const DEFAULT_LATE_BIND_SETTLE_MS: u64 = 1000;

/// The run variables a fingerprint writes as `${name}`, because their
/// values differ on every run (F12).
pub const RUN_VARS: &[&str] = &["run_id", "character", "p2_character"];

/// `*` matches any run of characters (also none); everything else matches
/// itself, case-sensitively.
pub fn glob_match(pattern: &str, name: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let (Some((first, rest)), Some(last)) = (parts.split_first(), parts.last()) else {
        return false;
    };
    if rest.is_empty() {
        return pattern == name;
    }
    let Some(mut tail) = name.strip_prefix(first) else {
        return false;
    };
    // Leftmost match of each middle piece leaves the most room for the
    // pieces after it, so it never misses a match a later position finds.
    for mid in &rest[..rest.len() - 1] {
        match tail.find(mid) {
            Some(at) => tail = &tail[at + mid.len()..],
            None => return false,
        }
    }
    tail.ends_with(last)
}

fn matches_any(globs: &[String], name: &str) -> bool {
    globs.iter().any(|g| glob_match(g, name))
}

/// The variance one row is fingerprinted with: the built-in defaults, then
/// the section's table, then the row's.
pub fn resolve(section: Option<&GoldenSpec>, row: Option<&GoldenSpec>) -> Variance {
    let layers: Vec<&GoldenSpec> = section.into_iter().chain(row).collect();
    let list = |pick: fn(&GoldenSpec) -> &Vec<String>| {
        first_seen(layers.iter().flat_map(|g| pick(g).iter().cloned()))
    };
    let ignore = first_seen(
        DEFAULT_IGNORE
            .iter()
            .map(|g| g.to_string())
            .chain(layers.iter().flat_map(|g| g.ignore.iter().cloned())),
    );
    // Validation only rejects an overlap inside one table, so a section
    // `ignore` and a row `allow_unordered` can name the same glob. An
    // ignored message never reaches routing, so `ignore` wins; dropping the
    // glob here keeps the stored variance saying what actually happened.
    let mut allow_unordered = list(|g| &g.allow_unordered);
    allow_unordered.retain(|g| !ignore.contains(g));
    let mut key_args = std::collections::BTreeMap::new();
    for g in &layers {
        key_args.extend(g.key_args.iter().map(|(k, v)| (k.clone(), v.clone())));
    }
    Variance {
        ignore,
        allow_unordered,
        collapse: list(|g| &g.collapse),
        ignore_windows: list(|g| &g.ignore_windows),
        key_args,
        late_bind_settle_ms: row
            .and_then(|g| g.late_bind_settle_ms)
            .or_else(|| section.and_then(|g| g.late_bind_settle_ms))
            .unwrap_or(DEFAULT_LATE_BIND_SETTLE_MS),
    }
}

fn first_seen(items: impl Iterator<Item = String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for item in items {
        if !out.contains(&item) {
            out.push(item);
        }
    }
    out
}

/// One tap normalised. `events_from_tap` returns the contract's pair from
/// it; `build_row` also needs the transitions, which are read before
/// `allow_unordered` routing takes events out of order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Normalised {
    pub events: Vec<Event>,
    /// Rendered `allow_unordered` events, sorted.
    pub unordered: Vec<String>,
    pub transitions: Vec<String>,
}

/// The ordered events and the sorted, rendered unordered ones.
pub fn events_from_tap(
    messages: &[Value],
    late: bool,
    labels: &mut EntityLabels,
    v: &Variance,
    subst: &[(String, String)],
) -> (Vec<Event>, Vec<String>) {
    let n = normalise_tap(messages, late, labels, v, subst);
    (n.events, n.unordered)
}

/// `messages` in tap order. `subst` is `(name, value)` for each run
/// variable; a value found in a string is written `${name}`.
pub fn normalise_tap(
    messages: &[Value],
    late: bool,
    labels: &mut EntityLabels,
    v: &Variance,
    subst: &[(String, String)],
) -> Normalised {
    let mut vars: Vec<(String, String)> = subst
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .cloned()
        .collect();
    // Longest first, so a character name holding the run id is replaced
    // whole rather than leaving its prefix behind.
    vars.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));

    // The server's timestamp, never a host clock: the cut must land on the
    // same messages however late the host read the tap.
    let settle_until = messages.first().and_then(ts_ms).map(|first| {
        first.saturating_add(i64::try_from(v.late_bind_settle_ms).unwrap_or(i64::MAX))
    });

    let mut kept = Vec::new();
    for m in messages {
        if let (true, Some(until), Some(ts)) = (late, settle_until, ts_ms(m)) {
            if ts < until {
                continue;
            }
        }
        let name = event_name(m);
        // Before anything else, so an ignored message never takes a `?n`.
        if matches_any(&v.ignore, &name) {
            continue;
        }
        if let Some(event) = build_event(m, name, labels, v, &vars) {
            kept.push(event);
        }
    }

    let transitions = transitions(&kept);
    let (unordered, mut events): (Vec<Event>, Vec<Event>) = kept
        .into_iter()
        .partition(|e| matches_any(&v.allow_unordered, &e.name));
    events.dedup_by(|later, earlier| later == earlier && matches_any(&v.collapse, &later.name));
    let mut unordered: Vec<String> = unordered.iter().map(Event::render).collect();
    unordered.sort();
    Normalised {
        events,
        unordered,
        transitions,
    }
}

fn ts_ms(m: &Value) -> Option<i64> {
    m.get("ts_ms").and_then(Value::as_i64)
}

/// The tap's `msg_name`; `unknown#<method_index>` when it has none.
fn event_name(m: &Value) -> String {
    match m.get("msg_name").and_then(Value::as_str) {
        Some(name) if name != "unknown" => name.to_string(),
        _ => {
            let index = m.get("method_index").and_then(Value::as_i64).unwrap_or(-1);
            format!("unknown#{index}")
        }
    }
}

/// None for a record whose direction is neither `in` nor `out`.
fn build_event(
    m: &Value,
    name: String,
    labels: &mut EntityLabels,
    v: &Variance,
    vars: &[(String, String)],
) -> Option<Event> {
    match m.get("dir").and_then(Value::as_str)? {
        "in" => {
            let args = inbound_args(m)
                .into_iter()
                .flatten()
                .map(|(k, a)| (k.to_string(), render_in(&a, labels)))
                .collect();
            Some(Event {
                dir: Dir::C2s,
                name,
                target: None,
                args,
            })
        }
        "out" => {
            let target = m
                .get("target_entity_id")
                .and_then(Value::as_u64)
                .and_then(|id| u32::try_from(id).ok())
                .map(|id| labels.label(id));
            let args = outbound_args(m, &name, labels, v, vars);
            Some(Event {
                dir: Dir::S2c,
                name,
                target,
                args,
            })
        }
        _ => None,
    }
}

/// Inbound calls carry no `decoded` (F5): their key arguments come from
/// the raw bytes.
fn inbound_args(m: &Value) -> Option<Vec<(&'static str, super::key_args::InArg)>> {
    let index = i32::try_from(m.get("method_index")?.as_i64()?).ok()?;
    let msg_id = m
        .get("msg_id")
        .and_then(Value::as_u64)
        .and_then(|id| u8::try_from(id).ok());
    let bytes = hex_bytes(m.get("args_hex")?.as_str()?)?;
    decode_inbound(index, msg_id, &bytes)
}

fn hex_bytes(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// The spec's `key_args` entry for the message, else the built-in one;
/// fields the record lacks are left out.
fn outbound_args(
    m: &Value,
    name: &str,
    labels: &mut EntityLabels,
    v: &Variance,
    vars: &[(String, String)],
) -> Vec<(String, String)> {
    let fields: Vec<&str> = match v.key_args.get(name) {
        Some(fields) => fields.iter().map(String::as_str).collect(),
        None => OUTBOUND_KEYS
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, fields)| fields.to_vec())
            .unwrap_or_default(),
    };
    let Some(decoded) = m.get("decoded") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for field in fields {
        let Some(value) = field.split('.').try_fold(decoded, |at, key| at.get(key)) else {
            continue;
        };
        let leaf = field.rsplit('.').next().unwrap_or(field);
        let entity = if ENTITY_FIELDS.contains(&leaf) {
            value.as_u64().and_then(|id| u32::try_from(id).ok())
        } else {
            None
        };
        let rendered = match entity {
            Some(id) => labels.label(id),
            // Substituted inside the value: render_json cuts at 80
            // characters, which could split a run id and keep half of it.
            None => render_json(&substitute(value, vars)),
        };
        out.push((field.to_string(), rendered));
    }
    out
}

/// Every string leaf with the run variables written as `${name}`.
fn substitute(v: &Value, vars: &[(String, String)]) -> Value {
    match v {
        Value::String(s) => Value::String(replace_vars(s, vars)),
        Value::Array(items) => Value::Array(items.iter().map(|i| substitute(i, vars)).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, i)| (k.clone(), substitute(i, vars)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// One left-to-right pass, `vars` longest first, ASCII case-insensitive. A
/// single pass never rescans a `${name}` it wrote, so a short value cannot
/// match inside a placeholder.
fn replace_vars(s: &str, vars: &[(String, String)]) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at = 0;
    'scan: while let Some(rest) = s.get(at..) {
        for (name, value) in vars {
            let hit = rest
                .get(..value.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(value));
            if hit {
                out.push_str("${");
                out.push_str(name);
                out.push('}');
                at += value.len();
                continue 'scan;
            }
        }
        let Some(c) = rest.chars().next() else {
            break;
        };
        out.push(c);
        at += c.len_utf8();
    }
    out
}

/// `mission <MissionID> -> <Status>`, `step <StepID> -> <Status>` and
/// `objective <ObjectiveID> -> <Status>`, in event order. An event missing
/// either argument (a spec `key_args` that drops it) gives none.
pub fn transitions(events: &[Event]) -> Vec<String> {
    const KINDS: &[(&str, &str, &str)] = &[
        ("onMissionUpdate", "mission", "MissionID"),
        ("onStepUpdate", "step", "StepID"),
        ("onObjectiveUpdate", "objective", "ObjectiveID"),
    ];
    let arg = |e: &Event, key: &str| {
        e.args
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
    };
    events
        .iter()
        .filter_map(|e| {
            let (_, kind, id_key) = KINDS.iter().find(|(n, _, _)| *n == e.name)?;
            Some(format!(
                "{kind} {} -> {}",
                arg(e, id_key)?,
                arg(e, "Status")?
            ))
        })
        .collect()
}

/// The row's net window changes from its samples (F9): opened is what a
/// later good sample shows that the first did not, closed is what any good
/// sample showed that the last does not. `samples` is the windows.json body
/// or its `samples` array.
pub fn windows(samples: Option<&Value>, v: &Variance) -> (Vec<String>, Vec<String>) {
    let list = samples
        .and_then(|s| s.get("samples").or(Some(s)))
        .and_then(Value::as_array);
    let good: Vec<BTreeSet<String>> = list
        .into_iter()
        .flatten()
        .filter(|s| s.get("error").is_none())
        .map(|s| {
            s.get("windows")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .collect();
    let (Some(start), Some(last)) = (good.first(), good.last()) else {
        return (Vec::new(), Vec::new());
    };
    let kept = |w: &&String| !matches_any(&v.ignore_windows, w);
    let opened: BTreeSet<&String> = good[1..]
        .iter()
        .flatten()
        .filter(|w| !start.contains(*w))
        .filter(kept)
        .collect();
    let closed: BTreeSet<&String> = good
        .iter()
        .flatten()
        .filter(|w| !last.contains(*w))
        .filter(kept)
        .collect();
    (
        opened.into_iter().cloned().collect(),
        closed.into_iter().cloned().collect(),
    )
}

#[cfg(test)]
#[path = "normalise_tests.rs"]
mod tests;
