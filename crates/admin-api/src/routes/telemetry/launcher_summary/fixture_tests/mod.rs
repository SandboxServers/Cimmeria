//! Offline guard for the operator fixtures in `docs/operations/signoz/`:
//! `launcher-journey.dashboard.json` and `launcher-summary.view.json`.
//!
//! Nothing here talks to a SigNoz. The guard reads the two files, pulls out
//! every key a query names (filter expressions, group-by lists, aggregation
//! expressions, list columns) and checks each against what the ingest
//! really writes: it posts the golden `request-all.json` through
//! `ingest_inner` and takes the field names from the captured rows. A field
//! renamed in `rows.rs` therefore fails here until the fixture follows.
//!
//! One predicate, [`check_fixtures`], holds every rule, and `breakage`
//! feeds deliberately broken copies of the real files through it. A
//! query the guard cannot read (an unparseable expression, a shape other
//! than a logs builder query) is a failure, never a skip.
//!
//! - `queries` — finds the queries in the two JSON shapes.
//! - `expr` — reads a filter or aggregation expression.
//! - `breakage` — the broken fixtures the guard must refuse.
//!
//! What this cannot prove: that SigNoz accepts the JSON. See
//! `docs/operations/signoz/launcher-summary-views.md`.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::routes::dev_session::SESSION_KIND_LAUNCHER_SUMMARY;

use super::dto::{Arch, ErrorCode, Operation, Os, Outcome, Phase, TimedPhase, Verdict};
use super::rows::{
    duration_bucket, EVENT_BATCH, EVENT_PHASE, EVENT_SUMMARY, LAUNCHER_SUMMARY_BATCH_TARGET,
    LAUNCHER_SUMMARY_TARGET,
};
use super::tests::{capture, Env, Harness, REQUEST_ALL};

mod breakage;
mod expr;
mod queries;

use expr::{parse_aggregation, parse_filter, Clause, Expr, Literal, Op};
use queries::{dashboard_queries, text, view_queries, Query};

macro_rules! repo_file {
    ($path:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../", $path))
    };
}

const DASHBOARD: &str = repo_file!("docs/operations/signoz/launcher-journey.dashboard.json");
const VIEW: &str = repo_file!("docs/operations/signoz/launcher-summary.view.json");
/// Where the two service names below are defined. admin-api cannot depend
/// on the server crate, so the last test pins them by source text.
const SERVER_OTEL: &str = repo_file!("crates/server/src/otel.rs");

/// `service.name` of the index the `launcher.summary` rows are routed to
/// (`CLIENT_TARGETS` in `crates/server/src/otel.rs`).
pub(super) const CLIENT_INDEX: &str = "cimmeria-client";
/// `service.name` of the index everything else lands in.
pub(super) const SERVER_INDEX: &str = "cimmeria-server";

/// Keys SigNoz has on every log record. `body` is the row's `message`.
const INTRINSIC_KEYS: &[&str] = &["service.name", "scope_name", "severity_text", "body"];

/// What the dashboard description must say about who is counted.
const CAVEAT: &[&str] = &[
    "opted-in",
    "successfully received",
    "not an all-player funnel",
    "not an install success rate",
    "not a login or world-entry metric",
    "unknown is its own outcome",
];

/// What each dashboard panel's description must keep saying about how to
/// read it, by panel title. These are the points an operator gets wrong
/// from the numbers alone; `docs/operations/signoz/launcher-summary-views.md`
/// explains each.
const PANEL_CAVEATS: &[(&str, &[&str])] = &[
    (
        "Attempts by launcher version, OS and outcome",
        &[
            "exited with code 0",
            "never login or world entry",
            "retry_count + 1 identical failed commands",
        ],
    ),
    (
        "Failures by operation, phase and error code",
        &[
            "reported by a later launcher process",
            "retry_count + 1 identical failed commands",
            "before or at admission",
            "launch_not_started can appear with phase = running",
            "at-least-once approximation",
        ],
    ),
    (
        "Phase durations by operation and phase (sample counts)",
        &[
            "accumulate across the seed and every patch",
            "content verification and promotion after the last unpack",
            "only the starting (preparation) phase is timed",
            "ends at the failure",
            "no timing for the phase that was open",
        ],
    ),
    (
        "Attempt durations by operation",
        &[
            "every launch",
            "length of the play session",
            "unknown attempts",
        ],
    ),
];

/// One required view: the rows it counts and how it splits them.
struct View {
    title: &'static str,
    event: &'static str,
    group_by: &'static [&'static str],
    /// Further `key = 'value'` conditions every counted row must meet.
    also: &'static [(&'static str, &'static str)],
}

