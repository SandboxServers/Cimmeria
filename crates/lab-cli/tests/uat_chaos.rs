//! LX-17a: the batch-level chaos helpers.

mod uat_support;

use cimmeria_lab_cli::batch::RunRecord;
use cimmeria_lab_cli::commands::uat::chaos::{run_ref, select_header, select_runs};
use uat_support::batch_fx::*;

fn with_header(index: u32, header: Option<&str>) -> RunRecord {
    let mut r = run("default", index, true, good_rows());
    r.chaos = header.map(str::to_string);
    r
}

fn errored(index: u32) -> RunRecord {
    let mut r = with_header(index, None);
    r.error = Some("x".into());
    r
}

/// tc:57-58 'the runs' header replaces the typed one, not appended to it'
#[test]
fn the_runs_header_replaces_the_typed_one() {
    let runs = vec![
        with_header(1, Some("chaos=cpu seed=5")),
        with_header(2, Some("chaos=cpu seed=5")),
    ];
    assert_eq!(
        select_header(&runs, Some("chaos=cpu")).as_deref(),
        Some("chaos=cpu seed=5")
    );
}

/// tc:59-60 'runs that differ only by seed name the seed once as <per run>'
#[test]
fn runs_that_differ_only_by_seed_name_the_seed_once() {
    let runs = vec![
        with_header(1, Some("chaos=cpu seed=5")),
        with_header(2, Some("chaos=cpu seed=9")),
        errored(3),
    ];
    assert_eq!(
        select_header(&runs, Some("chaos=cpu")).as_deref(),
        Some("chaos=cpu seed=<per run>")
    );
}

/// Review fix: `-replace 'seed=\d+'` ignores case, so `SEED=5` and `SEED=9`
/// name the seed once too (checked in pwsh 7).
#[test]
fn the_seed_is_found_whatever_its_case() {
    let runs = vec![
        with_header(1, Some("chaos=cpu SEED=5")),
        with_header(2, Some("chaos=cpu SEED=9")),
    ];
    assert_eq!(
        select_header(&runs, None).as_deref(),
        Some("chaos=cpu seed=<per run>")
    );
}

/// New: headers that differ beyond the seed are counted, not listed.
#[test]
fn runs_that_differ_beyond_the_seed_are_counted() {
    let runs = vec![
        with_header(1, Some("chaos=cpu seed=5")),
        with_header(2, Some("chaos=flush:pack seed=9")),
    ];
    assert_eq!(
        select_header(&runs, None).as_deref(),
        Some("chaos=cpu seed=<per run> (+1 other spec(s) in batch.jsonl)")
    );
}

/// tc:61 'no run returned a header: the typed one stays'
#[test]
fn no_run_header_keeps_the_typed_one() {
    assert_eq!(
        select_header(&[errored(1)], Some("chaos=cpu")).as_deref(),
        Some("chaos=cpu")
    );
}

/// tc:62 'no chaos at all gives no header'
#[test]
fn no_chaos_at_all_gives_no_header() {
    assert_eq!(select_header(&[], None), None);
    assert_eq!(select_header(&[], Some("")), None);
}

/// New (Get-UatChaosRunRef): the run's seed, once per distinct value.
#[test]
fn a_chaos_run_ref_names_the_runs_seed() {
    assert_eq!(
        run_ref(&with_header(1, Some("chaos=flush:pack seed=7"))),
        "default#1 seed=7"
    );
    let two = "chaos=cpu:safe seed=12345678901234567890; chaos=net:latency=120,loss=1 seed=12345678901234567890";
    assert_eq!(
        run_ref(&with_header(2, Some(two))),
        "default#2 seed=12345678901234567890"
    );
}

/// New: a header without a seed is shown whole; no header is the plain ref.
#[test]
fn a_header_without_a_seed_is_shown_whole() {
    assert_eq!(
        run_ref(&with_header(1, Some("chaos=cpu"))),
        "default#1 (chaos=cpu)"
    );
    assert_eq!(run_ref(&with_header(1, None)), "default#1");
}

/// New (Select-UatChaosRuns): the header splits the runs; an empty header is none.
#[test]
fn the_header_splits_chaos_runs_from_the_rest() {
    let runs = vec![
        with_header(1, Some("chaos=cpu seed=1")),
        with_header(2, None),
        with_header(3, Some("")),
    ];
    assert_eq!(select_runs(&runs, true).len(), 1);
    assert_eq!(select_runs(&runs, false).len(), 2);
}
