//! `lab_uat_attest`: fill in what the runner cannot observe itself — a
//! SigNoz query's rows, a person's answer, a server read made another
//! way, the server build — then re-grade the row with the same rules.
//!
//! The runner never queries SigNoz: the agent driving it has the SigNoz
//! MCP, so each SigNoz clause is written PENDING with its exact filter and
//! time window, and the agent attests the row count and key rows. The
//! grading of those rows (`min_rows`, `max_rows`, `field op value`) is
//! still done here, by code, not by the agent's judgement.

use std::path::PathBuf;

use serde_json::{json, Value};

use super::clause::grade_signoz;
use super::evidence::{clip, RowEvidence, RunDir, RunManifest, Verdict};
use super::grade::grade;
use super::ledger;
use super::spec::Source;

/// One attestation.
#[derive(Debug, Clone, Default)]
pub struct AttestRequest {
    pub run_dir: PathBuf,
    pub section: Option<String>,
    pub row: Option<String>,
    pub clause: Option<String>,
    /// SigNoz: rows the filter returned in the clause's window.
    pub row_count: Option<u64>,
    /// SigNoz: the key rows (flattened attribute maps).
    pub rows: Vec<Value>,
    /// SigNoz: the filter actually run, if it differs from the clause's.
    pub query_ran: Option<String>,
    /// Human / server: the verdict and what was seen.
    pub verdict: Option<Verdict>,
    pub answer: Option<String>,
    pub by: Option<String>,
    /// Run-level: the server's `service.version`.
    pub server_version: Option<String>,
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Apply one attestation and return the row's new result.
pub fn attest(req: &AttestRequest) -> Result<Value, String> {
    let run = RunDir::open(&req.run_dir)?;
    let mut manifest: RunManifest = run.read_json(&run.run_json())?;
    let mut out = json!({});
    if let Some(v) = &req.server_version {
        manifest.server["service_version"] = json!(v);
        manifest.server["source"] = json!("attested");
        run.write_json(&run.run_json(), &manifest)?;
        out["server_version"] = json!(v);
    }
    if let Some(row_id) = &req.row {
        let mut row = find_row(&run, req.section.as_deref(), row_id)?;
        let clause_id = req
            .clause
            .as_deref()
            .ok_or("attesting a row needs `clause`")?;
        apply(&mut row, clause_id, req)?;
        let g = grade(
            &row.blocked,
            &row.actions,
            &row.clauses,
            row.required_native,
        );
        row.result = g.result;
        row.reasons = g.reasons;
        row.flags = g.flags;
        run.write_json(&run.row_json(&row.section, &row.row_id), &row)?;
        let key = format!("{}/{}", row.section, row.row_id);
        for r in &mut manifest.rows {
            if r.get("row").and_then(Value::as_str) == Some(key.as_str()) {
                r["result"] = json!(row.result.as_str());
            }
        }
        run.write_json(&run.run_json(), &manifest)?;
        out["row"] = json!(key);
        out["result"] = json!(row.result.as_str());
        out["reasons"] = json!(row.reasons);
    }
    let build = manifest
        .server
        .get("service_version")
        .and_then(Value::as_str)
        .unwrap_or("unknown (attest service.version)")
        .to_string();
    let rows = run.rows()?;
    std::fs::write(
        run.ledger_md(),
        ledger::ledger(&rows, &build, &manifest.run_id),
    )
    .map_err(|e| e.to_string())?;
    if let Some(row_id) = &req.row {
        if let Some(r) = rows.iter().find(|r| &r.row_id == row_id) {
            out["ledger_block"] = json!(ledger::block(r, &build));
        }
    }
    Ok(out)
}

fn find_row(run: &RunDir, section: Option<&str>, row_id: &str) -> Result<RowEvidence, String> {
    let rows = run.rows()?;
    let hits: Vec<RowEvidence> = rows
        .into_iter()
        .filter(|r| r.row_id == row_id && section.is_none_or(|s| s == r.section))
        .collect();
    match hits.len() {
        1 => Ok(hits.into_iter().next().expect("one row")),
        0 => Err(format!("no row {row_id} in this run")),
        _ => Err(format!(
            "row id {row_id} is in several sections: pass `section`"
        )),
    }
}

fn apply(row: &mut RowEvidence, clause_id: &str, req: &AttestRequest) -> Result<(), String> {
    let c = row
        .clauses
        .iter_mut()
        .find(|c| c.id == clause_id)
        .ok_or_else(|| format!("row {} has no clause {clause_id}", row.row_id))?;
    let record = json!({
        "clause": clause_id,
        "host_ms": now_ms(),
        "by": req.by,
        "row_count": req.row_count,
        "query_ran": req.query_ran,
        "verdict": req.verdict,
        "answer": req.answer,
    });
    match c.source {
        Source::Signoz => {
            let n = req
                .row_count
                .ok_or("a SigNoz attestation needs row_count")?;
            // Re-parse the clause's grading fields from the stored query:
            // the spec is not at hand here, so the runner kept them.
            let spec = super::spec::ExpectSpec {
                id: c.id.clone(),
                text: c.text.clone(),
                source: Source::Signoz,
                required: c.required,
                filter: c
                    .query
                    .as_ref()
                    .and_then(|q| q.get("filter"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                min_rows: stored(c.query.as_ref(), "min_rows").and_then(|v| v.as_u64()),
                max_rows: stored(c.query.as_ref(), "max_rows").and_then(|v| v.as_u64()),
                field: stored(c.query.as_ref(), "field")
                    .and_then(|v| v.as_str().map(str::to_string)),
                op: stored(c.query.as_ref(), "op").and_then(|v| serde_json::from_value(v).ok()),
                value: stored(c.query.as_ref(), "value"),
                tolerance: stored(c.query.as_ref(), "tolerance").and_then(|v| v.as_f64()),
                ..blank_expect()
            };
            c.verdict = grade_signoz(&spec, n, &req.rows)?;
            c.observed = json!({ "row_count": n, "key_rows": clip(&json!(req.rows), 400), "query_ran": req.query_ran });
            c.detail = None;
        }
        Source::Human | Source::Server => {
            let v = req
                .verdict
                .ok_or("a human or server attestation needs verdict (pass or fail)")?;
            if !matches!(v, Verdict::Pass | Verdict::Fail) {
                return Err("verdict must be pass or fail".into());
            }
            c.verdict = v;
            c.observed = json!({ "answer": req.answer, "by": req.by });
            c.detail = None;
        }
        other => {
            return Err(format!(
                "a {other:?} clause is observed by the runner itself; re-run the row instead"
            ))
        }
    }
    c.evaluated_ms = now_ms();
    row.attestations.push(record);
    Ok(())
}

fn stored(q: Option<&Value>, key: &str) -> Option<Value> {
    q.and_then(|q| q.get("grading"))
        .and_then(|g| g.get(key))
        .cloned()
        .filter(|v| !v.is_null())
}

fn blank_expect() -> super::spec::ExpectSpec {
    toml::from_str("id = \"\"\ntext = \"\"\nsource = \"signoz\"\nfilter = \"\"")
        .expect("static clause")
}
