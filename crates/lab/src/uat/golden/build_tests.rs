use serde_json::{json, Value};

use super::*;
use crate::uat::evidence::{RowResult, Verdict};
use crate::uat::golden::testkit::{row_evidence, write_run};

fn fixture(text: &str) -> Value {
    serde_json::from_str(text).unwrap()
}

/// The FS-P3 window of the Praxis capture with its synthetic entity file
/// and window samples.
fn fs_p3() -> RowInput {
    RowInput {
        evidence: row_evidence("FS-P3", RowResult::Pass, &[]),
        tap: Some(fixture(include_str!("fixtures/fs_p3_tap.json"))),
        entities: Some(fixture(include_str!("fixtures/fs_p3_entities.json"))),
        windows: Some(fixture(include_str!("fixtures/fs_p3_windows.json"))),
    }
}

#[test]
fn fs_p3_normalises_to_the_expected_events() {
    let fp = build_row(&fs_p3(), &resolve(None, None));
    let expected = [
        "c2s gmGotoXYZ pos=-325.0,73.6,-212.8",
        "s2c onPlayerCommunication@self Channel=9 Text=gmGotoXYZ: teleported to (-325, 73.6, -212.8)",
        "s2c unknown#27@?1",
        "c2s triggerClientHintedGenericRegion region=14 entering=0",
        "c2s interact target=tag:ArmYourself_FrostBody",
        "s2c onDialogDisplay@self dialog_id=3995 entity_id=tag:ArmYourself_FrostBody mission_id=0",
        "s2c InteractionType@tag:ArmYourself_FrostBody TypeId=0",
        "s2c InteractionType@tag:ArmYourself_GuardBody TypeId=1073741824",
        "s2c onObjectiveUpdate@self ObjectiveID=3238 Status=1 Hidden=1 Optional=1",
        "s2c onObjectiveUpdate@self ObjectiveID=2452 Status=1 Hidden=0 Optional=0",
        "s2c onStepUpdate@self StepID=2113 Status=1",
        "s2c onStepUpdate@self StepID=80623 Status=0",
        "s2c onObjectiveUpdate@self ObjectiveID=90623 Status=0 Hidden=0 Optional=0",
        "s2c onMissionUpdate@self MissionID=1360 Status=0",
        "s2c onStepUpdate@self StepID=4037 Status=0",
        "s2c onObjectiveUpdate@self ObjectiveID=4650 Status=0 Hidden=0 Optional=0",
        "c2s dialogButtonChoice dialog=3995 button=-1",
    ];
    assert_eq!(fp.events, expected);
    assert!(fp.unordered.is_empty());
    assert_eq!(fp.row, "FS-P3");
    assert_eq!(fp.result, "PASS");
    assert_eq!(fp.tap, "full");
    assert!(fp.tap_faults.is_empty());
    assert_eq!(fp.variance, resolve(None, None));
}

#[test]
fn fs_p3_transitions_and_windows() {
    let fp = build_row(&fs_p3(), &resolve(None, None));
    assert_eq!(
        fp.transitions,
        [
            "objective 3238 -> 1",
            "objective 2452 -> 1",
            "step 2113 -> 1",
            "step 80623 -> 0",
            "objective 90623 -> 0",
            "mission 1360 -> 0",
            "step 4037 -> 0",
            "objective 4650 -> 0",
        ]
    );
    assert_eq!(fp.windows_opened, ["DialogWin"]);
    assert_eq!(fp.windows_closed, ["DialogWin"]);
}

/// The row's `vars` reach the normaliser: the same text with two run ids
/// fingerprints the same.
#[test]
fn row_vars_are_substituted() {
    let mut a = fs_p3();
    let mut b = fs_p3();
    for (input, id) in [(&mut a, "abcdef"), (&mut b, "ghijkl")] {
        input.evidence.vars = json!({ "run_id": id, "character": format!("Px{id}") });
        let messages = input.tap.as_mut().unwrap()["tap"]["messages"]
            .as_array_mut()
            .unwrap();
        messages[1]["decoded"]["Text"] = json!(format!("Welcome Px{id} (run {id})"));
    }
    let (fa, fb) = (
        build_row(&a, &resolve(None, None)),
        build_row(&b, &resolve(None, None)),
    );
    assert_eq!(fa.events, fb.events);
    assert_eq!(
        fa.events[1],
        "s2c onPlayerCommunication@self Channel=9 Text=Welcome ${character} (run ${run_id})"
    );
}

