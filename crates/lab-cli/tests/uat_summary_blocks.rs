//! LX-17b: the golden, chaos and heal blocks of summary.md (format_markdown).
//! Ports test-uat.ps1 [323-325, 193-194, 320-322, 432-433, 552-555, 561-562]
//! and test-uat-chaos.ps1 [41-52, 218-219].

mod uat_support;

use cimmeria_lab_cli::commands::uat::golden::{GoldenGroup, GoldenSummary};
use cimmeria_lab_cli::commands::uat::summary::{format_markdown, HealEntry, RowStats, Summary};
use uat_support::batch_fx::*;

const BATCH: &str = r"C:\b";

fn group(line: &str, count: u32, runs: &[&str]) -> GoldenGroup {
    GoldenGroup {
        line: line.into(),
        count,
        runs: runs.iter().map(|r| r.to_string()).collect(),
    }
}

fn base() -> Summary {
    Summary {
        ok: true,
        runs: 5,
        passed: 5,
        rows: RowStats(vec![rs("FS-P1", 5, 5)]),
        lanes: vec![ls(1, "default", 5, 5, false)],
        ..Default::default()
    }
}

/// [323-324] 'the Markdown summary has a Golden block with each line and its
/// count'
#[test]
fn the_markdown_summary_has_a_golden_block() {
    let mut s = base();
    s.golden = Some(GoldenSummary {
        matched: 2,
        diverged: 3,
        errors: 0,
        groups: vec![
            group(DIV_A, 2, &["default#2", "default#4"]),
            group(DIV_B, 1, &["default#5"]),
        ],
    });
    let md = format_markdown(&s, "lab uat first-session", BATCH);
    assert!(md.contains("Golden: 2 match, 3 diverged"));
    assert!(md.contains(&format!("- 2x {DIV_A}; runs: default#2, default#4")));
    assert!(md.contains(&format!("- 1x {DIV_B}")));
}

/// [325] 'the Golden block counts runs that could not be compared'
#[test]
fn the_golden_block_counts_runs_that_could_not_be_compared() {
    let mut s = base();
    s.golden = Some(GoldenSummary {
        matched: 1,
        diverged: 0,
        errors: 1,
        groups: vec![group(CANNOT, 1, &["default#2"])],
    });
    assert!(format_markdown(&s, "t", BATCH)
        .contains("Golden: 1 match, 0 diverged, 1 could not be compared"));
}

/// [320-322, text half] 'a batch without Golden fields prints no golden'
#[test]
fn a_batch_without_golden_prints_no_golden_block() {
    assert!(!format_markdown(&base(), "t", BATCH).contains("Golden"));
}

/// tc:218-219 'summary.md lists the chaos divergence with its run and seed'
#[test]
fn summary_md_lists_the_chaos_divergence_with_its_run_and_seed() {
    let mut s = base();
    s.chaos_divergence = Some(GoldenSummary {
        matched: 0,
        diverged: 1,
        errors: 0,
        groups: vec![group(DIV_LINE, 1, &["default#1 seed=7"])],
    });
    let md = format_markdown(&s, "t", BATCH);
    assert!(md.contains("Chaos divergence from the golden: 0 match, 1 diverged"));
    assert!(md.contains(&format!("- 1x {DIV_LINE}; runs: default#1 seed=7")));
}

/// tc:39-52 'the chaos header leads the summary'; 'a run without chaos has no
/// chaos header'
#[test]
fn the_chaos_header_leads_the_summary() {
    let mut s = base();
    s.chaos = Some("chaos=cpu; chaos=flush:pack".into());
    assert!(format_markdown(&s, "t", BATCH).starts_with("chaos=cpu; chaos=flush:pack"));
    s.chaos = None;
    assert!(!format_markdown(&s, "t", BATCH).contains("chaos="));
}

/// [552-555] 'summary.md lists every proposal with its full path'; [561-562]
/// 'a batch without a proposal has no heal in summary.md'
#[test]
fn summary_md_lists_every_proposal_with_its_full_path() {
    let mut s = base();
    assert!(!format_markdown(&s, "t", BATCH).contains("Heal"));
    let dir = r"C:\Users\someone\AppData\Local\cimmeria-lab\uat-runs\batch-20261010-183832";
    s.heal = ["frost", "supply", "guard", "crate", "console"]
        .iter()
        .map(|p| HealEntry {
            row: "FS-P3".into(),
            pose: (*p).into(),
            diff: format!(r"{dir}\heal-FS-P3-{p}.diff"),
        })
        .collect();
    let md = format_markdown(&s, "t", dir);
    assert!(md.contains(&format!(r"- FS-P3 console: {dir}\heal-FS-P3-console.diff")));
    assert!(md.contains("Heal proposals (spec diffs to review; nothing was applied):"));
}

/// [193-194] 'the Markdown summary names the verdict and the known issue'
#[test]
fn the_markdown_summary_names_the_verdict_and_the_known_issue() {
    let s = Summary {
        ok: false,
        runs: 4,
        passed: 1,
        failed: 3,
        failed_known: 2,
        failed_new: 1,
        failures: vec![cimmeria_lab_cli::commands::uat::summary::FailureGroup {
            signature: "FS-P2 FAIL: clause movie-over failed".into(),
            count: 2,
            issue: Some("#1341".into()),
            runs: vec!["default#2".into(), "p2#1".into()],
        }],
        lanes: vec![ls(1, "default", 2, 1, false)],
        ..Default::default()
    };
    let md = format_markdown(&s, "lab uat first-session", BATCH);
    assert!(md.contains("FAIL (1/4 runs passed)") && md.contains("known: #1341"));
}