/// The five views, by title. The fixtures hold exactly these: a new panel
/// is added here with the `event` it counts, so nobody adds an attempt
/// count over `launcher_phase` rows (one per timed phase, so it would
/// count an attempt several times) without saying so.
const VIEWS: &[View] = &[
    View {
        title: "Attempts by launcher version, OS and outcome",
        event: EVENT_SUMMARY,
        group_by: &["launcher_version", "os", "outcome"],
        also: &[],
    },
    View {
        title: "Failures by operation, phase and error code",
        event: EVENT_SUMMARY,
        group_by: &["operation", "phase", "error_code"],
        also: &[("outcome", "failed")],
    },
    View {
        title: "Phase durations by operation and phase (sample counts)",
        event: EVENT_PHASE,
        group_by: &["operation", "phase", "duration_bucket"],
        also: &[],
    },
    View {
        title: "Attempt durations by operation",
        event: EVENT_SUMMARY,
        group_by: &["operation", "duration_bucket"],
        also: &[],
    },
    View {
        title: "Launcher summary — ingest batches",
        event: EVENT_BATCH,
        group_by: &[],
        also: &[],
    },
];

/// What the ingest wrote under one `event`.
pub(super) struct EventRows {
    target: String,
    levels: BTreeSet<String>,
    /// Every field any row of this event carried. `message` is left out:
    /// the OTLP bridge exports it as the record's body, not an attribute.
    keys: BTreeSet<String>,
}

pub(super) type Emitted = BTreeMap<String, EventRows>;

/// Post the golden request through the real ingest and group the captured
/// rows by `event`.
pub(super) fn emitted_for_the_golden_request() -> Emitted {
    let _env = Env::install();
    let h = Harness::new();
    let (response, rows) = capture(|| h.post(REQUEST_ALL.as_bytes()));
    let results = response.expect("a 200").results;
    assert!(!results.is_empty() && results.iter().all(|v| *v == Verdict::Accepted));

    let mut emitted = Emitted::new();
    for row in &rows {
        let of_event = emitted
            .entry(row.event().to_string())
            .or_insert_with(|| EventRows {
                target: row.target.clone(),
                levels: BTreeSet::new(),
                keys: BTreeSet::new(),
            });
        assert_eq!(of_event.target, row.target, "one event, one target");
        of_event.levels.insert(row.level.to_string());
        let fields = row.fields.keys().filter(|key| *key != "message");
        of_event.keys.extend(fields.cloned());
    }
    emitted
}

/// The index an event's rows are exported to.
fn index_of(event: &str) -> &'static str {
    match event {
        EVENT_BATCH => SERVER_INDEX,
        _ => CLIENT_INDEX,
    }
}

