//! LX-17a: the `--json` summary object, byte for byte, and its size budgets.
//! Ports the compact-JSON checks of tools/lab/cli/test-uat.ps1 and the JSON
//! checks of test-uat-chaos.ps1.

mod uat_support;

use cimmeria_lab_cli::commands::uat::golden::{GoldenGroup, GoldenSummary};
use cimmeria_lab_cli::commands::uat::json::{compact_json, compact_json_with, MAX_HEAL};
use cimmeria_lab_cli::commands::uat::summary::{
    FailureGroup, HealEntry, RowStat, RowStats, Summary,
};
use serde_json::Value;
use uat_support::batch_fx::*;

const BATCH_SHORT: &str = r"C:\b";
const BATCH_CHAOS: &str =
    r"C:\Users\someone\AppData\Local\cimmeria-lab\uat-runs\batch-20261011-120000";

const CLEAN: &str = r#"{"ok":true,"runs":1,"passed":1,"failed":0,"rows":{"FS-P1":"1/1","FS-P2":"1/1"},"lanes":[{"instance":"default","passed":"1/1"}],"batch":"C:\\b"}"#;
const MIXED_CAP1: &str = r##"{"ok":false,"runs":4,"passed":1,"failed":3,"failed_known":2,"failed_new":1,"rows":{"FS-P1":"3/3","FS-P2":"1/3"},"lanes":[{"instance":"default","passed":"1/2"},{"instance":"p2","passed":"0/2","braked":true}],"failures":[{"n":2,"why":"FS-P2 FAIL: clause movie-over failed","issue":"#1341","runs":["default#2","p2#1"]}],"more_failure_groups":1,"batch":"C:\\b"}"##;
const REALISTIC: &str = r#"{"ok":true,"runs":10,"passed":10,"failed":0,"rows":{"FS-01":"10/10","FS-02":"10/10","FS-P1":"10/10","FS-P2":"10/10","FS-P3":"10/10","FS-P4":"10/10","FS-P5":"10/10"},"lanes":[{"instance":"default","passed":"5/5"},{"instance":"p2","passed":"5/5"}],"batch":"C:\\Users\\someone\\AppData\\Local\\cimmeria-lab\\uat-runs\\batch-20261010-183832"}"#;
const FIVE_GOLDEN: &str = r#"{"ok":true,"runs":5,"passed":5,"failed":0,"rows":{"FS-01":"5/5","FS-02":"5/5","FS-P1":"5/5","FS-P2":"5/5","FS-P3":"5/5","FS-P4":"5/5","FS-P5":"5/5"},"lanes":[{"instance":"default","passed":"5/5"}],"golden":{"match":3,"diverged":2,"first":"row FS-P3: event 4 expected dialogDisplay(dialog=3995), got dialogDisplay(dialog=2982); after mission(688).step(2)=ac..."},"batch":"C:\\Users\\someone\\AppData\\Local\\cimmeria-lab\\uat-runs\\batch-20261010-183832"}"#;
const FIVE_HEAL: &str = r#"{"ok":false,"runs":5,"passed":0,"failed":5,"failed_known":0,"failed_new":5,"rows":{"FS-01":"5/5","FS-02":"5/5","FS-P1":"5/5","FS-P2":"5/5","FS-P3":"0/5"},"lanes":[{"instance":"default","passed":"0/5"}],"failures":[{"n":5,"why":"FS-P3 FAIL: clause hover failed: frost not under the cursor","runs":["default#1","default#2","default#3","default#4","default#5"]}],"heal":[{"row":"FS-P3","pose":"frost","diff":"heal-FS-P3-frost.diff"},{"row":"FS-P3","pose":"supply","diff":"heal-FS-P3-supply.diff"},{"row":"FS-P3","pose":"guard","diff":"heal-FS-P3-guard.diff"}],"more_heal":2,"batch":"C:\\Users\\someone\\AppData\\Local\\cimmeria-lab\\uat-runs\\batch-20261010-183832"}"#;
const CHAOS_ONE: &str = r#"{"ok":true,"runs":1,"passed":1,"failed":0,"rows":{"FS-01":"1/1","FS-02":"1/1","FS-P1":"1/1","FS-P2":"1/1","FS-P3":"1/1","FS-P4":"1/1"},"lanes":[{"instance":"default","passed":"1/1"}],"chaos_divergence":{"match":0,"diverged":1,"first":"row FS-P3: expected `s2c onDialogDisplay@self dialog_id=3995` after `c2s interact target=tag:ArmYourself_FrostBody`, ..."},"chaos":"chaos=flush:pack seed=7","batch":"C:\\Users\\someone\\AppData\\Local\\cimmeria-lab\\uat-runs\\batch-20261011-120000"}"#;

