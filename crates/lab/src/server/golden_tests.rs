use serde_json::{json, Value};

use super::*;
use crate::uat::evidence::RowResult;
use crate::uat::golden::testkit::{
    fs_p3_entities, fs_p3_tap, fs_p3_windows, row_evidence, tap_messages, write_run,
};

const SECTION: &str = "first-session";
const NOW: &str = "2026-10-11T00:00:00Z";

/// Two rows; FS-P3's golden ignores `onPlayerCommunication` (as in GD-04's
/// acceptance), so the diff must run under the stored variance to match.
const SPEC: &str = r#"
schema = 1
[section]
id = "first-session"
system = "golden"
guide = "unified-uat.md#golden"
ledger = "lab-golden/README.md"

[[row]]
id = "FS-P1"
title = "teleport"
expected = "e"
step = [{ chat = ".help", label = "help" }]
[[row.expect]]
id = "help"
text = "t"
source = "chat"
since = "help"
contains = "help"

[[row]]
id = "FS-P3"
title = "frost"
expected = "e"
step = [{ chat = ".help", label = "help" }]
[[row.expect]]
id = "help"
text = "t"
source = "chat"
since = "help"
contains = "help"
[row.golden]
ignore = ["onPlayerCommunication"]
"#;

/// GD-04's acceptance line for the FS-P3 dialog changed from 3995 to 3996.
const DIALOG_LINE: &str = "row FS-P3: expected `s2c onDialogDisplay@self dialog_id=3995 entity_id=tag:ArmYourself_FrostBody mission_id=0` after `c2s interact target=tag:ArmYourself_FrostBody`, got `s2c onDialogDisplay@self dialog_id=3996 entity_id=tag:ArmYourself_FrostBody mission_id=0`";

/// A specs dir holding the two-row spec, and that spec file's hash.
struct Lab {
    specs: tempfile::TempDir,
    runs: tempfile::TempDir,
    sha: String,
}

impl Lab {
    fn new() -> Self {
        let specs = tempfile::tempdir().unwrap();
        std::fs::write(specs.path().join("first-session.toml"), SPEC).unwrap();
        let sha = load_sections(specs.path(), None).unwrap()[0].sha256.clone();
        Self {
            specs,
            runs: tempfile::tempdir().unwrap(),
            sha,
        }
    }

    /// A run of both rows, its `run.json` naming this spec's real hash.
    /// `edit` changes the manifest after that.
    fn run(
        &self,
        id: &str,
        p1: RowResult,
        p1_tap: Value,
        p3_tap: Value,
        edit: impl FnOnce(&mut Value),
    ) -> String {
        let rows = [
            (row_evidence("FS-P1", p1, &[]), Some(p1_tap), None, None),
            (
                row_evidence("FS-P3", RowResult::Pass, &[]),
                Some(p3_tap),
                Some(fs_p3_entities()),
                Some(fs_p3_windows()),
            ),
        ];
        let run = write_run(self.runs.path(), id, SECTION, &rows);
        let mut manifest: Value = run.read_json(&run.run_json()).unwrap();
        manifest["specs"][0]["sha256"] = json!(self.sha);
        edit(&mut manifest);
        run.write_json(&run.run_json(), &manifest).unwrap();
        run.root.display().to_string()
    }

    /// An unchanged, passing run.
    fn good(&self, id: &str) -> String {
        self.run(id, RowResult::Pass, p1_tap(), fs_p3_tap(), |_| {})
    }

    /// Runs 1 to 5, with run `k` replaced by `odd(k)` when it returns one.
    fn five(&self, odd: impl Fn(&Self, usize) -> Option<String>) -> Vec<String> {
        (1..=5)
            .map(|k| odd(self, k).unwrap_or_else(|| self.good(&format!("run{k}"))))
            .collect()
    }

    fn record(&self, run_dirs: Vec<String>, rebless: bool) -> Value {
        record(
            GoldenRecordArgs {
                section: SECTION.to_string(),
                rows: None,
                run_dirs,
                specs_dir: Some(self.specs.path().display().to_string()),
                rebless,
            },
            NOW,
        )
    }

    fn diff(&self, run_dir: String) -> Value {
        diff(GoldenDiffArgs {
            run_dir,
            section: SECTION.to_string(),
            specs_dir: Some(self.specs.path().display().to_string()),
        })
    }

    fn golden(&self) -> std::path::PathBuf {
        golden_path(self.specs.path(), SECTION)
    }
}

