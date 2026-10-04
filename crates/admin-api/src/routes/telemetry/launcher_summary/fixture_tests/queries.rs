//! Finds the queries in the two fixture shapes: a dashboard export
//! (`widgets[].query.builder.queryData[]`) and a saved Logs Explorer view
//! (`compositeQuery.queries[].spec` plus the columns in `extraData`).
//!
//! Both readers are closed: a query type, data source or key they do not
//! know is an error, because it may hold a query the guard would not scan.

use std::collections::BTreeSet;

use serde_json::Value;

/// One query of a fixture, reduced to the strings that name keys.
#[derive(Debug)]
pub(super) struct Query {
    pub(super) title: String,
    pub(super) filter: String,
    pub(super) group_by: Vec<String>,
    pub(super) aggregations: Vec<String>,
    pub(super) columns: Vec<String>,
}

pub(super) fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .ok_or_else(|| format!("`{key}` is not a string"))
}

fn list<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    value[key]
        .as_array()
        .ok_or_else(|| format!("`{key}` is not a list"))
}

/// `value` is an object with no key outside `known`: a key the guard has
/// never seen may hold a query it does not scan.
fn only_keys(value: &Value, known: &[&str]) -> Result<(), String> {
    let object = value.as_object().ok_or("expected an object")?;
    match object.keys().find(|key| !known.contains(&key.as_str())) {
        Some(key) => Err(format!("unknown key `{key}`; teach the guard to read it")),
        None => Ok(()),
    }
}

fn is(value: &Value, key: &str, wanted: &str) -> Result<(), String> {
    match &value[key] {
        Value::String(found) if found == wanted => Ok(()),
        found => Err(format!(
            "`{key}` is {found}; the guard reads only `{wanted}`"
        )),
    }
}

fn names(entries: &[Value], key: &str) -> Result<Vec<String>, String> {
    entries
        .iter()
        .map(|entry| text(entry, key).map(str::to_string))
        .collect()
}

/// The keys of a dashboard widget's builder query, as the neighbouring
/// fixtures have them.
const QUERY_DATA_KEYS: &[&str] = &[
    "aggregations",
    "dataSource",
    "disabled",
    "expression",
    "filter",
    "functions",
    "groupBy",
    "having",
    "legend",
    "limit",
    "offset",
    "orderBy",
    "pageSize",
    "queryName",
    "reduceTo",
    "selectColumns",
    "stepInterval",
];

/// Every query of a dashboard export (`widgets[].query.builder.queryData[]`).
pub(super) fn dashboard_queries(doc: &Value) -> Result<Vec<Query>, String> {
    let widgets = list(doc, "widgets")?;
    let ids = |entries: &[Value], key: &str| -> Result<BTreeSet<String>, String> {
        Ok(names(entries, key)?.into_iter().collect())
    };
    if ids(widgets, "id")? != ids(list(doc, "layout")?, "i")? {
        return Err("`layout` and `widgets` name different panels".into());
    }
    let mut queries = Vec::new();
    for widget in widgets {
        let title = text(widget, "title")?;
        let at = |why: String| format!("{title}: {why}");
        let query = &widget["query"];
        only_keys(query, &["builder", "clickhouse_sql", "promql", "queryType"]).map_err(at)?;
        is(query, "queryType", "builder").map_err(at)?;
        for raw in ["clickhouse_sql", "promql"] {
            for entry in list(query, raw).map_err(at)? {
                is(entry, "query", "").map_err(at)?;
            }
        }
        let builder = &query["builder"];
        only_keys(builder, &["queryData", "queryFormulas"]).map_err(at)?;
        let columns = names(list(widget, "selectedLogFields").map_err(at)?, "name").map_err(at)?;
        let data = list(builder, "queryData").map_err(at)?;
        if data.is_empty() || !list(builder, "queryFormulas").map_err(at)?.is_empty() {
            return Err(at("expected plain queries and no formula".into()));
        }
        for entry in data {
            only_keys(entry, QUERY_DATA_KEYS).map_err(at)?;
            is(entry, "dataSource", "logs").map_err(at)?;
            for unread in ["functions", "having", "orderBy", "selectColumns"] {
                if !list(entry, unread).map_err(at)?.is_empty() {
                    return Err(at(format!("`{unread}` is not empty; the guard reads none")));
                }
            }
            let groups = list(entry, "groupBy").map_err(at)?;
            let group_by = names(groups, "key").map_err(at)?;
            if names(groups, "name").map_err(at)? != group_by {
                return Err(at("a group-by `name` differs from its `key`".into()));
            }
            let aggregations = list(entry, "aggregations").map_err(at)?;
            for aggregation in aggregations {
                only_keys(aggregation, &["expression"]).map_err(at)?;
            }
            if aggregations.is_empty() {
                return Err(at("no aggregation".into()));
            }
            queries.push(Query {
                title: title.to_string(),
                filter: text(&entry["filter"], "expression")
                    .map_err(at)?
                    .to_string(),
                group_by,
                aggregations: names(aggregations, "expression").map_err(at)?,
                columns: columns.clone(),
            });
        }
    }
    Ok(queries)
}

/// Every query of a saved Logs Explorer view, which is a list: a filter
/// and the columns in `extraData`.
pub(super) fn view_queries(doc: &Value) -> Result<Vec<Query>, String> {
    let title = text(doc, "name")?;
    let at = |why: String| format!("{title}: {why}");
    let top = [
        "name",
        "sourcePage",
        "category",
        "tags",
        "compositeQuery",
        "extraData",
    ];
    only_keys(doc, &top).map_err(at)?;
    let composite = &doc["compositeQuery"];
    only_keys(composite, &["queryType", "panelType", "queries"]).map_err(at)?;
    is(composite, "queryType", "builder").map_err(at)?;
    is(composite, "panelType", "list").map_err(at)?;
    let extra: Value = serde_json::from_str(text(doc, "extraData").map_err(at)?)
        .map_err(|_| at("`extraData` is not JSON".into()))?;
    only_keys(&extra, &["selectColumns"]).map_err(at)?;
    let columns = names(list(&extra, "selectColumns").map_err(at)?, "name").map_err(at)?;

    let mut queries = Vec::new();
    for entry in list(composite, "queries").map_err(at)? {
        only_keys(entry, &["type", "spec"]).map_err(at)?;
        is(entry, "type", "builder_query").map_err(at)?;
        let spec = &entry["spec"];
        let known = [
            "name",
            "signal",
            "source",
            "stepInterval",
            "filter",
            "having",
        ];
        only_keys(spec, &known).map_err(at)?;
        is(spec, "signal", "logs").map_err(at)?;
        is(&spec["having"], "expression", "").map_err(at)?;
        queries.push(Query {
            title: title.to_string(),
            filter: text(&spec["filter"], "expression").map_err(at)?.to_string(),
            group_by: Vec::new(),
            aggregations: Vec::new(),
            columns: columns.clone(),
        });
    }
    Ok(queries)
}
