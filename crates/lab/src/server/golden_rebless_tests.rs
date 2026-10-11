//! Record's run-set rules: rebless needs fresh runs and keeps every row,
//! `rows: []` means every row, and one run passed twice is refused.

use super::*;

fn evidence_ids(lab: &Lab) -> Vec<String> {
    file::load(&lab.golden())
        .unwrap()
        .evidence
        .into_iter()
        .map(|e| e.run_id)
        .collect()
}

#[test]
fn rebless_with_fresh_runs_names_the_new_evidence() {
    let lab = Lab::new();
    assert_eq!(lab.record(lab.five_from(1), false)["ok"], true);

    let reply = lab.record(lab.five_from(6), true);
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(
        evidence_ids(&lab),
        ["run6", "run7", "run8", "run9", "run10"]
    );
}

/// The owner's rule is five fresh agreeing runs: the old golden's own runs
/// are not evidence for a new one, even one of them.
#[test]
fn rebless_refuses_the_old_goldens_runs() {
    let lab = Lab::new();
    let old = lab.five_from(1);
    assert_eq!(lab.record(old.clone(), false)["ok"], true);
    let before = std::fs::read(lab.golden()).unwrap();

    let reply = lab.record(old.clone(), true);
    assert_eq!(code(&reply), "cannot_record", "{reply}");
    assert!(error(&reply).contains("fresh runs"), "{reply}");

    let mut mixed = lab.five_from(6);
    mixed[2] = old[0].clone();
    let reply = lab.record(mixed, true);
    assert_eq!(code(&reply), "cannot_record", "{reply}");
    assert!(error(&reply).ends_with("already holds run1"), "{reply}");
    assert_eq!(std::fs::read(lab.golden()).unwrap(), before);
}

#[test]
fn rebless_refuses_to_drop_rows() {
    let lab = Lab::new();
    assert_eq!(lab.record(lab.five_from(1), false)["ok"], true);
    let before = std::fs::read(lab.golden()).unwrap();

    let reply = lab.record_rows(Some(&["FS-P1"]), lab.five_from(6), true);
    assert_eq!(code(&reply), "cannot_record", "{reply}");
    assert!(error(&reply).contains("drop rows FS-P3"), "{reply}");
    assert_eq!(std::fs::read(lab.golden()).unwrap(), before);
}

/// A subset golden can grow on rebless: only dropping is refused.
#[test]
fn rebless_may_add_rows() {
    let lab = Lab::new();
    let reply = lab.record_rows(Some(&["FS-P1"]), lab.five_from(1), false);
    assert_eq!(reply["rows"], 1, "{reply}");

    let reply = lab.record(lab.five_from(6), true);
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(file::load(&lab.golden()).unwrap().rows, ["FS-P1", "FS-P3"]);
}

/// `rows: []` is the same as omitting `rows` (a CLI with no row filter may
/// send an empty list), not a golden of no rows.
#[test]
fn record_with_empty_rows_records_every_row() {
    let lab = Lab::new();
    let reply = lab.record_rows(Some(&[]), lab.five_from(1), false);
    assert_eq!(reply["ok"], true, "{reply}");
    assert_eq!(reply["rows"], 2);
    assert_eq!(file::load(&lab.golden()).unwrap().rows, ["FS-P1", "FS-P3"]);
}

/// One run directory passed twice is one run, not two agreeing ones.
#[test]
fn record_refuses_the_same_run_dir_twice() {
    let lab = Lab::new();
    let mut runs = lab.five_from(1);
    runs[4] = runs[0].clone();
    let reply = lab.record(runs, false);
    assert_eq!(code(&reply), "cannot_record", "{reply}");
    assert!(error(&reply).contains("`run1` appears twice"), "{reply}");
    assert!(!lab.golden().exists());
}
