//! Ledger output: the unified-uat.md "Recording results" block per row,
//! the per-session summary table, and the per-section report.
//!
//! The block keeps the guide's field names and order so a coordinator
//! pastes it into the campaign ledger unchanged. It adds one line,
//! `Automation:`, with the native level used and the bundle path. The
//! `Result:` line uses the guide's words (PASS, FAIL, BLOCKED, SKIPPED)
//! and, for the automation-only outcomes, NEEDS_HUMAN, UNVERIFIED and
//! NATIVE_SHORTFALL, each followed by the reason: none of those is a pass.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use super::evidence::{RowEvidence, RowResult, Verdict};

fn character_line(c: &Value) -> String {
    let name = c.get("name").and_then(Value::as_str).unwrap_or("unknown");
    let arch = c.get("archetype").and_then(Value::as_str).unwrap_or("?");
    let level = c.get("level").map_or_else(
        || "?".to_string(),
        |l| match l {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        },
    );
    format!("{name}, {arch}, {level}")
}

fn result_line(r: &RowEvidence) -> String {
    let first = r.reasons.first().cloned().unwrap_or_default();
    match r.result {
        RowResult::Pass | RowResult::Fail => {
            if r.result == RowResult::Fail && !first.is_empty() {
                format!("FAIL ({first})")
            } else {
                r.result.as_str().to_string()
            }
        }
        _ if first.is_empty() => r.result.as_str().to_string(),
        _ => format!("{} ({first})", r.result.as_str()),
    }
}

/// What the row saw, one clause per `;`-separated item.
pub fn saw_line(r: &RowEvidence) -> String {
    let mut parts = Vec::new();
    for c in &r.clauses {
        let tag = match c.verdict {
            Verdict::Pass => "ok",
            Verdict::Fail => "FAILED",
            Verdict::Pending => "pending",
            Verdict::Unverified => "unverified",
            Verdict::NeedsHuman => "for a human",
            Verdict::Blocked => "blocked",
        };
        let obs = summarize_observed(&c.observed);
        parts.push(if obs.is_empty() {
            format!("[{tag}] {}", c.text)
        } else {
            format!("[{tag}] {}: {obs}", c.text)
        });
    }
    if parts.is_empty() {
        r.reasons.join("; ")
    } else {
        parts.join("; ")
    }
}

/// A short human reading of an observation.
fn summarize_observed(v: &Value) -> String {
    if let Some(lines) = v.get("matching_lines").and_then(Value::as_array) {
        let shown: Vec<String> = lines
            .iter()
            .take(2)
            .filter_map(Value::as_str)
            .map(|s| format!("\"{}\"", s.chars().take(90).collect::<String>()))
            .collect();
        return if shown.is_empty() {
            "no matching line".to_string()
        } else {
            format!("{} matching: {}", lines.len(), shown.join(", "))
        };
    }
    if let Some(n) = v.get("row_count") {
        return format!("{n} SigNoz row(s)");
    }
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.chars().take(120).collect(),
        other => other.to_string().chars().take(120).collect(),
    }
}

/// One "Recording results" block.
pub fn block(r: &RowEvidence, build: &str) -> String {
    let time = r
        .started_utc
        .get(..16)
        .unwrap_or(&r.started_utc)
        .replace('T', " ");
    let account = if r.account_kind == "gm" {
        "GM"
    } else {
        "non-GM"
    };
    let bug = match &r.anchor {
        Some(a) => match a.bookmark_id {
            Some(id) => format!("{} (bookmark_id {id})", a.note),
            None => format!("{} (no bookmark reply seen)", a.note),
        },
        None => "none".to_string(),
    };
    let known = if r.known_issues.is_empty() {
        "none".to_string()
    } else {
        r.known_issues.join(", ")
    };
    let native = match r.native_used {
        Some(t) => format!(
            "native {} (row needs {})",
            t.as_str(),
            r.required_native.as_str()
        ),
        None => format!(
            "no step drove the client (row needs {})",
            r.required_native.as_str()
        ),
    };
    format!(
        "```text\n\
System:        {system}\n\
Step id:       {id}\n\
Result:        {result}\n\
Time:          {time} UTC\n\
Character:     {character}  Account: {account}\n\
Build:         {build}\n\
.bug note:     {bug}\n\
Saw:           {saw}\n\
Expected:      {expected}\n\
After relog:   {relog}\n\
Known issue?:  {known}\n\
Automation:    {native}; bundle rows/{section}/{id}.json\n\
```\n",
        system = r.system,
        id = r.row_id,
        result = result_line(r),
        character = character_line(&r.character),
        saw = saw_line(r),
        expected = r.expected,
        relog = r.relog.as_deref().unwrap_or("not tried"),
        section = r.section,
    )
}

/// The whole ledger file: a summary table, then every block.
pub fn ledger(rows: &[RowEvidence], build: &str, run_id: &str) -> String {
    let mut s = format!(
        "# UAT run {run_id}\n\nBuild: {build}. Generated by `lab_uat_run`; the bundle JSON next to this file is the evidence.\n\n\
| System | Step | Result | Time | Character | `.bug` note |\n|---|---|---|---|---|---|\n"
    );
    for r in rows {
        let bug = r
            .anchor
            .as_ref()
            .map_or_else(|| "none".to_string(), |a| a.note.clone());
        s.push_str(&format!(
            "| {} | {} | {} | {} UTC | {} | {} |\n",
            r.system,
            r.row_id,
            r.result.as_str(),
            r.started_utc
                .get(..16)
                .unwrap_or(&r.started_utc)
                .replace('T', " "),
            character_line(&r.character),
            bug
        ));
    }
    for r in rows {
        s.push('\n');
        s.push_str(&block(r, build));
    }
    s
}

