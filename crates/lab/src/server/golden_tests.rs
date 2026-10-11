use serde_json::{json, Value};

use super::*;
use crate::uat::evidence::RowResult;
use crate::uat::golden::testkit::{
    fs_p3_entities, fs_p3_tap, fs_p3_windows, row_evidence, tap_messages, write_run,
};

#[path = "golden_rebless_tests.rs"]
mod rebless;

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
        Self::with_spec(SPEC)
    }

    fn with_spec(spec: &str) -> Self {
        let specs = tempfile::tempdir().unwrap();
        std::fs::write(specs.path().join("first-session.toml"), spec).unwrap();
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

    /// Five unchanged, passing runs numbered from `first`.
    fn five_from(&self, first: usize) -> Vec<String> {
        (first..first + 5)
            .map(|k| self.good(&format!("run{k}")))
            .collect()
    }

    /// A run of `rows`, each with FS-P1's tap and no entities or windows.
    fn plain_run(&self, id: &str, rows: &[(&str, RowResult)]) -> String {
        let rows: Vec<_> = rows
            .iter()
            .map(|(row, result)| (row_evidence(row, *result, &[]), Some(p1_tap()), None, None))
            .collect();
        let run = write_run(self.runs.path(), id, SECTION, &rows);
        let mut manifest: Value = run.read_json(&run.run_json()).unwrap();
        manifest["specs"][0]["sha256"] = json!(self.sha);
        run.write_json(&run.run_json(), &manifest).unwrap();
        run.root.display().to_string()
    }

    fn record(&self, run_dirs: Vec<String>, rebless: bool) -> Value {
        self.record_rows(None, run_dirs, rebless)
    }

    fn record_rows(&self, rows: Option<&[&str]>, run_dirs: Vec<String>, rebless: bool) -> Value {
        record(
            GoldenRecordArgs {
                section: SECTION.to_string(),
                rows: rows.map(|r| r.iter().map(|s| s.to_string()).collect()),
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

    let reply = lab.record(lab.five_from(6), true);
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

/// lab-chaos D-CH7: five runs that agree and pass are still refused when one
/// ran under chaos (CH-08 writes `chaos` into its `run.json`).
#[test]
fn record_refuses_a_chaos_run() {
    let lab = Lab::new();
    let runs = lab.five(|lab, k| {
        (k == 4).then(|| {
            lab.run("run4", RowResult::Pass, p1_tap(), fs_p3_tap(), |m| {
                m["chaos"] = json!("chaos=flush:pack seed=7");
            })
        })
    });
    let reply = lab.record(runs, false);
    assert_eq!(code(&reply), "cannot_record", "{reply}");
    assert!(
        error(&reply).contains("chaos runs never become golden (chaos=flush:pack seed=7)"),
        "{reply}"
    );
    assert!(error(&reply).contains("run4"), "{reply}");
    assert!(!lab.golden().exists());
}

/// The refusal does not depend on where the chaos run sits: the first run
/// is checked before anything is fingerprinted.
#[test]
fn record_refuses_a_chaos_run_in_first_place() {
    let lab = Lab::new();
    let runs = lab.five(|lab, k| {
        (k == 1).then(|| {
            lab.run("run1", RowResult::Pass, p1_tap(), fs_p3_tap(), |m| {
                m["chaos"] = json!("chaos=cpu:safe seed=12345678901234567890");
            })
        })
    });
    let reply = lab.record(runs, false);
    assert_eq!(code(&reply), "cannot_record", "{reply}");
    assert!(error(&reply).contains("run1"), "{reply}");
    assert!(
        error(&reply).contains("chaos runs never become golden"),
        "{reply}"
    );
    assert!(!lab.golden().exists());
}

/// A non-chaos runner may write `"chaos": null`; that is not a chaos run.
#[test]
fn record_accepts_a_null_chaos_key() {
    let lab = Lab::new();
    let runs = lab.five(|lab, k| {
        (k == 3).then(|| {
            lab.run("run3", RowResult::Pass, p1_tap(), fs_p3_tap(), |m| {
                m["chaos"] = Value::Null;
            })
        })
    });
    let reply = lab.record(runs, false);
    assert_eq!(reply["ok"], true, "{reply}");
    assert!(lab.golden().exists());
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

/// Seven rows all diverge: `first` plus four in `rows`, `diverged` the total.
#[test]
fn diff_caps_the_listed_rows() {
    let ids = ["R1", "R2", "R3", "R4", "R5", "R6", "R7"];
    let mut spec = SPEC[..SPEC.find("[[row]]").unwrap()].to_string();
    for id in ids {
        spec.push_str(&format!(
            "[[row]]\nid = \"{id}\"\ntitle = \"t\"\nexpected = \"e\"\n\
             step = [{{ chat = \".help\", label = \"help\" }}]\n\
             [[row.expect]]\nid = \"help\"\ntext = \"t\"\nsource = \"chat\"\n\
             since = \"help\"\ncontains = \"help\"\n\n"
        ));
    }
    let lab = Lab::with_spec(&spec);
    let pass: Vec<_> = ids.iter().map(|r| (*r, RowResult::Pass)).collect();
    let runs = (1..=5)
        .map(|k| lab.plain_run(&format!("run{k}"), &pass))
        .collect();
    assert_eq!(lab.record(runs, false)["ok"], true);

    let fail: Vec<_> = ids.iter().map(|r| (*r, RowResult::Fail)).collect();
    let reply = lab.diff(lab.plain_run("run6", &fail));
    assert_eq!(reply["first"], "row R1: expected result `PASS`, got `FAIL`");
    assert_eq!(reply["diverged"], 7);
    let listed: Vec<&str> = reply["rows"]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(listed, ["R2", "R3", "R4", "R5"]);
}

#[test]
fn diff_of_an_unreadable_run_is_cannot_diff() {
    let lab = Lab::new();
    assert_eq!(lab.record(lab.five(|_, _| None), false)["ok"], true);
    let missing = lab.runs.path().join("no-such-run");
    let reply = lab.diff(missing.display().to_string());
    assert_eq!(code(&reply), "cannot_diff", "{reply}");
    assert!(!error(&reply).contains('\n'));
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
