//! A committed golden as what `lab golden show` prints. Ports
//! `Get-GoldenSummary` and `Format-GoldenSummary` (the removed PowerShell golden library).

use serde::Serialize;
use serde_json::Value;

use super::reply::text_of;
use crate::ui::{Tone, Ui};

/// Section, rows, total ordered events, build, recorded time and evidence run
/// ids. Field order is the JSON key order; absent options are left out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Summary {
    pub section: String,
    pub rows: Vec<String>,
    pub events: usize,
    pub server: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sgw_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recorded: Option<String>,
    pub runs: Vec<String>,
}

fn truthy(value: &Value) -> bool {
    !value.is_null() && value.as_str() != Some("")
}

fn non_empty(value: Option<&Value>) -> Option<String> {
    let text = text_of(value);
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// The summary of a parsed golden file.
pub fn summarize(golden: &Value) -> Summary {
    let events = golden
        .pointer("/fingerprint/rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|row| !row.is_null())
                .map(|row| {
                    row.get("events")
                        .and_then(Value::as_array)
                        .map(|events| events.iter().filter(|e| !e.is_null()).count())
                        .unwrap_or(0)
                })
                .sum()
        })
        .unwrap_or(0);
    let rows = golden
        .get("rows")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter(|r| truthy(r))
                .map(|r| text_of(Some(r)))
                .collect()
        })
        .unwrap_or_default();
    let runs = golden
        .get("evidence")
        .and_then(Value::as_array)
        .map(|evidence| {
            evidence
                .iter()
                .filter(|e| !e.is_null())
                .map(|e| text_of(e.get("run_id")))
                .collect()
        })
        .unwrap_or_default();
    Summary {
        section: text_of(golden.get("section")),
        rows,
        events,
        server: text_of(golden.pointer("/build/server")),
        sgw_sha256: non_empty(golden.pointer("/build/sgw_sha256")),
        recorded: non_empty(golden.get("recorded_utc")),
        runs,
    }
}

/// The `--json` object of `show`: `{"ok":true,"exit":0,` then the summary keys.
pub fn show_json(summary: &Summary) -> String {
    #[derive(Serialize)]
    struct Shown<'a> {
        ok: bool,
        exit: u8,
        #[serde(flatten)]
        summary: &'a Summary,
    }
    serde_json::to_string(&Shown {
        ok: true,
        exit: 0,
        summary,
    })
    .unwrap_or_default()
}

/// Plain mode: the lines the PowerShell printed, joined by `\n`.
pub fn summary_plain(s: &Summary) -> String {
    let mut lines = vec![
        format!(
            "golden {}: {} rows, {} events",
            s.section,
            s.rows.len(),
            s.events
        ),
        format!("rows:   {}", s.rows.join(", ")),
        format!("server: {}", s.server),
    ];
    if let Some(sgw) = &s.sgw_sha256 {
        lines.push(format!("sgw:    {sgw}"));
    }
    if let Some(when) = &s.recorded {
        lines.push(format!("when:   {when}"));
    }
    lines.push(format!("runs:   {}", s.runs.join(", ")));
    lines.join("\n")
}

/// Fancy mode: the section as a bold heading, then a dim-keyed block.
pub fn summary_fancy(ui: &Ui, s: &Summary) -> String {
    let mut lines = vec![format!(
        "{}  {}",
        ui.paint(Tone::Bold, &format!("golden {}", s.section)),
        ui.paint(
            Tone::Dim,
            &format!("{} rows · {} events", s.rows.len(), s.events)
        )
    )];
    let mut line = |key: &str, value: &str| {
        lines.push(format!(
            "  {}  {value}",
            ui.paint(Tone::Dim, &format!("{key:<6}"))
        ));
    };
    line("rows", &s.rows.join(", "));
    line("server", &s.server);
    if let Some(sgw) = &s.sgw_sha256 {
        line("sgw", sgw);
    }
    if let Some(when) = &s.recorded {
        line("when", when);
    }
    line("runs", &s.runs.join(", "));
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::strip_ansi;

    const GOLDEN: &str = r#"{"schema":1,"section":"first-session","rows":["FS-01","FS-P1"],"recorded_utc":"2026-10-11T10:00:00Z",
 "spec_sha256":"ab","build":{"server":"1a2b3c4","sgw_sha256":"cd"},
 "evidence":[{"run_id":"r1","run_dir":"d1"},{"run_id":"r2","run_dir":"d2"}],
 "fingerprint":{"schema":1,"section":"first-session","rows":[
   {"row":"FS-01","result":"PASS","tap":"full","events":["c2s a","s2c b"],"variance":{"late_bind_settle_ms":1000}},
   {"row":"FS-P1","result":"PASS","tap":"full","events":["c2s c"],"variance":{"late_bind_settle_ms":1000}}]}}"#;

    const NO_EVENTS: &str = r#"{"section":"s","rows":["R"],"build":{"server":"abc1234"},"evidence":[],"fingerprint":{"rows":[{"row":"R","variance":{}}]}}"#;

    fn golden() -> Summary {
        summarize(&serde_json::from_str(GOLDEN).unwrap())
    }

    // PS L155
    #[test]
    fn show_names_the_section_and_rows() {
        let s = golden();
        assert_eq!(s.section, "first-session");
        assert_eq!(s.rows.join(","), "FS-01,FS-P1");
    }

    // PS L156
    #[test]
    fn show_totals_the_ordered_events_over_the_rows() {
        assert_eq!(golden().events, 3);
    }

    // PS L157
    #[test]
    fn show_names_the_build_and_the_evidence_run_ids() {
        let s = golden();
        assert_eq!(s.server, "1a2b3c4");
        assert_eq!(s.sgw_sha256.as_deref(), Some("cd"));
        assert_eq!(s.runs.join(","), "r1,r2");
    }

    // PS L160
    #[test]
    fn a_row_without_events_counts_0_and_a_missing_sgw_hash_is_left_out() {
        let s = summarize(&serde_json::from_str(NO_EVENTS).unwrap());
        assert_eq!(s.events, 0);
        assert_eq!(s.sgw_sha256, None);
        assert_eq!(s.recorded, None);
    }

    // PS L161
    #[test]
    fn the_show_summary_has_no_nulls() {
        let s = summarize(&serde_json::from_str(NO_EVENTS).unwrap());
        assert!(!serde_json::to_string(&s).unwrap().contains("null"));
        assert!(!show_json(&s).contains("null"));
        assert_eq!(
            show_json(&s),
            r#"{"ok":true,"exit":0,"section":"s","rows":["R"],"events":0,"server":"abc1234","runs":[]}"#
        );
    }

    // PS L163
    #[test]
    fn the_show_text() {
        assert_eq!(
            summary_plain(&golden()),
            "golden first-session: 2 rows, 3 events\nrows:   FS-01, FS-P1\nserver: 1a2b3c4\nsgw:    cd\nwhen:   2026-10-11T10:00:00Z\nruns:   r1, r2"
        );
    }

    // New: fancy mode.
    #[test]
    fn the_fancy_show_block_has_a_bold_heading() {
        let ui = Ui {
            color: true,
            fancy: true,
            width: None,
        };
        let text = summary_fancy(&ui, &golden());
        assert_eq!(
            strip_ansi(&text),
            "golden first-session  2 rows · 3 events\n  rows    FS-01, FS-P1\n  server  1a2b3c4\n  sgw     cd\n  when    2026-10-11T10:00:00Z\n  runs    r1, r2"
        );
        assert!(text.starts_with("\x1b[1mgolden first-session\x1b[0m"));
    }
}
