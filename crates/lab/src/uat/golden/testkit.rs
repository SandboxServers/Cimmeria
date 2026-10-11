//! Test-only bundle builders: a row's evidence and a run directory laid
//! out as the runner writes one, so golden tests need no lab.

use std::path::Path;

use serde_json::{json, Value};

use crate::uat::evidence::{
    path_safe, Attachment, ClauseResult, RowEvidence, RowResult, RunDir, Verdict, BUNDLE_SCHEMA,
};
use crate::uat::spec::Source;
use crate::uat::tier::Tier;

/// A row's evidence with only its id, result and clauses set; every other
/// field is empty or zero. Clauses are `(id, source, required, verdict)`.
pub fn row_evidence(
    row: &str,
    result: RowResult,
    clauses: &[(&str, Source, bool, Verdict)],
) -> RowEvidence {
    RowEvidence {
        schema: BUNDLE_SCHEMA,
        run_id: String::new(),
        section: String::new(),
        system: String::new(),
        row_id: row.to_string(),
        title: String::new(),
        guide_ref: String::new(),
        ledger_ref: String::new(),
        expected: String::new(),
        result,
        reasons: Vec::new(),
        flags: Vec::new(),
        required_native: Tier::N1,
        native_used: None,
        character: Value::Null,
        account_kind: String::new(),
        started_utc: String::new(),
        ended_utc: String::new(),
        host_started_ms: 0,
        host_ended_ms: 0,
        anchor: None,
        actions: Vec::new(),
        clauses: clauses
            .iter()
            .map(|(id, source, required, verdict)| ClauseResult {
                id: id.to_string(),
                text: String::new(),
                source: *source,
                required: *required,
                expected: String::new(),
                verdict: *verdict,
                observed: Value::Null,
                detail: None,
                evaluated_ms: 0,
                query: None,
                evidence_refs: Vec::new(),
                client: None,
            })
            .collect(),
        attachments: Vec::new(),
        known_issues: Vec::new(),
        relog: None,
        notes: None,
        vars: json!({}),
        attestations: Vec::new(),
        blocked: Vec::new(),
    }
}

/// A run directory under `root` holding `rows` of `section`: `run.json`
/// (one `specs[]` entry, server build `testbuild`, an SGW.exe hash), each
/// row JSON, and each present attachment as `(tap, entities, windows)`,
/// listed in the row's `attachments[]` under the names the runner uses.
pub fn write_run(
    root: &Path,
    run_id: &str,
    section: &str,
    rows: &[(RowEvidence, Option<Value>, Option<Value>, Option<Value>)],
) -> RunDir {
    let run = RunDir::create(root, "2026-10-11", Some("testbuild"), run_id).unwrap();
    let manifest = json!({
        "schema": BUNDLE_SCHEMA,
        "run_id": run_id,
        "server": { "service_version": "testbuild", "source": "arg" },
        "client": { "sgw_exe": { "sha256": "sgw-sha" } },
        "specs": [
            { "path": format!("{section}.toml"), "sha256": format!("{section}-sha"), "section": section },
        ],
    });
    run.write_json(&run.run_json(), &manifest).unwrap();
    for (evidence, tap, entities, windows) in rows {
        let mut ev = evidence.clone();
        ev.run_id = run_id.to_string();
        ev.section = section.to_string();
        let attachments = [
            ("packet_tap", "packet-tap.json", tap),
            ("entities", "entities.json", entities),
            ("windows", "windows.json", windows),
        ];
        for (name, file, body) in attachments {
            let Some(body) = body else { continue };
            let rel = format!(
                "rows/{}/{}/{file}",
                path_safe(section),
                path_safe(&ev.row_id)
            );
            run.write_json(&run.root.join(&rel), body).unwrap();
            ev.attachments.push(Attachment {
                name: name.to_string(),
                path: rel,
                tool: "test".to_string(),
                host_ms: 0,
            });
        }
        run.write_json(&run.row_json(section, &ev.row_id), &ev)
            .unwrap();
    }
    run
}