fn parse(text: &str) -> Value {
    serde_json::from_str(text).expect("one JSON object")
}

fn group(line: &str, count: u32, runs: &[&str]) -> GoldenGroup {
    GoldenGroup {
        line: line.into(),
        count,
        runs: runs.iter().map(|r| r.to_string()).collect(),
    }
}

fn golden(matched: u32, diverged: u32, errors: u32, groups: Vec<GoldenGroup>) -> GoldenSummary {
    GoldenSummary {
        matched,
        diverged,
        errors,
        groups,
    }
}

fn rows_of(ids: &[&str], pass: u32, total: u32) -> Vec<(String, RowStat)> {
    ids.iter().map(|r| rs(r, pass, total)).collect()
}

fn clean(rows: Vec<(String, RowStat)>, runs: u32, lane_passed: &[(&str, u32, u32)]) -> Summary {
    Summary {
        ok: true,
        runs,
        passed: runs,
        failed: 0,
        rows: RowStats(rows),
        lanes: lane_passed
            .iter()
            .enumerate()
            .map(|(i, (inst, runs, passed))| ls(i as u32 + 1, inst, *runs, *passed, false))
            .collect(),
        ..Default::default()
    }
}

/// test-uat.ps1 `$s`: 4 runs on two lanes, p2 braked; the biggest group is
/// stored last so the sort is what puts it first.
fn mixed() -> Summary {
    Summary {
        ok: false,
        runs: 4,
        passed: 1,
        failed: 3,
        failed_known: 2,
        failed_new: 1,
        rows: RowStats(vec![rs("FS-P1", 3, 3), rs("FS-P2", 1, 3)]),
        lanes: vec![ls(1, "default", 2, 1, false), ls(2, "p2", 2, 0, true)],
        failures: vec![
            FailureGroup {
                signature: "error: lab_uat_run failed: bridge down".into(),
                count: 1,
                issue: None,
                runs: vec!["p2#2".into()],
            },
            FailureGroup {
                signature: "FS-P2 FAIL: clause movie-over failed".into(),
                count: 2,
                issue: Some("#1341".into()),
                runs: vec!["default#2".into(), "p2#1".into()],
            },
        ],
        ..Default::default()
    }
}

fn five_golden_summary(groups: Vec<GoldenGroup>, matched: u32, diverged: u32) -> Summary {
    let mut s = clean(rows_of(&PRAXIS, 5, 5), 5, &[("default", 5, 5)]);
    s.golden = Some(golden(matched, diverged, 0, groups));
    s
}

/// test-uat.ps1 [86-87] 'rows read as pass/total'
#[test]
fn rows_read_as_pass_over_total() {
    let j = parse(&compact_json_with(&mixed(), BATCH_SHORT, 1, MAX_HEAL));
    assert_eq!(j["rows"]["FS-P2"], "1/3");
    assert_eq!(j["rows"]["FS-P1"], "3/3");
}

/// [88] 'failure groups are capped, with a count of the rest'
#[test]
fn failure_groups_are_capped_with_a_count_of_the_rest() {
    let j = parse(&compact_json_with(&mixed(), BATCH_SHORT, 1, MAX_HEAL));
    assert_eq!(j["failures"].as_array().unwrap().len(), 1);
    assert_eq!(j["more_failure_groups"], 1);
}

