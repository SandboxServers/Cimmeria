use super::*;
use crate::uat::golden::normalise::resolve;

fn strings(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

/// A passing, fully tapped row with a little of every part.
fn row(id: &str) -> RowFingerprint {
    RowFingerprint {
        row: id.to_string(),
        result: "PASS".to_string(),
        tap: "full".to_string(),
        tap_faults: Vec::new(),
        clauses: strings(&["chat=pass", "wire=pass"]),
        transitions: strings(&["mission 1360 -> 0", "step 4037 -> 0"]),
        windows_opened: strings(&["DialogWin", "TutorialWin"]),
        windows_closed: strings(&["DialogWin"]),
        events: strings(&[
            "c2s interact target=self",
            "s2c onDialogDisplay@self dialog_id=3995",
        ]),
        unordered: strings(&["s2c onEffectResults@self", "s2c onEffectResults@tag:Guard"]),
        variance: resolve(None, None),
    }
}

/// `golden` against a copy changed by `edit`.
fn line_after(id: &str, edit: impl FnOnce(&mut RowFingerprint)) -> String {
    let golden = row(id);
    let mut run = golden.clone();
    edit(&mut run);
    diff_row(&golden, &run).expect("a divergence").line()
}

#[test]
fn every_part_has_its_line() {
    let golden = RunFingerprint {
        schema: 1,
        section: "first-session".to_string(),
        rows: vec![row("FS-P4"), row("FS-P5")],
    };
    let run = RunFingerprint {
        rows: vec![row("FS-P4"), row("FS-P6")],
        ..golden.clone()
    };
    let report = diff_run(&golden, &run);
    assert_eq!(
        report.first.unwrap().line(),
        "row FS-P5: in the golden, not in this run"
    );
    assert_eq!(report.rows[0], ("FS-P4".to_string(), None));
    assert_eq!(report.not_in_golden, ["FS-P6"]);

    let cases: [(&str, Box<dyn FnOnce(&mut RowFingerprint)>, &str); 12] = [
        (
            "FS-P3",
            Box::new(|r| r.variance.ignore.push("onPlayerCommunication".into())),
            "row FS-P3: fingerprinted under a different variance than the golden's; \
             diff it under the golden's variance, or re-bless the golden",
        ),
        (
            "FS-P2",
            Box::new(|r| r.tap = "late:play".into()),
            "row FS-P2: expected tap `full`, got `late:play`",
        ),
        (
            "FS-P3",
            Box::new(|r| r.tap_faults = strings(&["dropped 3"])),
            "row FS-P3: tap fault `dropped 3`; the diff is unreliable",
        ),
        (
            "FS-P4",
            Box::new(|r| r.result = "FAIL".into()),
            "row FS-P4: expected result `PASS`, got `FAIL`",
        ),
        (
            "FS-P4",
            Box::new(|r| r.clauses[1] = "wire=fail".into()),
            "row FS-P4: expected clause `wire=pass` after `chat=pass`, got `wire=fail`",
        ),
        (
            "FS-P4",
            Box::new(|r| r.transitions[0] = "mission 1360 -> 1".into()),
            "row FS-P4: expected transition `mission 1360 -> 0` first, got `mission 1360 -> 1`",
        ),
        (
            "FS-P4",
            Box::new(|r| r.windows_opened.truncate(1)),
            "row FS-P4: expected windows opened `DialogWin, TutorialWin`, got `DialogWin`",
        ),
        (
            "FS-P4",
            Box::new(|r| r.windows_closed.clear()),
            "row FS-P4: expected windows closed `DialogWin`, got nothing",
        ),
        (
            "FS-P3",
            Box::new(|r| r.events[1] = "s2c onDialogDisplay@self dialog_id=3996".into()),
            "row FS-P3: expected `s2c onDialogDisplay@self dialog_id=3995` after \
             `c2s interact target=self`, got `s2c onDialogDisplay@self dialog_id=3996`",
        ),
        (
            "FS-P3",
            Box::new(|r| {
                r.events.pop();
            }),
            "row FS-P3: expected `s2c onDialogDisplay@self dialog_id=3995` after \
             `c2s interact target=self`, got the end of the row",
        ),
        (
            "FS-P3",
            Box::new(|r| r.events.push("s2c onTimer@self".into())),
            "row FS-P3: expected the end of the row after \
             `s2c onDialogDisplay@self dialog_id=3995`, got `s2c onTimer@self`",
        ),
        (
            "FS-P3",
            Box::new(|r| {
                r.unordered.remove(0);
            }),
            "row FS-P3: expected unordered `s2c onEffectResults@self` first, \
             got `s2c onEffectResults@tag:Guard`",
        ),
    ];
    for (id, edit, want) in cases {
        assert_eq!(line_after(id, edit), want);
    }
}

/// The parts are compared in the packet's order: with everything changed
/// at once, each removal of the earliest difference exposes the next.
#[test]
fn parts_are_compared_in_order() {
    let golden = row("R");
    let mut run = golden.clone();
    run.variance.collapse.push("onTimer*".into());
    run.tap = "late:play".into();
    run.tap_faults = strings(&["dropped 1"]);
    run.result = "FAIL".into();
    run.clauses.clear();
    run.transitions.clear();
    run.windows_opened.clear();
    run.windows_closed.clear();
    run.events.clear();
    run.unordered.clear();
    let order = [
        Part::Variance,
        Part::Tap,
        Part::TapFault,
        Part::Result,
        Part::Clause,
        Part::Transition,
        Part::WindowsOpened,
        Part::WindowsClosed,
        Part::Event,
        Part::Unordered,
    ];
    for part in order {
        let d = diff_row(&golden, &run).expect("a divergence");
        assert_eq!(d.part, part);
        match part {
            Part::Variance => run.variance = golden.variance.clone(),
            Part::Tap => run.tap = golden.tap.clone(),
            Part::TapFault => run.tap_faults.clear(),
            Part::Result => run.result = golden.result.clone(),
            Part::Clause => run.clauses = golden.clauses.clone(),
            Part::Transition => run.transitions = golden.transitions.clone(),
            Part::WindowsOpened => run.windows_opened = golden.windows_opened.clone(),
            Part::WindowsClosed => run.windows_closed = golden.windows_closed.clone(),
            Part::Event => run.events = golden.events.clone(),
            Part::Unordered => run.unordered = golden.unordered.clone(),
            Part::Missing => unreachable!(),
        }
    }
    assert_eq!(diff_row(&golden, &run), None);
}

/// The first row with a divergence is `first`, even when a later row's
/// divergence comes earlier in its own events.
#[test]
fn the_first_divergent_row_is_first() {
    let golden = RunFingerprint {
        schema: 1,
        section: "s".to_string(),
        rows: vec![row("A"), row("B"), row("C")],
    };
    let mut run = golden.clone();
    run.rows[1].events[1] = "s2c changed".into();
    run.rows[2].result = "FAIL".into();
    let report = diff_run(&golden, &run);
    let first = report.first.unwrap();
    assert_eq!((first.row.as_str(), first.part), ("B", Part::Event));
    assert_eq!(report.rows[2].1.as_ref().unwrap().part, Part::Result);
}

#[test]
fn line_caps_hold_for_long_events() {
    let long = format!("s2c onPlayerCommunication@self Text={}", "x".repeat(400));
    let mut golden = row("FS-P3");
    golden.events = vec![long.clone(), format!("{long}a")];
    let mut run = golden.clone();
    run.events[1] = format!("s2c two\nlines {long}");
    let line = diff_row(&golden, &run).unwrap().line();
    assert_eq!(line.chars().count(), 300);
    assert!(!line.contains('\n'));
    assert!(line.ends_with("..."), "{line}");

    // Each quoted value is cut to 120 characters, backticks aside.
    let golden = row("FS-P3");
    let mut run = golden.clone();
    run.events[0] = long.clone();
    let d = diff_row(&golden, &run).unwrap();
    let line = d.line();
    let quoted = line.split('`').nth(1).unwrap();
    assert_eq!(quoted, golden.events[0]);
    let got = line.rsplit('`').nth(1).unwrap();
    assert_eq!(got.chars().count(), 120);
    assert!(got.ends_with("...") && long.starts_with(&got[..117]));
}
