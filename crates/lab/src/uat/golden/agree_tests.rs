use std::path::Path;

use serde_json::{json, Value};

use super::*;
use crate::uat::evidence::{RowResult, RunDir};
use crate::uat::golden::build::{fingerprint_run, fingerprint_run_with};
use crate::uat::golden::diff::{diff_run, diff_run_dir, golden_variances, Part};
use crate::uat::golden::event::RowFingerprint;
use crate::uat::golden::testkit::{
    fs_p3_entities, fs_p3_tap, fs_p3_windows, row_evidence, tap_messages, write_run,
};

const SECTION: &str = "first-session";

/// FS-P1 and FS-P3. FS-P3's golden ignores `onPlayerCommunication`, which
/// the defaults do not, so a diff under the spec's variance after an edit
/// would see a difference the golden's variance hides.
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

const DIALOG_LINE: &str = "row FS-P3: expected `s2c onDialogDisplay@self dialog_id=3995 entity_id=tag:ArmYourself_FrostBody mission_id=0` after `c2s interact target=tag:ArmYourself_FrostBody`, got `s2c onDialogDisplay@self dialog_id=3996 entity_id=tag:ArmYourself_FrostBody mission_id=0`";

/// A run of both rows: FS-P1 is the capture's first five records (the
/// teleport), FS-P3 the whole fixture with `p3_tap` in place of its tap.
fn bundle(root: &Path, run_id: &str, p3_tap: Value) -> RunDir {
    let mut p1_tap = fs_p3_tap();
    tap_messages(&mut p1_tap).truncate(5);
    let rows = [
        (
            row_evidence("FS-P1", RowResult::Pass, &[]),
            Some(p1_tap),
            None,
            None,
        ),
        (
            row_evidence("FS-P3", RowResult::Pass, &[]),
            Some(p3_tap),
            Some(fs_p3_entities()),
            Some(fs_p3_windows()),
        ),
    ];
    write_run(root, run_id, SECTION, &rows)
}

fn fingerprint(run: &RunDir) -> RunFingerprint {
    let spec = crate::uat::spec::parse(SPEC).unwrap();
    fingerprint_run(run, &spec, None).unwrap().0
}

/// Five fingerprints of the unchanged fixture.
fn five_runs(root: &Path) -> Vec<RunFingerprint> {
    (1..=5)
        .map(|k| fingerprint(&bundle(root, &format!("run{k}"), fs_p3_tap())))
        .collect()
}

/// The FS-P3 tap with the first dialog the server sent changed to 3996.
fn changed_dialog() -> Value {
    let mut tap = fs_p3_tap();
    let m = tap_messages(&mut tap);
    assert_eq!(m[16]["msg_name"], "onDialogDisplay");
    m[16]["decoded"]["dialog_id"] = json!(3996);
    tap
}