#[test]
fn faults_and_clauses_are_recorded() {
    let clauses = [
        ("chat", Source::Chat, true, Verdict::Pass),
        ("slow", Source::Timing, true, Verdict::Fail),
        ("maybe", Source::Tool, false, Verdict::Fail),
        ("logs", Source::Signoz, true, Verdict::Pending),
        ("ask", Source::Human, true, Verdict::NeedsHuman),
        ("wire", Source::Packet, true, Verdict::Unverified),
    ];
    let input = RowInput {
        evidence: row_evidence("R1", RowResult::Fail, &clauses),
        tap: Some(json!({
            "entity_id": 8, "read_ok": false, "stopped": false,
            "coverage": "late:frost", "replaced_existing": true,
            "tap": { "dropped": 3, "messages": [] },
        })),
        entities: None,
        windows: None,
    };
    let fp = build_row(&input, &resolve(None, None));
    assert_eq!(fp.result, "FAIL");
    assert_eq!(fp.tap, "late:frost");
    assert_eq!(
        fp.tap_faults,
        ["dropped 3", "replaced", "not stopped", "read failed"]
    );
    assert_eq!(fp.clauses, ["chat=pass", "wire=unverified"]);

    let clean = RowInput {
        tap: Some(json!({ "entity_id": 8, "read_ok": true, "stopped": true,
                          "coverage": "full", "replaced_existing": false,
                          "tap": { "dropped": 0, "messages": [] } })),
        ..input.clone()
    };
    assert!(build_row(&clean, &resolve(None, None))
        .tap_faults
        .is_empty());

    let untapped = RowInput { tap: None, ..input };
    let fp = build_row(&untapped, &resolve(None, None));
    assert_eq!(fp.tap, "none");
    assert!(fp.tap_faults.is_empty() && fp.events.is_empty());
}

const SPEC: &str = r#"
schema = 1
[section]
id = "golden-test"
system = "golden"
guide = "unified-uat.md#golden"
ledger = "lab-golden/README.md"

[section.golden]
allow_unordered = ["onEffectResults"]

[[row]]
id = "R1"
title = "one"
expected = "e"
step = [{ chat = ".help", label = "help" }]
[[row.expect]]
id = "help"
text = "t"
source = "chat"
since = "help"
contains = "help"

[[row]]
id = "R2"
title = "cleanup"
expected = "e"
step = [{ chat = ".help", label = "help" }]
[[row.expect]]
id = "help"
text = "t"
source = "chat"
since = "help"
contains = "help"
[row.golden]
skip = true

[[row]]
id = "R3"
title = "never reached"
expected = "e"
step = [{ chat = ".help", label = "help" }]
[[row.expect]]
id = "help"
text = "t"
source = "chat"
since = "help"
contains = "help"
[row.golden]
collapse = ["onTimer*"]
"#;

#[test]
fn a_run_fingerprints_from_its_bundle() {
    let spec = crate::uat::spec::parse(SPEC).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let p3 = fs_p3();
    let rows = [
        (
            row_evidence("R1", RowResult::Pass, &[]),
            p3.tap.clone(),
            p3.entities.clone(),
            p3.windows.clone(),
        ),
        (row_evidence("R2", RowResult::Pass, &[]), None, None, None),
    ];
    let run = write_run(dir.path(), "runabc", "golden-test", &rows);

    let (fp, meta) = fingerprint_run(&run, &spec, None).unwrap();
    assert_eq!(fp.schema, FINGERPRINT_SCHEMA);
    assert_eq!(fp.section, "golden-test");
    let ids: Vec<&str> = fp.rows.iter().map(|r| r.row.as_str()).collect();
    assert_eq!(ids, ["R1"], "R2 is skipped, R3 never ran");
    assert_eq!(fp.rows[0].events.len(), 17);
    assert_eq!(fp.rows[0].windows_opened, ["DialogWin"]);
    assert_eq!(fp.rows[0].variance.allow_unordered, ["onEffectResults"]);
    assert_eq!(
        meta,
        RunMeta {
            run_id: "runabc".into(),
            run_dir: run.root.file_name().unwrap().to_string_lossy().into_owned(),
            spec_sha256: Some("golden-test-sha".into()),
            server_build: Some("testbuild".into()),
            sgw_sha256: Some("sgw-sha".into()),
        }
    );

    let listed = |ids: &[&str]| ids.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let err = fingerprint_run(&run, &spec, Some(&listed(&["R1", "R3"]))).unwrap_err();
    assert!(err.contains("R3"), "{err}");
    let err = fingerprint_run(&run, &spec, Some(&listed(&["R9"]))).unwrap_err();
    assert!(err.contains("R9"), "{err}");
    let (fp, _) = fingerprint_run(&run, &spec, Some(&listed(&["R2", "R1"]))).unwrap();
    assert_eq!(fp.rows.len(), 1, "a listed skip row is still skipped");

    // The golden's stored variance, not the spec's.
    let mut v = resolve(None, None);
    v.ignore.push("onPlayerCommunication".into());
    let (fp, meta_with) =
        fingerprint_run_with(&run, "golden-test", &[("R1".into(), v.clone())]).unwrap();
    assert_eq!(fp.rows[0].events.len(), 16);
    assert_eq!(fp.rows[0].variance, v);
    assert_eq!(meta_with, meta);
    let err = fingerprint_run_with(&run, "golden-test", &[("R3".into(), v)]).unwrap_err();
    assert!(err.contains("R3"), "{err}");
}

#[test]
fn a_row_without_attachments_loads_with_none() {
    let dir = tempfile::tempdir().unwrap();
    let rows = [(
        row_evidence("R2", RowResult::Skipped, &[]),
        None,
        None,
        None,
    )];
    let run = write_run(dir.path(), "runabc", "golden-test", &rows);
    let input = load_row_input(&run, "golden-test", "R2").unwrap();
    assert!(input.tap.is_none() && input.entities.is_none() && input.windows.is_none());
    assert_eq!(input.evidence.row_id, "R2");
}
