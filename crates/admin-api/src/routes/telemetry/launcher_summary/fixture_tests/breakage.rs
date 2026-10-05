//! The negative half of the operator-fixture guard: broken copies of the
//! real fixtures, each of which `check_fixtures` must refuse for the reason
//! named. The first is the misspelt key. The guard itself, and the test
//! that the committed fixtures pass, are in the parent module.

use serde_json::Value;

use super::{check_fixtures, emitted_for_the_golden_request, fixtures, CLIENT_INDEX, SERVER_INDEX};

/// The dashboard's query at `widget`.
fn data(dashboard: &mut Value, widget: usize) -> &mut Value {
    &mut dashboard["widgets"][widget]["query"]["builder"]["queryData"][0]
}

/// Rewrite the string at `value`; the text to replace must be there.
fn swap(value: &mut Value, from: &str, to: &str) {
    let text = value.as_str().expect("a string");
    assert!(text.contains(from), "`{from}` is not in `{text}`");
    *value = text.replace(from, to).into();
}

fn filter(dashboard: &mut Value, widget: usize) -> &mut Value {
    &mut data(dashboard, widget)["filter"]["expression"]
}

/// A broken fixture and a fragment of the refusal it must draw. Widgets 0
/// to 3 are the four dashboard panels, in the order of `VIEWS`.
type Breakage = (&'static str, fn(&mut Value, &mut Value), &'static str);