/// FS-P1's tap: the capture's first five records (the teleport).
fn p1_tap() -> Value {
    let mut tap = fs_p3_tap();
    tap_messages(&mut tap).truncate(5);
    tap
}

fn code(v: &Value) -> &str {
    v["code"].as_str().unwrap_or("")
}

fn error(v: &Value) -> &str {
    v["error"].as_str().unwrap_or("")
}

#[test]
fn record_writes_a_golden_from_five_agreeing_runs() {
    let lab = Lab::new();
    let reply = lab.record(lab.five(|_, _| None), false);
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(reply["rows"], 2);
    assert_eq!(reply["runs"], 5);
    assert!(reply["events"].as_u64().unwrap() > 0);
    assert_eq!(reply["path"], lab.golden().display().to_string());

    let g = file::load(&lab.golden()).unwrap();
    assert_eq!(g.rows, ["FS-P1", "FS-P3"]);
    assert_eq!(g.spec_sha256, lab.sha);
    assert_eq!(g.build.server, "testbuild");
    assert_eq!(g.build.sgw_sha256.as_deref(), Some("sgw-sha"));
    assert_eq!(g.recorded_utc, NOW);
    let ids: Vec<&str> = g.evidence.iter().map(|e| e.run_id.as_str()).collect();
    assert_eq!(ids, ["run1", "run2", "run3", "run4", "run5"]);
    assert_eq!(reply["bytes"], std::fs::read(lab.golden()).unwrap().len());
    // Nothing else is written under the specs dir.
    let golden_dir: Vec<_> = std::fs::read_dir(lab.specs.path().join("golden"))
        .unwrap()
        .collect();
    assert_eq!(golden_dir.len(), 1);
}

#[test]
fn record_refuses_four_runs() {
    let lab = Lab::new();
    let mut runs = lab.five(|_, _| None);
    runs.pop();
    let reply = lab.record(runs, false);
    assert_eq!(code(&reply), "cannot_record", "{reply}");
    assert!(error(&reply).contains("needs 5"), "{reply}");
    assert!(!lab.golden().exists());
}

#[test]
fn record_refuses_an_existing_golden_without_rebless() {
    let lab = Lab::new();
    let runs = lab.five(|_, _| None);
    assert_eq!(lab.record(runs.clone(), false)["ok"], true);
    let before = std::fs::read(lab.golden()).unwrap();

    let reply = lab.record(runs.clone(), false);
    assert_eq!(code(&reply), "exists", "{reply}");
    assert!(error(&reply).contains("rebless"), "{reply}");
    assert_eq!(std::fs::read(lab.golden()).unwrap(), before);

    let reply = lab.record(runs, true);
    assert_eq!(reply["ok"], true, "{reply}");
}

/// Run 3 got a different dialog: the refusal is the GD-04 line, run-tagged.
#[test]
fn record_refuses_disagreeing_runs_with_the_line() {
    let lab = Lab::new();
    let runs = lab.five(|lab, k| {
        (k == 3).then(|| {
            let mut tap = fs_p3_tap();
            tap_messages(&mut tap)[16]["decoded"]["dialog_id"] = json!(3996);
            lab.run("run3", RowResult::Pass, p1_tap(), tap, |_| {})
        })
    });
    let reply = lab.record(runs, false);
    assert_eq!(code(&reply), "disagree", "{reply}");
    assert_eq!(
        error(&reply),
        format!("run 3 of 5 disagrees with run 1: {DIALOG_LINE}")
    );
    assert!(!lab.golden().exists());
}

#[test]
fn record_refuses_an_unavailable_tap() {
    let lab = Lab::new();
    let runs = lab.five(|lab, k| {
        (k == 2).then(|| {
            let mut tap = p1_tap();
            tap["coverage"] = json!("unavailable");
            lab.run("run2", RowResult::Pass, tap, fs_p3_tap(), |_| {})
        })
    });
    let reply = lab.record(runs, false);
    assert_eq!(code(&reply), "cannot_record", "{reply}");
    assert!(
        error(&reply).contains("run 2 row FS-P1: tap unavailable"),
        "{reply}"
    );
    assert!(!lab.golden().exists());
}

/// D-GD6 (a): agreeing runs that all failed one row are not a golden.
#[test]
fn record_refuses_a_row_that_is_not_pass() {
    let lab = Lab::new();
    let runs: Vec<String> = (1..=5)
        .map(|k| {
            lab.run(
                &format!("run{k}"),
                RowResult::Fail,
                p1_tap(),
                fs_p3_tap(),
                |_| {},
            )
        })
        .collect();
    let reply = lab.record(runs, false);
    assert_eq!(code(&reply), "not_passing", "{reply}");
    assert!(error(&reply).contains("row FS-P1 is FAIL"), "{reply}");
    assert!(!lab.golden().exists());
}