/// [89] 'the biggest group comes first, tagged'
#[test]
fn the_biggest_group_comes_first_tagged() {
    let j = parse(&compact_json(&mixed(), BATCH_SHORT));
    assert_eq!(j["failures"][0]["issue"], "#1341");
    assert_eq!(j["failures"][0]["n"], 2);
    assert_eq!(j["failures"][1]["n"], 1);
    assert!(j["failures"][1].get("issue").is_none());
}

/// New: the whole object, key order included (the `json!` trap: sorted keys).
#[test]
fn a_failed_batch_is_this_exact_line() {
    assert_eq!(
        compact_json_with(&mixed(), BATCH_SHORT, 1, MAX_HEAL),
        MIXED_CAP1
    );
}

/// [90-91] 'a clean batch has no nulls and no failure list'
#[test]
fn a_clean_batch_has_no_nulls_and_no_failure_list() {
    let s = clean(
        vec![rs("FS-P1", 1, 1), rs("FS-P2", 1, 1)],
        1,
        &[("default", 1, 1)],
    );
    let text = compact_json(&s, BATCH_SHORT);
    assert_eq!(text, CLEAN);
    assert!(!text.contains("null") && !text.contains("failures"));
}

/// [92-97] 'a realistic clean batch stays a few hundred characters'
#[test]
fn a_realistic_clean_batch_stays_under_500_characters() {
    let s = clean(
        rows_of(&PRAXIS, 10, 10),
        10,
        &[("default", 5, 5), ("p2", 5, 5)],
    );
    let text = compact_json(&s, BATCH_LONG);
    assert_eq!(text, REALISTIC);
    assert_eq!(text.len(), 338);
    assert!(text.len() < 500);
}

/// Review fix: `Select-Object -First 5` on a group's runs (the count `n` stays whole).
#[test]
fn a_failure_group_lists_at_most_five_runs() {
    let mut s = failed_batch();
    s.failures[0].count = 7;
    s.failures[0].runs = (1..=7).map(|i| format!("default#{i}")).collect();
    let j = parse(&compact_json(&s, BATCH_SHORT));
    assert_eq!(j["failures"][0]["n"], 7);
    assert_eq!(j["failures"][0]["runs"].as_array().unwrap().len(), 5);
    assert_eq!(j["failures"][0]["runs"][4], "default#5");
}

/// [98-99] 'a long failure reason is capped at 200 characters'
#[test]
fn a_long_failure_reason_is_capped_at_200_characters() {
    let mut s = mixed();
    s.failures = vec![FailureGroup {
        signature: format!("FS-P3 FAIL: {}", "x".repeat(600)),
        count: 1,
        issue: None,
        runs: vec!["default#1".into()],
    }];
    let j = parse(&compact_json(&s, BATCH_SHORT));
    let why = j["failures"][0]["why"].as_str().unwrap();
    assert_eq!(why.chars().count(), 200);
    assert!(why.ends_with("..."));
    assert!(why.starts_with("FS-P3 FAIL: xxx"));
}

/// [242-249] 'compact_json_with_golden_stays_under_500_chars': 5 runs, two
/// distinct divergences, `first` is the most common (first seen on a tie) cut
/// to 120 characters; no `errors` key.
#[test]
fn compact_json_with_golden_stays_under_500_chars() {
    let s = five_golden_summary(
        vec![
            group(DIV_A, 1, &["default#2"]),
            group(DIV_B, 1, &["default#4"]),
        ],
        3,
        2,
    );
    let text = compact_json(&s, BATCH_LONG);
    assert_eq!(text, FIVE_GOLDEN);
    assert_eq!(text.len(), 454);
    assert!(!text.contains("null") && !text.contains("errors"));
}

/// [250-253] 'the compact first line is capped at 120 characters'
#[test]
fn the_compact_first_line_is_capped_at_120_characters() {
    let long = format!("row FS-P3: {}", "z".repeat(290));
    let s = five_golden_summary(vec![group(&long, 1, &["default#1"])], 0, 1);
    let j = parse(&compact_json(&s, BATCH_SHORT));
    let first = j["golden"]["first"].as_str().unwrap();
    assert_eq!(first.chars().count(), 120);
    assert!(first.ends_with("zzz..."));
}