/// The values a key can hold, where the wire contract closes the set.
fn closed_values(event: &str, key: &str) -> Option<BTreeSet<String>> {
    fn all<T: Copy>(values: &[T], as_str: fn(T) -> &'static str) -> Vec<&'static str> {
        values.iter().map(|value| as_str(*value)).collect()
    }
    let values = match key {
        "event" => vec![EVENT_SUMMARY, EVENT_PHASE, EVENT_BATCH],
        "operation" => all(Operation::ALL, Operation::as_str),
        // A phase row's `phase` is a timed phase; a summary's is any phase.
        "phase" if event == EVENT_PHASE => all(TimedPhase::ALL, TimedPhase::as_str),
        "phase" => all(Phase::ALL, Phase::as_str),
        "outcome" => all(Outcome::ALL, Outcome::as_str),
        "error_code" => all(ErrorCode::ALL, ErrorCode::as_str),
        "os" => all(Os::ALL, Os::as_str),
        "arch" => all(Arch::ALL, Arch::as_str),
        "duration_bucket" => [0, 1_000, 10_000, 60_000, 300_000, 1_800_000]
            .map(duration_bucket)
            .to_vec(),
        "cimmeria.session_kind" => vec![SESSION_KIND_LAUNCHER_SUMMARY],
        _ => return None,
    };
    Some(values.into_iter().map(str::to_string).collect())
}

fn known_key(key: &str, event: &str, rows: &EventRows) -> Result<(), String> {
    if INTRINSIC_KEYS.contains(&key) || rows.keys.contains(key) {
        Ok(())
    } else {
        Err(format!("`{key}` is not a field of the {event} rows"))
    }
}

/// The value `key` is fixed to: the filter names it once, with `=` and one
/// string, among the conditions every row must meet. An `OR`, an `IN` or a
/// second mention would let other rows in.
fn pinned(expr: &Expr, key: &str) -> Result<String, String> {
    let mentions = expr.clauses().iter().filter(|c| c.key == key).count();
    let required = expr.required().into_iter().find(|c| c.key == key);
    match required {
        Some(Clause {
            op: Op::Eq, values, ..
        }) if mentions == 1 => match values.as_slice() {
            [Literal::Text(value)] => Ok(value.clone()),
            _ => Err(format!("`{key}` is not compared with a string")),
        },
        _ => Err(format!(
            "`{key}` must be pinned: named once, with `=`, at the top level of the filter"
        )),
    }
}

/// A literal compared with a closed-set key is a member of the set.
fn check_literals(clause: &Clause, event: &str, rows: &EventRows) -> Result<(), String> {
    let key = clause.key.as_str();
    let allowed: BTreeSet<String> = match key {
        "service.name" => [index_of(event).to_string()].into(),
        "scope_name" => [rows.target.clone()].into(),
        "severity_text" => rows.levels.clone(),
        _ => match closed_values(event, key) {
            Some(values) => values,
            None => return Ok(()),
        },
    };
    if let Op::Other(op) = &clause.op {
        return Err(format!(
            "`{key}` holds one of a closed set of values; `{op}` cannot be checked against it"
        ));
    }
    for value in &clause.values {
        match value {
            Literal::Text(text) if allowed.contains(text) => {}
            other => {
                return Err(format!(
                    "`{key}` is never {other:?}: it is one of {allowed:?}"
                ))
            }
        }
    }
    Ok(())
}

/// A query that passed, with what the view table needs to know about it.
#[derive(Debug)]
pub(super) struct Checked {
    title: String,
    event: String,
    group_by: Vec<String>,
    /// The `key = 'value'` conditions every counted row meets.
    required: BTreeMap<String, String>,
    /// Every key the query named, once per use.
    keys: Vec<String>,
}

fn check_query(query: &Query, emitted: &Emitted) -> Result<Checked, String> {
    let at = |why: String| format!("{}: {why}", query.title);
    let expr = parse_filter(&query.filter).map_err(at)?;
    let event = pinned(&expr, "event").map_err(at)?;
    let rows = emitted
        .get(&event)
        .ok_or_else(|| at(format!("no row is emitted with event = '{event}'")))?;
    let service = pinned(&expr, "service.name").map_err(at)?;
    if service != index_of(&event) {
        let index = index_of(&event);
        return Err(at(format!(
            "{event} rows are in the {index} index, not {service}"
        )));
    }

    let mut keys = Vec::new();
    for clause in expr.clauses() {
        known_key(&clause.key, &event, rows).map_err(at)?;
        check_literals(clause, &event, rows).map_err(at)?;
        keys.push(clause.key.clone());
    }
    for key in query.group_by.iter().chain(&query.columns) {
        known_key(key, &event, rows).map_err(at)?;
        keys.push(key.clone());
    }
    for text in &query.aggregations {
        let aggregation = parse_aggregation(text).map_err(at)?;
        for key in &aggregation.keys {
            known_key(key, &event, rows).map_err(at)?;
            keys.push(key.clone());
        }
        let plain = text.split_whitespace().collect::<String>() == "count()";
        if aggregation.function != "count" || !plain {
            return Err(at(format!("`{text}` is not count()")));
        }
    }

    let required = expr
        .required()
        .into_iter()
        .filter_map(|clause| match (&clause.op, clause.values.as_slice()) {
            (Op::Eq, [Literal::Text(value)]) => Some((clause.key.clone(), value.clone())),
            _ => None,
        })
        .collect();
    Ok(Checked {
        title: query.title.clone(),
        event,
        group_by: query.group_by.clone(),
        required,
        keys,
    })
}

/// The whole guard: every query of both fixtures passes [`check_query`],
/// the queries are exactly the five of [`VIEWS`], each counting the rows
/// its title promises, the dashboard says who is counted, and each panel
/// keeps its [`PANEL_CAVEATS`].
pub(super) fn check_fixtures(
    dashboard: &Value,
    view: &Value,
    emitted: &Emitted,
) -> Result<Vec<Checked>, String> {
    let mut queries = dashboard_queries(dashboard)?;
    queries.extend(view_queries(view)?);
    let checked: Vec<Checked> = queries
        .iter()
        .map(|query| check_query(query, emitted))
        .collect::<Result<_, _>>()?;

    for wanted in VIEWS {
        let mut matching = checked.iter().filter(|c| c.title == wanted.title);
        let (Some(found), None) = (matching.next(), matching.next()) else {
            return Err(format!("expected one query titled `{}`", wanted.title));
        };
        let at = |why: String| format!("{}: {why}", wanted.title);
        if found.event != wanted.event {
            return Err(at(format!(
                "it must count {} rows, not {}",
                wanted.event, found.event
            )));
        }
        if found.group_by != wanted.group_by {
            return Err(at(format!("it must group by {:?}", wanted.group_by)));
        }
        for (key, value) in wanted.also {
            if found.required.get(*key).map(String::as_str) != Some(*value) {
                return Err(at(format!("it must require {key} = '{value}'")));
            }
        }
    }
    if let Some(extra) = checked
        .iter()
        .find(|c| VIEWS.iter().all(|wanted| wanted.title != c.title))
    {
        return Err(format!(
            "`{}` is not in VIEWS; add it with the event it counts",
            extra.title
        ));
    }

    let description = text(dashboard, "description")?;
    if let Some(missing) = CAVEAT.iter().find(|phrase| !description.contains(*phrase)) {
        return Err(format!("the cohort caveat lost `{missing}`"));
    }
    // `dashboard_queries` passed, so `widgets` is a list of titled panels.
    let panels = dashboard["widgets"].as_array().into_iter().flatten();
    for panel in panels {
        let title = text(panel, "title")?;
        let at = |why: String| format!("{title}: {why}");
        let Some((_, phrases)) = PANEL_CAVEATS.iter().find(|(wanted, _)| *wanted == title) else {
            return Err(at("it has no PANEL_CAVEATS entry".into()));
        };
        let description = text(panel, "description").map_err(at)?;
        if let Some(missing) = phrases.iter().find(|phrase| !description.contains(*phrase)) {
            return Err(at(format!("the panel description lost `{missing}`")));
        }
    }
    Ok(checked)
}

pub(super) fn fixtures() -> (Value, Value) {
    let parse = |text: &str| -> Value { serde_json::from_str(text).expect("the fixture is JSON") };
    (parse(DASHBOARD), parse(VIEW))
}

/// The fixtures as committed pass, against the fields the golden request
/// really produced, and the scan saw the keys it was meant to see.
#[test]
fn the_operator_fixtures_name_only_fields_the_ingest_emits() {
    let emitted = emitted_for_the_golden_request();
    assert_eq!(
        emitted.keys().collect::<Vec<_>>(),
        [EVENT_PHASE, EVENT_SUMMARY, EVENT_BATCH],
        "the golden request writes all three rows"
    );
    for (event, target) in [
        (EVENT_SUMMARY, LAUNCHER_SUMMARY_TARGET),
        (EVENT_PHASE, LAUNCHER_SUMMARY_TARGET),
        (EVENT_BATCH, LAUNCHER_SUMMARY_BATCH_TARGET),
    ] {
        let rows = &emitted[event];
        assert_eq!(rows.target, target, "{event}");
        assert_eq!(rows.levels, BTreeSet::from(["INFO".to_string()]), "{event}");
        assert!(!rows.keys.contains("message"), "{event}");
        assert!(rows.keys.contains("event"), "{event}");
    }

    let (dashboard, view) = fixtures();
    let checked = check_fixtures(&dashboard, &view, &emitted).unwrap_or_else(|why| panic!("{why}"));

    // Not vacuous: five queries, and the keys they name were extracted.
    assert_eq!(checked.len(), VIEWS.len());
    let uses: Vec<&str> = checked
        .iter()
        .flat_map(|c| c.keys.iter().map(String::as_str))
        .collect();
    let distinct: BTreeSet<&str> = uses.iter().copied().collect();
    let fields = distinct
        .iter()
        .filter(|key| !INTRINSIC_KEYS.contains(key))
        .count();
    assert!(uses.len() >= 30, "only {} key uses scanned", uses.len());
    assert!(
        fields >= 12,
        "only {fields} emitted fields scanned: {distinct:?}"
    );
    for query in &checked {
        assert!(query.keys.len() >= 5, "{query:?}");
        assert_eq!(query.required["service.name"], index_of(&query.event));
        assert_eq!(query.required["scope_name"], emitted[&query.event].target);
    }
}

/// The two `service.name` literals the fixtures are held to are the ones
/// the server exports under. `launcher.summary` being a client target is
/// pinned in the server crate (`logging/client_index_tests.rs`).
#[test]
fn the_index_names_are_the_servers() {
    for line in [
        format!("pub const CLIENT_SERVICE_NAME: &str = \"{CLIENT_INDEX}\";"),
        format!("const DEFAULT_SERVICE_NAME: &str = \"{SERVER_INDEX}\";"),
    ] {
        assert!(
            SERVER_OTEL.contains(&line),
            "otel.rs no longer has `{line}`"
        );
    }
}
