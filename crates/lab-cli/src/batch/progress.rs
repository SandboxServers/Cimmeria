//! The progress line `lab uat` prints for one finished run.

use super::RunRecord;

fn non_empty(text: &Option<String>) -> Option<&str> {
    text.as_deref().filter(|t| !t.is_empty())
}

/// `[<instance> <index>/<total>] <what> (<seconds>s)`, then `; golden: <g>`
/// when the record has a `Golden` value. `<what>` is `PASS`; `ERROR <error>`
/// for a run that threw; else `FAIL <row>: <reason>` plus ` [known <issue>]`
/// when the verdict has an issue.
pub fn format_progress_line(record: &RunRecord, total: u32) -> String {
    let what = if record.ok {
        "PASS".to_string()
    } else if let Some(error) = non_empty(&record.error) {
        format!("ERROR {error}")
    } else {
        let verdict = record.verdict.as_ref();
        let row = verdict.and_then(|v| non_empty(&v.row)).unwrap_or("");
        let reason = verdict.and_then(|v| non_empty(&v.reason)).unwrap_or("");
        let tag = verdict
            .and_then(|v| non_empty(&v.issue))
            .map(|issue| format!(" [known {issue}]"))
            .unwrap_or_default();
        format!("FAIL {row}: {reason}{tag}")
    };
    let mut line = format!(
        "[{} {}/{}] {} ({}s)",
        record.instance, record.index, total, what, record.seconds
    );
    if let Some(golden) = non_empty(&record.golden) {
        line.push_str(&format!("; golden: {golden}"));
    }
    line
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batch::Verdict;

    fn record(ok: bool) -> RunRecord {
        RunRecord {
            instance: "p2".into(),
            index: 3,
            seconds: 41,
            ok,
            ..Default::default()
        }
    }

    #[test]
    fn a_passing_run_reads_pass() {
        assert_eq!(
            format_progress_line(&record(true), 5),
            "[p2 3/5] PASS (41s)"
        );
    }

    #[test]
    fn a_failing_run_names_the_row_the_reason_and_the_known_issue() {
        let mut r = record(false);
        r.verdict = Some(Verdict {
            ok: false,
            row: Some("FS-P2".into()),
            result: Some("FAIL".into()),
            reason: Some("clause movie-over failed".into()),
            issue: Some("#1341".into()),
        });
        assert_eq!(
            format_progress_line(&r, 5),
            "[p2 3/5] FAIL FS-P2: clause movie-over failed [known #1341] (41s)"
        );
    }

    #[test]
    fn a_run_that_threw_reads_error() {
        let mut r = record(false);
        r.error = Some("lab_uat_run failed: bridge down".into());
        assert_eq!(
            format_progress_line(&r, 5),
            "[p2 3/5] ERROR lab_uat_run failed: bridge down (41s)"
        );
    }

    #[test]
    fn a_golden_value_is_appended() {
        let mut r = record(true);
        r.golden = Some("match".into());
        assert_eq!(
            format_progress_line(&r, 5),
            "[p2 3/5] PASS (41s); golden: match"
        );
    }
}