/// [254-260] 'compact_first_is_a_divergence_not_an_error': 3 errors outnumber 2
/// identical divergences, but `first` is the divergence and `errors` is 3.
#[test]
fn compact_first_is_a_divergence_not_an_error() {
    let mut s = five_golden_summary(
        vec![
            group(CANNOT, 3, &["default#1", "default#2", "default#3"]),
            group(DIV_A, 2, &["default#4", "default#5"]),
        ],
        0,
        2,
    );
    s.golden.as_mut().unwrap().errors = 3;
    let j = parse(&compact_json(&s, BATCH_SHORT));
    assert_eq!(j["golden"]["errors"], 3);
    assert!(j["golden"]["first"]
        .as_str()
        .unwrap()
        .starts_with("row FS-P3: event 4"));
    assert!(j["golden"].get("error").is_none());
}

/// [261-262] 'with no divergence the compact golden names the first error,
/// not a first line' (and errors stays before error: key order).
#[test]
fn with_no_divergence_the_compact_golden_names_the_first_error() {
    let mut s = five_golden_summary(vec![group(CANNOT, 1, &["default#2"])], 1, 0);
    s.golden.as_mut().unwrap().errors = 1;
    let text = compact_json(&s, BATCH_SHORT);
    assert!(text.contains(
        r#""golden":{"match":1,"diverged":0,"errors":1,"error":"error: cannot_diff: run d: no fingerprint.json"}"#
    ));
    assert!(parse(&text)["golden"].get("first").is_none());
}

/// [263-264] 'the compact error line is capped at 120 characters'
#[test]
fn the_compact_error_line_is_capped_at_120_characters() {
    let long = format!("error: {}", "e".repeat(300));
    let mut s = five_golden_summary(vec![group(&long, 1, &["default#1"])], 0, 0);
    s.golden.as_mut().unwrap().errors = 1;
    let j = parse(&compact_json(&s, BATCH_SHORT));
    assert_eq!(j["golden"]["error"].as_str().unwrap().chars().count(), 120);
}

/// [265-273] 'compact_json_with_200_char_divergences_stays_under_500_chars'
#[test]
fn compact_json_with_200_char_divergences_stays_under_500_chars() {
    let a = format!("row FS-P3: {}", "w".repeat(189));
    let b = format!("row FS-P5: {}", "v".repeat(189));
    assert_eq!(a.chars().count(), 200);
    let s = five_golden_summary(
        vec![group(&a, 1, &["default#2"]), group(&b, 1, &["default#4"])],
        3,
        2,
    );
    let text = compact_json(&s, BATCH_LONG);
    assert_eq!(text.len(), 454);
}

