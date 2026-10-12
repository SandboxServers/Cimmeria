//! LX-17a: the exit code, and what `merge` hands it for golden and chaos
//! batches (Step A2 replaced `exit_code`; the first four tests also exist in
//! LX-16a and guard that the codes 0, 1 and 4 survived the replacement). Ports the exit-code checks of the removed PowerShell test file and
//! the removed PowerShell test file.

mod uat_support;

use cimmeria_lab_cli::batch::RunRecord;
use cimmeria_lab_cli::commands::uat::summary::{exit_code, merge, HealEntry, Summary};
use uat_support::batch_fx::*;

fn merged(runs: Vec<RunRecord>, braked: &[&str]) -> Summary {
    let braked: Vec<String> = braked.iter().map(|s| s.to_string()).collect();
    merge(&runs, &plan_lanes(), &braked)
}

fn g(i: u32, golden: &str) -> RunRecord {
    with_golden(run("default", i, true, good_rows()), golden)
}

fn g_runs() -> Vec<RunRecord> {
    vec![
        g(1, "match"),
        g(2, DIV_A),
        g(3, "match"),
        g(4, DIV_A),
        g(5, DIV_B),
    ]
}

fn heal_entry() -> Vec<HealEntry> {
    vec![HealEntry {
        row: "FS-P3".into(),
        pose: "aim".into(),
        diff: r"C:\b\heal-FS-P3-aim.diff".into(),
    }]
}

fn four_runs() -> Vec<RunRecord> {
    let mut bridge = run("p2", 2, false, vec![]);
    bridge.error = Some("lab_uat_run failed: bridge down".into());
    vec![
        run("default", 1, true, good_rows()),
        run("default", 2, false, frost_rows()),
        run("p2", 1, false, frost_rows()),
        bridge,
    ]
}

/// the removed PowerShell test file [79] 'a braked lane exits 4'
#[test]
fn a_braked_lane_exits_4() {
    assert_eq!(exit_code(&merged(four_runs(), &["p2"])), 4);
}

/// [80-81] 'a failed run exits 1'
#[test]
fn a_failed_run_exits_1() {
    assert_eq!(
        exit_code(&merged(vec![run("default", 1, false, frost_rows())], &[])),
        1
    );
}

/// [82-83] 'all passed exits 0'
#[test]
fn all_passed_exits_0() {
    let s = merged(vec![run("default", 1, true, good_rows())], &[]);
    assert!(s.ok);
    assert_eq!(exit_code(&s), 0);
}

/// [120-121] 'no finished runs is not a pass'
#[test]
fn no_finished_runs_is_not_a_pass() {
    let s = merged(vec![], &[]);
    assert_eq!(s.runs, 0);
    assert!(!s.ok);
    assert_eq!(exit_code(&s), 1);
}

/// [225-226] 'a divergence leaves every run passed and the batch ok' and
/// 'a_divergence_on_passing_runs_exits_5'
#[test]
fn a_divergence_on_passing_runs_exits_5() {
    let s = merged(g_runs(), &[]);
    assert!(s.ok && s.passed == 5 && s.failed == 0);
    assert_eq!(exit_code(&s), 5);
}

/// [227-228] 'every run matching the golden exits 0'
#[test]
fn every_run_matching_the_golden_exits_0() {
    let s = merged((1..=5).map(|i| g(i, "match")).collect(), &[]);
    assert_eq!(s.golden.as_ref().unwrap().matched, 5);
    assert_eq!(exit_code(&s), 0);
}

/// [229-230] 'a run the diff could not compare is counted as an error and is
/// not a match (exit 5)'
#[test]
fn a_run_the_diff_could_not_compare_exits_5() {
    let s = merged(vec![g(1, "match"), g(2, CANNOT)], &[]);
    let golden = s.golden.as_ref().unwrap();
    assert_eq!((golden.errors, golden.matched), (1, 1));
    assert_eq!(exit_code(&s), 5);
}

/// [231-232] 'a failed run exits 1 even when another run diverged'
#[test]
fn a_failed_run_exits_1_even_when_another_run_diverged() {
    let failed = with_golden(run("default", 1, false, frost_rows()), DIV_A);
    let s = merged(vec![failed, g(2, DIV_B)], &[]);
    assert_eq!(s.failed, 1);
    assert_eq!(exit_code(&s), 1);
}

/// [233-234] 'a braked lane exits 4 even with a divergence'
#[test]
fn a_braked_lane_exits_4_even_with_a_divergence() {
    let failed = with_golden(run("default", 1, false, frost_rows()), DIV_A);
    assert_eq!(exit_code(&merged(vec![failed], &["default"])), 4);
}

/// [236-237] 'a heal proposal (8) wins over a golden divergence (5)'
#[test]
fn a_heal_proposal_wins_over_a_golden_divergence() {
    let mut s = merged(g_runs(), &[]);
    s.heal = heal_entry();
    assert_eq!(exit_code(&s), 8);
}

/// [238-239] 'a heal proposal (8) wins over a failed run (1)'
#[test]
fn a_heal_proposal_wins_over_a_failed_run() {
    let mut s = merged(vec![run("default", 1, false, frost_rows())], &[]);
    assert_eq!(exit_code(&s), 1);
    s.heal = heal_entry();
    assert_eq!(exit_code(&s), 8);
}

/// [240-241] 'an empty heal list is no proposal'
#[test]
fn an_empty_heal_list_is_no_proposal() {
    let mut s = merged(g_runs(), &[]);
    s.heal = vec![];
    assert_eq!(exit_code(&s), 5);
}