/// Counts per section and the non-pass rows with their first reason.
pub fn report(rows: &[RowEvidence]) -> Value {
    let mut sections: BTreeMap<String, BTreeMap<&'static str, u32>> = BTreeMap::new();
    let mut open = Vec::new();
    for r in rows {
        *sections
            .entry(r.section.clone())
            .or_default()
            .entry(r.result.as_str())
            .or_default() += 1;
        if r.result != RowResult::Pass {
            open.push(json!({
                "section": r.section,
                "row": r.row_id,
                "result": r.result.as_str(),
                "reason": r.reasons.first(),
            }));
        }
    }
    let mut totals: BTreeMap<&'static str, u32> = BTreeMap::new();
    for counts in sections.values() {
        for (k, n) in counts {
            *totals.entry(k).or_default() += n;
        }
    }
    json!({ "rows": rows.len(), "totals": totals, "sections": sections, "not_passed": open })
}

/// The report as a Markdown table.
pub fn report_markdown(report: &Value) -> String {
    const COLS: [&str; 7] = [
        "PASS",
        "FAIL",
        "BLOCKED",
        "SKIPPED",
        "NEEDS_HUMAN",
        "UNVERIFIED",
        "NATIVE_SHORTFALL",
    ];
    let mut s = format!(
        "| Section | {} |\n|---|{}\n",
        COLS.join(" | "),
        "---|".repeat(COLS.len())
    );
    if let Some(secs) = report.get("sections").and_then(Value::as_object) {
        for (name, counts) in secs {
            let cells: Vec<String> = COLS
                .iter()
                .map(|c| {
                    counts
                        .get(*c)
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        .to_string()
                })
                .collect();
            s.push_str(&format!("| {name} | {} |\n", cells.join(" | ")));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uat::evidence::{Anchor, ClauseResult, BUNDLE_SCHEMA};
    use crate::uat::spec::Source;
    use crate::uat::tier::Tier;

    fn row(result: RowResult) -> RowEvidence {
        RowEvidence {
            schema: BUNDLE_SCHEMA,
            run_id: "r1".into(),
            section: "gm-parity".into(),
            system: "GM console command parity".into(),
            row_id: "M1-1".into(),
            title: "t".into(),
            guide_ref: "g".into(),
            ledger_ref: "l".into(),
            expected: "Each answers in chat.".into(),
            result,
            reasons: if result == RowResult::Pass {
                vec![]
            } else {
                vec!["needs client_cache_files".into()]
            },
            flags: vec![],
            required_native: Tier::N1,
            native_used: Some(Tier::N1),
            character: json!({"name": "Labone", "archetype": "Soldier", "level": 3}),
            account_kind: "gm".into(),
            started_utc: "2026-09-29T14:05:07Z".into(),
            ended_utc: "2026-09-29T14:05:30Z".into(),
            host_started_ms: 0,
            host_ended_ms: 0,
            anchor: Some(Anchor {
                note: "uat M1-1".into(),
                bookmark_id: Some(1790),
                ..Default::default()
            }),
            actions: vec![],
            clauses: vec![ClauseResult {
                id: "help".into(),
                text: ".help answers".into(),
                source: Source::Chat,
                required: true,
                expected: "a new chat line".into(),
                verdict: Verdict::Pass,
                observed: json!({"matching_lines": ["Commands: .help"], "match_count": 1}),
                detail: None,
                evaluated_ms: 0,
                query: None,
                evidence_refs: vec![],
            }],
            attachments: vec![],
            known_issues: vec![],
            relog: None,
            notes: None,
            vars: json!({}),
            attestations: vec![],
            blocked: vec![],
        }
    }

    /// The block keeps the guide's eleven field names, in order.
    #[test]
    fn block_matches_the_recording_results_template() {
        let b = block(&row(RowResult::Pass), "abc123");
        let fields: Vec<&str> = b
            .lines()
            .filter_map(|l| l.split_once(':').map(|(k, _)| k))
            .collect();
        assert_eq!(
            fields,
            [
                "System",
                "Step id",
                "Result",
                "Time",
                "Character",
                "Build",
                ".bug note",
                "Saw",
                "Expected",
                "After relog",
                "Known issue?",
                "Automation"
            ]
        );
        assert!(b.contains("Result:        PASS\n"));
        assert!(b.contains("Time:          2026-09-29 14:05 UTC"));
        assert!(b.contains("Labone, Soldier, 3  Account: GM"));
        assert!(b.contains("uat M1-1 (bookmark_id 1790)"));
        assert!(b.contains("[ok] .help answers: 1 matching: \"Commands: .help\""));
    }

    #[test]
    fn a_blocked_row_names_its_reason() {
        let b = block(&row(RowResult::Blocked), "abc");
        assert!(b.contains("Result:        BLOCKED (needs client_cache_files)"));
    }

    #[test]
    fn report_counts_per_section() {
        let rows = [row(RowResult::Pass), row(RowResult::Blocked)];
        let r = report(&rows);
        assert_eq!(r["sections"]["gm-parity"]["PASS"], 1);
        assert_eq!(r["totals"]["BLOCKED"], 1);
        assert_eq!(r["not_passed"].as_array().unwrap().len(), 1);
        let md = report_markdown(&r);
        assert!(md.contains("| gm-parity | 1 | 0 | 1 |"), "{md}");
    }
}