/// The campaign's test-only acceptance: a changed server answer in an
/// otherwise golden run is exactly one line, naming the row, the golden's
/// event, the event before it and what the server sent instead.
#[test]
fn a_changed_dialog_answer_is_one_line() {
    let dir = tempfile::tempdir().unwrap();
    let golden = agree(&five_runs(dir.path()), true).unwrap();
    let ids: Vec<&str> = golden.rows.iter().map(|r| r.row.as_str()).collect();
    assert_eq!(ids, ["FS-P1", "FS-P3"]);

    let mut tap = changed_dialog();
    let m = tap_messages(&mut tap);
    // Hidden by the golden's variance; under the defaults this would be
    // the first divergence (index 1, before the dialog).
    m[1]["decoded"]["Text"] = json!("gmGotoXYZ: teleported somewhere else");
    // A later divergence in the same row, which must not be the one shown.
    assert_eq!(m[18]["msg_name"], "InteractionType");
    m[18]["decoded"]["TypeId"] = json!(7);
    let run = bundle(dir.path(), "run6", tap);

    let (report, meta) = diff_run_dir(&golden, &run).unwrap();
    let line = report.first.as_ref().unwrap().line();
    assert_eq!(line, DIALOG_LINE);
    assert!(!line.contains('\n'));
    assert_eq!(report.rows[0], ("FS-P1".to_string(), None));
    assert_eq!(meta.run_id, "run6");

    // Rows pair by id: the same fingerprint with its rows reversed still
    // compares FS-P3 with FS-P3.
    let (mut fp, _) = fingerprint_run_with(&run, SECTION, &golden_variances(&golden)).unwrap();
    fp.rows.reverse();
    assert_eq!(diff_run(&golden, &fp).first.unwrap().line(), DIALOG_LINE);

    // The spec drops the ignore after the golden was recorded. A run
    // fingerprinted under the edited spec is refused, not compared under a
    // model the golden never agreed to.
    let edited =
        crate::uat::spec::parse(&SPEC.replace(r#"ignore = ["onPlayerCommunication"]"#, ""))
            .unwrap();
    let (by_spec, _) = fingerprint_run(&run, &edited, None).unwrap();
    let d = diff_run(&golden, &by_spec).first.unwrap();
    assert_eq!((d.row.as_str(), d.part), ("FS-P3", Part::Variance));
}

#[test]
fn five_identical_runs_agree_and_four_do_not() {
    let dir = tempfile::tempdir().unwrap();
    let mut fps = five_runs(dir.path());
    assert_eq!(agree(&fps[..4], true), Err(AgreeError::TooFew { have: 4 }));
    // Rows pair by id, so runs 2-5 with their rows reversed still agree;
    // the order agree does not compare shows which fingerprint came back.
    for fp in &mut fps[1..] {
        fp.rows.reverse();
    }
    let golden = agree(&fps, true).unwrap();
    assert_eq!(golden, fps[0]);
    assert_ne!(golden, fps[1]);
}

#[test]
fn a_run_with_no_rows_cannot_be_golden() {
    let dir = tempfile::tempdir().unwrap();
    let mut fps = five_runs(dir.path());
    for fp in &mut fps {
        fp.rows.clear();
    }
    assert_eq!(agree(&fps, false), Err(AgreeError::NoRows));
    assert_eq!(
        AgreeError::NoRows.to_string(),
        "run 1 has no rows; a golden needs at least one"
    );
}

#[test]
fn a_different_section_or_schema_disagrees() {
    let dir = tempfile::tempdir().unwrap();
    let fps = five_runs(dir.path());

    let mut section = fps.clone();
    section[2].section = "other".to_string();
    let err = agree(&section, true).unwrap_err();
    assert_eq!(
        err,
        AgreeError::Mismatch {
            run: 3,
            field: "section",
            expected: SECTION.to_string(),
            got: "other".to_string()
        }
    );
    assert_eq!(
        err.to_string(),
        "run 3 has section `other`, run 1 has `first-session`; the runs are not comparable"
    );

    let mut schema = fps;
    schema[4].schema = 2;
    assert!(matches!(
        agree(&schema, true),
        Err(AgreeError::Mismatch {
            run: 5,
            field: "schema",
            ..
        })
    ));
}

#[test]
fn mixed_results_disagree_without_pass_rule() {
    let dir = tempfile::tempdir().unwrap();
    let mut fps = five_runs(dir.path());
    fps[4].rows[1].result = "FAIL".to_string();
    let Err(AgreeError::Disagree(d)) = agree(&fps, false) else {
        panic!("not a disagreement")
    };
    assert_eq!((d.run, d.divergence.part), (5, Part::Result));
}

/// Every part diff_row compares, changed in run 4 alone, blocks agreement
/// and is named as that part.
#[test]
fn each_part_difference_blocks_agreement() {
    let dir = tempfile::tempdir().unwrap();
    let fps = five_runs(dir.path());
    let cases: [(Part, fn(&mut RowFingerprint)); 7] = [
        (Part::Tap, |r| r.tap = "late:play".to_string()),
        (Part::Clause, |r| r.clauses.push("extra=pass".to_string())),
        (Part::Transition, |r| {
            r.transitions.push("mission 1 -> 0".to_string())
        }),
        (Part::WindowsOpened, |r| {
            r.windows_opened.push("ExtraWin".to_string())
        }),
        (Part::WindowsClosed, |r| {
            r.windows_closed.push("ExtraWin".to_string())
        }),
        (Part::Unordered, |r| {
            r.unordered.push("s2c extra".to_string())
        }),
        (Part::Event, |r| r.events.push("s2c extra".to_string())),
    ];
    for (part, edit) in cases {
        let mut runs = fps.clone();
        edit(&mut runs[3].rows[1]);
        let Err(AgreeError::Disagree(d)) = agree(&runs, true) else {
            panic!("{part:?}: not a disagreement")
        };
        assert_eq!((d.run, d.divergence.part), (4, part));
    }
}

#[test]
fn any_non_pass_result_blocks_golden() {
    let dir = tempfile::tempdir().unwrap();
    let mut fps = five_runs(dir.path());
    fps[1].rows[0].result = "BLOCKED".to_string();
    assert_eq!(
        agree(&fps, true),
        Err(AgreeError::NotPassing {
            run: 2,
            row: "FS-P1".to_string(),
            result: "BLOCKED".to_string()
        })
    );
}

#[test]
fn a_missing_row_in_a_later_run_disagrees() {
    let dir = tempfile::tempdir().unwrap();
    let mut fps = five_runs(dir.path());
    fps[3].rows.remove(0);
    assert_eq!(
        agree(&fps, true).unwrap_err().to_string(),
        "run 4 of 5 disagrees with run 1: row FS-P1: in the golden, not in this run"
    );
}

#[test]
fn a_disagreeing_fifth_run_names_itself() {
    let dir = tempfile::tempdir().unwrap();
    let mut fps = five_runs(dir.path());
    fps[4] = fingerprint(&bundle(dir.path(), "run5b", changed_dialog()));
    let err = agree(&fps, true).unwrap_err();
    assert_eq!(
        err.to_string(),
        format!("run 5 of 5 disagrees with run 1: {DIALOG_LINE}")
    );
    let AgreeError::Disagree(d) = err else {
        panic!("not a disagreement")
    };
    assert_eq!((d.run, d.of), (5, 5));
}

#[test]
fn a_failing_row_cannot_be_golden_when_pass_is_required() {
    let dir = tempfile::tempdir().unwrap();
    let mut fps = five_runs(dir.path());
    fps[2].rows[1].result = "FAIL".to_string();
    let err = agree(&fps, true).unwrap_err();
    assert_eq!(
        err,
        AgreeError::NotPassing {
            run: 3,
            row: "FS-P3".to_string(),
            result: "FAIL".to_string()
        }
    );
    assert_eq!(err.to_string(), "run 3 row FS-P3 is FAIL, not PASS");

    // Without the rule, five runs that fail alike agree (D-GD6 option b).
    for fp in &mut fps {
        fp.rows[1].result = "FAIL".to_string();
    }
    assert_eq!(agree(&fps, false), Ok(fps[0].clone()));
    assert!(matches!(
        agree(&fps, true),
        Err(AgreeError::NotPassing { run: 1, .. })
    ));
}

#[test]
fn a_faulty_or_unavailable_tap_cannot_be_golden() {
    let dir = tempfile::tempdir().unwrap();
    let fps = five_runs(dir.path());

    let mut dropped = fps.clone();
    dropped[3].rows[1].tap_faults = vec!["dropped 3".to_string(), "replaced".to_string()];
    let err = agree(&dropped, false).unwrap_err();
    assert_eq!(
        err.to_string(),
        "run 4 row FS-P3: tap dropped 3, replaced; a golden needs a complete tap"
    );

    // An unavailable tap is the same in every run, so only this check
    // catches it.
    let mut unavailable = fps.clone();
    for fp in &mut unavailable {
        fp.rows[0].tap = "unavailable".to_string();
    }
    assert_eq!(
        agree(&unavailable, true),
        Err(AgreeError::TapUnreliable {
            run: 1,
            row: "FS-P1".to_string(),
            reason: "unavailable".to_string()
        })
    );

    let mut extra = fps;
    let mut row = extra[1].rows[0].clone();
    row.row = "FS-P9".to_string();
    extra[1].rows.push(row);
    assert_eq!(
        agree(&extra, true).unwrap_err().to_string(),
        "run 2 has row FS-P9, which run 1 does not"
    );
}

#[test]
fn ignored_noise_does_not_break_agreement() {
    let dir = tempfile::tempdir().unwrap();
    let mut fps = five_runs(dir.path());
    let mut noisy = fs_p3_tap();
    let m = tap_messages(&mut noisy);
    let update = m[3].clone();
    assert_eq!(update["msg_name"], "avatarUpdateExplicit");
    for at in [20, 16, 5] {
        m.insert(at, update.clone());
    }
    fps[2] = fingerprint(&bundle(dir.path(), "noisy", noisy));
    assert_eq!(agree(&fps, true), Ok(fps[0].clone()));
}

fn meta(run_id: &str, dir: &str) -> RunMeta {
    RunMeta {
        run_id: run_id.to_string(),
        run_dir: dir.to_string(),
        spec_sha256: Some("spec".to_string()),
        server_build: Some("build".to_string()),
        sgw_sha256: Some("sgw".to_string()),
    }
}

#[test]
fn check_runs_rejects_mixed_builds_and_repeated_runs() {
    let good: Vec<RunMeta> = (1..=5)
        .map(|k| meta(&format!("r{k}"), &format!("d{k}")))
        .collect();
    assert_eq!(check_runs(&good), Ok(()));

    let mut repeated = good.clone();
    repeated[3] = meta("r2", "d4");
    assert_eq!(
        check_runs(&repeated),
        Err("run id `r2` appears twice: `d2` and `d4`".to_string())
    );

    let mut build = good.clone();
    build[2].server_build = Some("other".to_string());
    assert_eq!(
        check_runs(&build),
        Err("server_build differs: `build` in run r1 and `other` in run r3".to_string())
    );

    let mut spec = good.clone();
    spec[4].spec_sha256 = Some("edited".to_string());
    assert_eq!(
        check_runs(&spec),
        Err("spec_sha256 differs: `spec` in run r1 and `edited` in run r5".to_string())
    );

    let mut sgw = good.clone();
    sgw[1].sgw_sha256 = None;
    assert_eq!(
        check_runs(&sgw),
        Err("sgw_sha256 is missing in run r2".to_string())
    );

    let mut first_missing = good.clone();
    first_missing[0].spec_sha256 = None;
    assert_eq!(
        check_runs(&first_missing),
        Err("spec_sha256 is missing in run r1".to_string())
    );

    // An empty value is missing, even when every run recorded it empty.
    let mut empty_build = good.clone();
    for m in &mut empty_build {
        m.server_build = Some(String::new());
    }
    assert_eq!(
        check_runs(&empty_build),
        Err("server_build is missing in run r1".to_string())
    );
    let mut empty_hash = good;
    empty_hash[3].sgw_sha256 = Some(String::new());
    assert_eq!(
        check_runs(&empty_hash),
        Err("sgw_sha256 is missing in run r4".to_string())
    );
}