/// [318-319] 'an all-match batch has no first line'
#[test]
fn an_all_match_batch_has_no_first_line() {
    let s = five_golden_summary(vec![], 5, 0);
    let text = compact_json(&s, BATCH_SHORT);
    assert!(text.contains(r#""golden":{"match":5,"diverged":0},"batch""#));
}

/// [320-322, JSON half] 'a batch without Golden fields prints no golden'
#[test]
fn a_batch_without_golden_prints_no_golden_key() {
    let s = clean(vec![rs("FS-P1", 1, 1)], 1, &[("default", 1, 1)]);
    assert!(!compact_json(&s, BATCH_SHORT).contains("golden"));
}

fn failed_batch() -> Summary {
    Summary {
        ok: false,
        runs: 5,
        passed: 0,
        failed: 5,
        failed_known: 0,
        failed_new: 5,
        rows: RowStats(vec![
            rs("FS-01", 5, 5),
            rs("FS-02", 5, 5),
            rs("FS-P1", 5, 5),
            rs("FS-P2", 5, 5),
            rs("FS-P3", 0, 5),
        ]),
        lanes: vec![ls(1, "default", 5, 0, false)],
        failures: vec![FailureGroup {
            signature: "FS-P3 FAIL: clause hover failed: frost not under the cursor".into(),
            count: 5,
            issue: None,
            runs: (1..=5).map(|i| format!("default#{i}")).collect(),
        }],
        ..Default::default()
    }
}

fn heal_entries(poses: &[String]) -> Vec<HealEntry> {
    poses
        .iter()
        .map(|p| HealEntry {
            row: "FS-P3".into(),
            pose: p.clone(),
            diff: format!(r"{BATCH_LONG}\heal-FS-P3-{p}.diff"),
        })
        .collect()
}

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

/// [429-431] '-Json lists heal as {row, pose, diff}, diff named in the batch
/// folder'; 'one proposal has no more_heal'
#[test]
fn heal_is_listed_as_row_pose_diff_with_the_file_name() {
    let mut s = failed_batch();
    s.heal = vec![HealEntry {
        row: "FS-P3".into(),
        pose: "frost".into(),
        diff: r"C:\tmp\batch\heal-FS-P3-frost.diff".into(),
    }];
    let text = compact_json(&s, r"C:\tmp\batch");
    assert!(text.contains(
        r#""heal":[{"row":"FS-P3","pose":"frost","diff":"heal-FS-P3-frost.diff"}],"batch":"C:\\tmp\\batch"}"#
    ));
    assert!(!text.contains("more_heal"));
}

/// [534-544] 'five proposals: three listed by file name, more_heal 2'
#[test]
fn five_proposals_list_three_and_count_the_rest() {
    let mut s = failed_batch();
    s.heal = heal_entries(&names(&["frost", "supply", "guard", "crate", "console"]));
    assert_eq!(compact_json(&s, BATCH_LONG), FIVE_HEAL);
}

/// [545-551] 'heal adds at most 240 characters, for 5 or 40 proposals' and 'a
/// failed batch with heal proposals stays under 700 characters'
#[test]
fn heal_adds_at_most_240_characters_and_stays_under_700() {
    let none = compact_json(&failed_batch(), BATCH_LONG);
    assert_eq!(none.len(), 452);
    let mut s = failed_batch();
    s.heal = heal_entries(&names(&["frost", "supply", "guard", "crate", "console"]));
    let five = compact_json(&s, BATCH_LONG);
    s.heal = heal_entries(&(1..=40).map(|i| format!("pose{i}")).collect::<Vec<_>>());
    let forty = compact_json(&s, BATCH_LONG);
    assert_eq!(
        (five.len() - none.len(), forty.len() - none.len()),
        (211, 210)
    );
    assert!(none.len() < 500 && five.len() < 700 && forty.len() < 700);
    assert!(forty.contains(r#""more_heal":37"#));
}

/// [559-562, JSON half] 'a batch without a proposal has no heal in -Json'
#[test]
fn a_batch_without_a_proposal_has_no_heal_key() {
    assert!(!compact_json(&failed_batch(), BATCH_LONG).contains("heal"));
}

/// Review fix: `ConvertTo-Json -Compress` writes U+0085, U+2028 and U+2029 as
/// `\uXXXX` (serde_json writes them raw) and leaves `< > & '`, DEL, U+00A0 and
/// astral characters as they are. Expected text taken from pwsh 7.
#[test]
fn line_separators_are_escaped_as_powershell_does() {
    let mut s = mixed();
    s.failures = vec![FailureGroup {
        signature: "error: a<b>&'c\u{2028}\u{85}\u{2029}\u{7f}\u{a0}x\u{1b}\u{1F600}".into(),
        count: 1,
        issue: None,
        runs: vec!["p2#2".into()],
    }];
    let text = compact_json(&s, BATCH_SHORT);
    assert!(
        text.contains(r#""why":"error: a<b>&'c\u2028\u0085\u2029"#)
            && text.contains("\u{7f}\u{a0}x\\u001b\u{1F600}\""),
        "{text}"
    );
}

/// Review fix: `Split-Path -Leaf` ignores a trailing separator.
#[test]
fn a_heal_diff_path_with_a_trailing_separator_still_has_a_file_name() {
    let mut s = failed_batch();
    s.heal = vec![HealEntry {
        row: "FS-P3".into(),
        pose: "frost".into(),
        diff: r"C:\tmp\batch\heal-dir\".into(),
    }];
    assert!(compact_json(&s, BATCH_SHORT).contains(r#""diff":"heal-dir""#));
}

/// tc:36-52 '-Json has the chaos string when the run had chaos' and 'a run
/// without chaos has no chaos key'
#[test]
fn the_chaos_header_is_the_top_level_chaos_key() {
    let mut s = clean(vec![rs("FS-P1", 1, 1)], 1, &[("default", 1, 1)]);
    s.chaos = Some("chaos=cpu; chaos=flush:pack".into());
    let text = compact_json(&s, BATCH_SHORT);
    assert!(text.ends_with(r#","chaos":"chaos=cpu; chaos=flush:pack","batch":"C:\\b"}"#));
    s.chaos = None;
    assert!(!compact_json(&s, BATCH_SHORT).contains("chaos"));
}

/// tc:205-216 'a chaos row that passed but diverged keeps status pass'; 'a
/// chaos batch reports no golden key'; the divergence under chaos_divergence
/// with `first` cut to 120; the header once, as the top-level chaos key
#[test]
fn a_chaos_divergence_reports_under_chaos_divergence_not_golden() {
    let mut s = clean(
        rows_of(
            &["FS-01", "FS-02", "FS-P1", "FS-P2", "FS-P3", "FS-P4"],
            1,
            1,
        ),
        1,
        &[("default", 1, 1)],
    );
    s.chaos_divergence = Some(golden(
        0,
        1,
        0,
        vec![group(DIV_LINE, 1, &["default#1 seed=7"])],
    ));
    s.chaos = Some("chaos=flush:pack seed=7".into());
    assert_eq!(compact_json(&s, BATCH_CHAOS), CHAOS_ONE);
}

/// tc:221-234 '-Json for 5 chaos runs of 6 rows, a two-spec header and 20-digit
/// seeds stays under 600 characters'; '-Json does not repeat the seeds per
/// divergence'
#[test]
fn five_chaos_runs_stay_under_600_characters() {
    let seeds = [
        "12345678901234567890",
        "98765432109876543210",
        "11112222333344445555",
        "55554444333322221111",
        "10293847561029384756",
    ];
    let refs: Vec<String> = seeds
        .iter()
        .enumerate()
        .map(|(i, s)| format!("default#{} seed={s}", i + 1))
        .collect();
    let ref_strs: Vec<&str> = refs.iter().map(String::as_str).collect();
    let mut s = clean(
        rows_of(
            &["FS-01", "FS-02", "FS-P1", "FS-P2", "FS-P3", "FS-P4"],
            5,
            5,
        ),
        5,
        &[("default", 5, 5)],
    );
    s.chaos_divergence = Some(golden(0, 5, 0, vec![group(DIV_LINE, 5, &ref_strs)]));
    s.chaos =
        Some("chaos=cpu:safe seed=<per run>; chaos=net:latency=120,loss=1 seed=<per run>".into());
    let text = compact_json(&s, BATCH_CHAOS);
    assert_eq!(text.len(), 535);
    assert!(text.len() < 600);
    assert!(!text.contains("12345678901234567890") && !text.contains("seed=98765432109876543210"));
}

/// tc:236-241 'a chaos run that matched gives chaos_divergence {match: 1,
/// diverged: 0}' and no golden key
#[test]
fn a_matched_chaos_batch_reports_match_1_diverged_0() {
    let mut s = clean(vec![rs("FS-01", 1, 1)], 1, &[("default", 1, 1)]);
    s.chaos_divergence = Some(golden(1, 0, 0, vec![]));
    let text = compact_json(&s, BATCH_SHORT);
    assert!(text.contains(r#""chaos_divergence":{"match":1,"diverged":0},"batch""#));
    assert!(!text.contains("\"golden\""));
}