/// [426-428, 434-437] 'a heal proposal exits 8' and '8 wins over 5 when both
/// apply'
#[test]
fn a_heal_proposal_exits_8_over_a_divergence() {
    let mut s = merged(vec![g(1, DIV_A)], &[]);
    assert_eq!(exit_code(&s), 5);
    s.heal = heal_entry();
    assert_eq!(exit_code(&s), 8);
}

fn chaos_rec(golden: Option<&str>, chaos: &str, index: u32) -> RunRecord {
    let rows = ["FS-01", "FS-02", "FS-P1", "FS-P2", "FS-P3", "FS-P4"]
        .iter()
        .map(|r| row(r, "PASS", ""))
        .collect();
    let mut r = run("default", index, true, rows);
    r.golden = golden.map(str::to_string);
    if !chaos.is_empty() {
        r.chaos = Some(chaos.to_string());
    }
    r
}

const SEVEN: &str = "chaos=flush:pack seed=7";

/// tc:205-217 'the divergence is one chaos_divergence line, unprefixed'; 'the
/// group names its run with the run's seed'; 'a chaos-only divergence does not
/// exit 5'
#[test]
fn a_chaos_only_divergence_does_not_exit_5() {
    let s = merged(vec![chaos_rec(Some(DIV_LINE), SEVEN, 1)], &[]);
    assert!(s.ok && s.passed == 1);
    assert!(s.golden.is_none());
    let cd = s.chaos_divergence.as_ref().unwrap();
    assert_eq!(cd.diverged, 1);
    assert_eq!(cd.groups.len(), 1);
    assert_eq!(cd.groups[0].line, DIV_LINE);
    assert_eq!(cd.groups[0].runs, ["default#1 seed=7"]);
    assert_eq!(exit_code(&s), 0);
}

/// tc:221-232 '5 runs with the same divergence and different seeds are one
/// group of 5' and 'the group lists each run by its seed'
#[test]
fn five_chaos_runs_with_different_seeds_are_one_group_of_five() {
    let seeds = [
        "12345678901234567890",
        "98765432109876543210",
        "11112222333344445555",
        "55554444333322221111",
        "10293847561029384756",
    ];
    let runs = (0..5)
        .map(|i| {
            let header = format!(
                "chaos=cpu:safe seed={0}; chaos=net:latency=120,loss=1 seed={0}",
                seeds[i]
            );
            chaos_rec(Some(DIV_LINE), &header, i as u32 + 1)
        })
        .collect();
    let s = merged(runs, &[]);
    let cd = s.chaos_divergence.as_ref().unwrap();
    assert_eq!(
        (cd.groups.len(), cd.groups[0].count, cd.diverged),
        (1, 5, 5)
    );
    let want: Vec<String> = (0..5)
        .map(|i| format!("default#{} seed={}", i + 1, seeds[i]))
        .collect();
    assert_eq!(cd.groups[0].runs, want);
}

/// tc:236-241 'a chaos run that matched gives chaos_divergence {match: 1,
/// diverged: 0}, exit 0' and 'a chaos run never compared has no
/// chaos_divergence key'
#[test]
fn a_matched_or_uncompared_chaos_run_exits_0() {
    let matched = merged(vec![chaos_rec(Some("match"), SEVEN, 1)], &[]);
    let cd = matched.chaos_divergence.as_ref().unwrap();
    assert_eq!((cd.matched, cd.diverged), (1, 0));
    assert!(matched.golden.is_none());
    assert_eq!(exit_code(&matched), 0);
    let never = merged(vec![chaos_rec(None, SEVEN, 1)], &[]);
    assert!(never.chaos_divergence.is_none() && never.golden.is_none());
}

/// tc:242-244 'under chaos a diff that could not be made still exits 5'
#[test]
fn under_chaos_a_diff_that_could_not_be_made_still_exits_5() {
    let s = merged(
        vec![
            chaos_rec(Some("error: cannot_diff: run x has no run.json"), SEVEN, 1),
            chaos_rec(Some(DIV_LINE), SEVEN, 1),
        ],
        &[],
    );
    let cd = s.chaos_divergence.as_ref().unwrap();
    assert_eq!(cd.errors, 1);
    assert!(cd
        .groups
        .iter()
        .any(|g| g.line == "error: cannot_diff: run x has no run.json"));
    assert_eq!(exit_code(&s), 5);
}

/// tc:245-246 'without chaos the same divergence stays under golden and exits 5'
#[test]
fn without_chaos_the_same_divergence_exits_5() {
    let s = merged(vec![chaos_rec(Some(DIV_LINE), "", 1)], &[]);
    assert_eq!(s.golden.as_ref().unwrap().diverged, 1);
    assert!(s.chaos_divergence.is_none());
    assert_eq!(exit_code(&s), 5);
}

/// tc:247-249 'a mixed batch: the run without a header stays under golden
/// (exit 5), the chaos run under chaos_divergence'
#[test]
fn a_mixed_batch_splits_golden_from_chaos_divergence() {
    let s = merged(
        vec![
            chaos_rec(Some(DIV_LINE), SEVEN, 1),
            chaos_rec(Some("row FS-01: expected `a`, got `b`"), "", 2),
        ],
        &[],
    );
    let golden = s.golden.as_ref().unwrap();
    let chaos = s.chaos_divergence.as_ref().unwrap();
    assert_eq!(golden.diverged, 1);
    assert_eq!(golden.groups[0].line, "row FS-01: expected `a`, got `b`");
    assert_eq!(chaos.diverged, 1);
    assert_eq!(chaos.groups[0].line, DIV_LINE);
    assert_eq!(exit_code(&s), 5);
}