const BREAKAGES: &[Breakage] = &[
    // Misspelt keys, in each place a key can sit.
    (
        "misspelt group-by key",
        |d, _| {
            let group = &mut data(d, 0)["groupBy"][0];
            group["key"] = "launcher_verison".into();
            group["name"] = "launcher_verison".into();
        },
        "`launcher_verison` is not a field of the launcher_summary rows",
    ),
    (
        "misspelt filter key",
        |d, _| swap(filter(d, 1), "outcome =", "outcom ="),
        "`outcom` is not a field",
    ),
    (
        "misspelt list column",
        |_, v| {
            swap(
                &mut v["extraData"],
                "client_dropped_expired",
                "client_droped_expired",
            )
        },
        "`client_droped_expired` is not a field of the launcher_summary_batch rows",
    ),
    (
        "misspelt aggregation argument",
        |d, _| data(d, 3)["aggregations"][0]["expression"] = "count_distinct(atempt_id)".into(),
        "`atempt_id` is not a field",
    ),
    (
        "a summary-only field on the phase panel",
        |d, _| {
            let group = &mut data(d, 2)["groupBy"][0];
            group["key"] = "outcome".into();
            group["name"] = "outcome".into();
        },
        "`outcome` is not a field of the launcher_phase rows",
    ),
    (
        "the row's message as a key",
        |d, _| {
            swap(
                filter(d, 0),
                "event =",
                "message = 'launcher summary' AND event =",
            )
        },
        "`message` is not a field",
    ),
    // Literals outside the closed sets.
    (
        "unknown outcome",
        |d, _| swap(filter(d, 1), "'failed'", "'failure'"),
        "`outcome` is never",
    ),
    (
        "pattern on a closed-set key",
        |d, _| swap(filter(d, 1), "outcome = 'failed'", "outcome LIKE 'fail%'"),
        "cannot be checked",
    ),
    (
        "a phase the launcher does not time, on the phase panel",
        |d, _| swap(filter(d, 2), "event =", "phase != 'admission' AND event ="),
        "`phase` is never",
    ),
    (
        "summary panel in the server index",
        |d, _| swap(filter(d, 0), CLIENT_INDEX, SERVER_INDEX),
        "launcher_summary rows are in the cimmeria-client index",
    ),
    (
        "batch view in the client index",
        |_, v| {
            let spec = &mut v["compositeQuery"]["queries"][0]["spec"];
            swap(
                &mut spec["filter"]["expression"],
                SERVER_INDEX,
                CLIENT_INDEX,
            );
        },
        "launcher_summary_batch rows are in the cimmeria-server index",
    ),
    (
        "wrong scope",
        |d, _| swap(filter(d, 0), "'launcher.summary'", "'launcher.ingest'"),
        "`scope_name` is never",
    ),
    (
        "no service clause",
        |d, _| swap(filter(d, 0), "service.name = 'cimmeria-client' AND ", ""),
        "`service.name` must be pinned",
    ),
    // Aggregations.
    (
        "a percentile",
        |d, _| data(d, 3)["aggregations"][0]["expression"] = "p95(duration_ms)".into(),
        "is not count()",
    ),
    (
        "unparseable aggregation",
        |d, _| data(d, 3)["aggregations"][0]["expression"] = "count(".into(),
        "is not one `name(args)` call",
    ),
    (
        "a metric aggregation",
        |d, _| data(d, 3)["aggregations"][0]["metricName"] = "launcher_attempts_total".into(),
        "unknown key `metricName`",
    ),
    (
        "no aggregation",
        |d, _| data(d, 3)["aggregations"] = Value::Array(Vec::new()),
        "no aggregation",
    ),
    // Counting the wrong rows.
    (
        "the phase panel counting attempts",
        |d, _| swap(filter(d, 2), "'launcher_phase'", "'launcher_summary'"),
        "it must count launcher_phase rows",
    ),
    (
        "an attempt panel counting phase rows",
        |d, _| swap(filter(d, 3), "'launcher_summary'", "'launcher_phase'"),
        "it must count launcher_summary rows",
    ),
    (
        "an attempt panel widened with OR",
        |d, _| {
            let both = "(event = 'launcher_summary' OR event = 'launcher_phase')";
            swap(filter(d, 3), "event = 'launcher_summary'", both);
        },
        "`event` must be pinned",
    ),
    (
        "an attempt panel widened with IN",
        |d, _| {
            swap(
                filter(d, 3),
                "event = 'launcher_summary'",
                "event IN ('launcher_summary', 'launcher_phase')",
            )
        },
        "`event` must be pinned",
    ),
    (
        "the failure panel counting every outcome",
        |d, _| swap(filter(d, 1), " AND outcome = 'failed'", ""),
        "it must require outcome = 'failed'",
    ),
    (
        "a regrouped panel",
        |d, _| {
            data(d, 0)["groupBy"].as_array_mut().unwrap().pop();
        },
        "it must group by",
    ),
    // The five views.
    (
        "a panel removed",
        |d, _| {
            d["widgets"].as_array_mut().unwrap().remove(3);
            d["layout"].as_array_mut().unwrap().remove(3);
        },
        "expected one query titled `Attempt durations by operation`",
    ),
    (
        "a panel retitled",
        |d, _| d["widgets"][0]["title"] = "Attempts".into(),
        "expected one query titled",
    ),
    (
        "a panel the table does not know",
        |d, _| {
            let mut extra = d["widgets"][0].clone();
            extra["title"] = "Attempts by architecture".into();
            extra["id"] = "attempts-by-arch".into();
            d["widgets"].as_array_mut().unwrap().push(extra);
            let mut slot = d["layout"][0].clone();
            slot["i"] = "attempts-by-arch".into();
            d["layout"].as_array_mut().unwrap().push(slot);
        },
        "is not in VIEWS",
    ),
    (
        "a panel with no layout slot",
        |d, _| {
            d["layout"].as_array_mut().unwrap().pop();
        },
        "name different panels",
    ),
    (
        "the caveat cut",
        |d, _| swap(&mut d["description"], "not an all-player funnel, ", ""),
        "the cohort caveat lost `not an all-player funnel`",
    ),
    (
        "a panel's reading note cut",
        |d, _| {
            swap(
                &mut d["widgets"][2]["description"],
                " and ends at the failure",
                "",
            )
        },
        "the panel description lost `ends at the failure`",
    ),
    (
        "a launch described as a login",
        |d, _| {
            swap(
                &mut d["widgets"][0]["description"],
                "it is never login or world entry",
                "the player logged in",
            )
        },
        "the panel description lost `never login or world entry`",
    ),
    (
        "a panel with no description",
        |d, _| {
            d["widgets"][3]
                .as_object_mut()
                .unwrap()
                .remove("description");
        },
        "Attempt durations by operation: `description` is not a string",
    ),
    // Queries the guard cannot read.
    (
        "unterminated string",
        |d, _| swap(filter(d, 0), "'launcher_summary'", "'launcher_summary"),
        "unterminated string",
    ),
    (
        "a bare search word",
        |d, _| swap(filter(d, 0), "event =", "timeout AND event ="),
        "no operator this guard reads",
    ),
    (
        "empty filter",
        |d, _| *filter(d, 0) = "".into(),
        "empty filter",
    ),
    (
        "filter in the old `filters` shape",
        |d, _| data(d, 0)["filters"] = serde_json::json!({ "items": [], "op": "AND" }),
        "unknown key `filters`",
    ),
    (
        "a ClickHouse panel",
        |d, _| d["widgets"][0]["query"]["queryType"] = "clickhouse_sql".into(),
        "the guard reads only `builder`",
    ),
    (
        "raw SQL beside the builder query",
        |d, _| d["widgets"][0]["query"]["clickhouse_sql"][0]["query"] = "SELECT 1".into(),
        "the guard reads only ``",
    ),
    (
        "a metrics panel",
        |d, _| data(d, 0)["dataSource"] = "metrics".into(),
        "the guard reads only `logs`",
    ),
    (
        "a formula",
        |d, _| {
            let formula = serde_json::json!([{ "expression": "A/B", "queryName": "F1" }]);
            d["widgets"][0]["query"]["builder"]["queryFormulas"] = formula;
        },
        "no formula",
    ),
    (
        "an ordering the guard does not read",
        |d, _| data(d, 0)["orderBy"] = serde_json::json!([{ "columnName": "os", "order": "asc" }]),
        "`orderBy` is not empty",
    ),
    (
        "a view that aggregates",
        |_, v| {
            let spec = &mut v["compositeQuery"]["queries"][0]["spec"];
            spec["aggregations"] = serde_json::json!([{ "expression": "count()" }]);
        },
        "unknown key `aggregations`",
    ),
    (
        "view columns that are not JSON",
        |_, v| v["extraData"] = "{".into(),
        "`extraData` is not JSON",
    ),
];

/// Each breakage, applied to a copy of the real fixtures, is refused by
/// `check_fixtures` for the reason named. The control is the unbroken pair,
/// which the same call accepts.
#[test]
fn a_broken_fixture_is_refused_by_the_same_check() {
    let emitted = emitted_for_the_golden_request();
    let (dashboard, view) = fixtures();
    check_fixtures(&dashboard, &view, &emitted).expect("the control passes");

    for (name, breakage, reason) in BREAKAGES {
        let (mut dashboard, mut view) = (dashboard.clone(), view.clone());
        breakage(&mut dashboard, &mut view);
        match check_fixtures(&dashboard, &view, &emitted) {
            Ok(_) => panic!("{name}: the broken fixture passed"),
            Err(why) => assert!(why.contains(reason), "{name}: refused, but with `{why}`"),
        }
    }
}