/// Identical fingerprints from two server builds are still refused.
#[test]
fn record_refuses_mixed_server_builds() {
    let lab = Lab::new();
    let runs = lab.five(|lab, k| {
        (k == 4).then(|| {
            lab.run("run4", RowResult::Pass, p1_tap(), fs_p3_tap(), |m| {
                m["server"]["service_version"] = json!("otherbuild");
            })
        })
    });
    let reply = lab.record(runs, false);
    assert_eq!(code(&reply), "cannot_record", "{reply}");
    assert!(error(&reply).contains("server_build differs"), "{reply}");
    assert!(!lab.golden().exists());
}

/// Runs that drove an older copy of the spec are refused: the golden would
/// store today's variance under yesterday's hash.
#[test]
fn record_refuses_runs_of_another_spec_version() {
    let lab = Lab::new();
    let runs = lab.five(|lab, k| {
        Some(lab.run(
            &format!("run{k}"),
            RowResult::Pass,
            p1_tap(),
            fs_p3_tap(),
            |m| m["specs"][0]["sha256"] = json!("old-sha"),
        ))
    });
    let reply = lab.record(runs, false);
    assert_eq!(code(&reply), "cannot_record", "{reply}");
    assert!(error(&reply).contains("old-sha"), "{reply}");
    assert!(!lab.golden().exists());
}

/// GD-04's acceptance through the tool: one changed server answer is one
/// line, and the whole reply stays short.
#[test]
fn diff_reports_the_changed_answer_in_one_line() {
    let lab = Lab::new();
    assert_eq!(lab.record(lab.five(|_, _| None), false)["ok"], true);

    let same = lab.diff(lab.good("run6"));
    assert_eq!(same, json!({ "ok": true, "match": true }));

    let mut tap = fs_p3_tap();
    let m = tap_messages(&mut tap);
    m[16]["decoded"]["dialog_id"] = json!(3996);
    // Hidden by the golden's stored variance, not by the defaults.
    m[1]["decoded"]["Text"] = json!("gmGotoXYZ: teleported somewhere else");
    let run = lab.run("run7", RowResult::Pass, p1_tap(), tap, |_| {});
    let reply = lab.diff(run);
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(reply["match"], false);
    assert_eq!(reply["first"], DIALOG_LINE);
    // One diverged row: no `rows`, `diverged` or `not_in_golden`, no nulls.
    let mut keys: Vec<&str> = reply
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["first", "match", "ok"]);
    let text = compact_text(&reply);
    assert!(text.len() < 500, "{} chars: {text}", text.len());
}

/// Two diverged rows: `first` is the first, `rows` lists only the other.
#[test]
fn diff_lists_the_other_diverged_rows() {
    let lab = Lab::new();
    assert_eq!(lab.record(lab.five(|_, _| None), false)["ok"], true);
    let mut tap = fs_p3_tap();
    tap_messages(&mut tap)[16]["decoded"]["dialog_id"] = json!(3996);
    let run = lab.run("run6", RowResult::Fail, p1_tap(), tap, |_| {});
    let reply = lab.diff(run);
    assert_eq!(
        reply["first"],
        "row FS-P1: expected result `PASS`, got `FAIL`"
    );
    assert_eq!(reply["diverged"], 2);
    assert_eq!(reply["rows"], json!({ "FS-P3": DIALOG_LINE }));
}

#[test]
fn diff_without_a_golden_says_so() {
    let lab = Lab::new();
    let reply = lab.diff(lab.good("run1"));
    assert_eq!(code(&reply), "no_golden", "{reply}");
    assert!(!error(&reply).contains('\n'));
}

#[test]
fn errors_are_one_capped_line() {
    let long = format!("a\nb\r\n{}", "x".repeat(1000));
    let line = one_line(&long);
    assert!(!line.contains(['\n', '\r']));
    assert_eq!(line.chars().count(), MAX_ERROR_CHARS);
    assert!(line.ends_with("..."));
}

/// The text a tool call returns.
fn compact_text(v: &Value) -> String {
    let r = compact(v);
    let v = serde_json::to_value(&r).unwrap();
    v["content"][0]["text"].as_str().unwrap().to_string()
}
