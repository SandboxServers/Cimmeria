//! The first place a run leaves its golden, as one line a reader can act on
//! (README D-GD7). A run is compared under the golden's stored variance, not
//! the spec's, so a spec edit cannot quietly widen what a golden accepts
//! (README D-GD5): [`diff_run_dir`] fingerprints the run that way, and
//! [`diff_row`] refuses a run fingerprint built under any other variance.

use super::build::{fingerprint_run_with, RunMeta};
use super::event::{RowFingerprint, RunFingerprint, Variance};
use crate::uat::evidence::RunDir;

/// A quoted value longer than this is cut, so one huge event cannot push
/// the rest of the line out.
const MAX_VALUE_CHARS: usize = 120;
/// The whole line, so it fits a tool reply or a PR comment as one line.
const MAX_LINE_CHARS: usize = 300;

/// Which part of a row diverged, in the order [`diff_row`] compares them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The golden has the row; the run does not.
    Missing,
    /// The run was fingerprinted under a different variance than the
    /// golden's, so nothing else in it can be compared (README D-GD5).
    Variance,
    Tap,
    TapFault,
    Result,
    Clause,
    Transition,
    WindowsOpened,
    WindowsClosed,
    Event,
    Unordered,
}

/// The first difference in one row. `None` in `expected` or `got` is the
/// end of the row; `after` is the golden's element before the difference,
/// `None` when the difference is at the start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    pub row: String,
    pub part: Part,
    pub expected: Option<String>,
    pub got: Option<String>,
    pub after: Option<String>,
}

/// Every golden row's first divergence, in golden order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffReport {
    /// The first divergence of the first row that has one.
    pub first: Option<Divergence>,
    pub rows: Vec<(String, Option<Divergence>)>,
    /// Rows the run fingerprinted that the golden does not hold.
    pub not_in_golden: Vec<String>,
}

/// The first difference between a golden row and the same row of a run.
pub fn diff_row(golden: &RowFingerprint, run: &RowFingerprint) -> Option<Divergence> {
    let at = |part, expected: Option<&String>, got: Option<&String>, after: Option<&String>| {
        Some(Divergence {
            row: golden.row.clone(),
            part,
            expected: expected.cloned(),
            got: got.cloned(),
            after: after.cloned(),
        })
    };
    if golden.variance != run.variance {
        return at(Part::Variance, None, None, None);
    }
    if golden.tap != run.tap {
        return at(Part::Tap, Some(&golden.tap), Some(&run.tap), None);
    }
    // Only the run's faults count: a golden is never recorded from a
    // faulty tap, and a faulty run is missing messages it cannot show.
    if !run.tap_faults.is_empty() {
        return at(Part::TapFault, None, Some(&run.tap_faults.join(", ")), None);
    }
    if golden.result != run.result {
        return at(Part::Result, Some(&golden.result), Some(&run.result), None);
    }
    let sequences = [
        (Part::Clause, &golden.clauses, &run.clauses),
        (Part::Transition, &golden.transitions, &run.transitions),
    ];
    for (part, g, r) in sequences {
        if let Some(d) = first_difference(&golden.row, part, g, r) {
            return Some(d);
        }
    }
    let sets = [
        (
            Part::WindowsOpened,
            &golden.windows_opened,
            &run.windows_opened,
        ),
        (
            Part::WindowsClosed,
            &golden.windows_closed,
            &run.windows_closed,
        ),
    ];
    for (part, g, r) in sets {
        if g != r {
            return at(part, Some(&g.join(", ")), Some(&r.join(", ")), None);
        }
    }
    first_difference(&golden.row, Part::Event, &golden.events, &run.events).or_else(|| {
        first_difference(
            &golden.row,
            Part::Unordered,
            &golden.unordered,
            &run.unordered,
        )
    })
}

/// The first index where two sequences differ; a shorter one has its end
/// there.
fn first_difference(
    row: &str,
    part: Part,
    golden: &[String],
    run: &[String],
) -> Option<Divergence> {
    let i = (0..golden.len().max(run.len())).find(|&i| golden.get(i) != run.get(i))?;
    Some(Divergence {
        row: row.to_string(),
        part,
        expected: golden.get(i).cloned(),
        got: run.get(i).cloned(),
        after: i.checked_sub(1).and_then(|p| golden.get(p)).cloned(),
    })
}

/// Rows are paired by id, never by position, so a run that skipped or
/// reordered a row is still compared row for row.
pub fn diff_run(golden: &RunFingerprint, run: &RunFingerprint) -> DiffReport {
    let rows: Vec<(String, Option<Divergence>)> = golden
        .rows
        .iter()
        .map(|g| {
            let d = match run.rows.iter().find(|r| r.row == g.row) {
                Some(r) => diff_row(g, r),
                None => Some(Divergence {
                    row: g.row.clone(),
                    part: Part::Missing,
                    expected: None,
                    got: None,
                    after: None,
                }),
            };
            (g.row.clone(), d)
        })
        .collect();
    let first = rows.iter().find_map(|(_, d)| d.clone());
    let not_in_golden = run
        .rows
        .iter()
        .filter(|r| !golden.rows.iter().any(|g| g.row == r.row))
        .map(|r| r.row.clone())
        .collect();
    DiffReport {
        first,
        rows,
        not_in_golden,
    }
}

/// Each golden row with the variance it was recorded under.
pub fn golden_variances(golden: &RunFingerprint) -> Vec<(String, Variance)> {
    golden
        .rows
        .iter()
        .map(|r| (r.row.clone(), r.variance.clone()))
        .collect()
}

/// Fingerprints `run` under the golden's stored variance and diffs it. A
/// golden row with no row JSON in the run is left out of the fingerprint so
/// it reports as [`Part::Missing`] rather than failing the whole diff. Rows
/// the golden does not hold are not fingerprinted, so `not_in_golden` is
/// empty here.
pub fn diff_run_dir(
    golden: &RunFingerprint,
    run: &RunDir,
) -> Result<(DiffReport, RunMeta), String> {
    let present: Vec<(String, Variance)> = golden_variances(golden)
        .into_iter()
        .filter(|(row, _)| run.row_json(&golden.section, row).is_file())
        .collect();
    let (fp, meta) = fingerprint_run_with(run, &golden.section, &present)?;
    Ok((diff_run(golden, &fp), meta))
}

impl Divergence {
    /// One line, no newline, at most 300 characters:
    /// ``row FS-P3: expected `E` after `A`, got `G` ``.
    pub fn line(&self) -> String {
        let row = &self.row;
        let (expected, got) = (quote(self.expected.as_deref()), quote(self.got.as_deref()));
        let line = match self.part {
            Part::Missing => format!("row {row}: in the golden, not in this run"),
            Part::Variance => format!(
                "row {row}: fingerprinted under a different variance than the golden's; \
                 diff it under the golden's variance, or re-bless the golden"
            ),
            Part::Tap => format!("row {row}: expected tap {expected}, got {got}"),
            Part::TapFault => format!("row {row}: tap fault {got}; the diff is unreliable"),
            Part::Result => format!("row {row}: expected result {expected}, got {got}"),
            Part::WindowsOpened | Part::WindowsClosed => {
                let which = if self.part == Part::WindowsOpened {
                    "opened"
                } else {
                    "closed"
                };
                let (expected, got) = (
                    window_set(self.expected.as_deref()),
                    window_set(self.got.as_deref()),
                );
                format!("row {row}: expected windows {which} {expected}, got {got}")
            }
            Part::Event | Part::Clause | Part::Transition | Part::Unordered => {
                let kind = match self.part {
                    Part::Clause => "clause ",
                    Part::Transition => "transition ",
                    Part::Unordered => "unordered ",
                    _ => "",
                };
                let position = match &self.after {
                    Some(a) => format!("after {}", quote(Some(a))),
                    None => "first".to_string(),
                };
                format!("row {row}: expected {kind}{expected} {position}, got {got}")
            }
        };
        cut(&line, MAX_LINE_CHARS)
    }
}

/// `` `v` ``, cut to fit; the end of a sequence reads as words. Newlines
/// would break the one-line promise, so they become spaces.
fn quote(v: Option<&str>) -> String {
    match v {
        Some(v) => format!("`{}`", cut(&v.replace(['\r', '\n'], " "), MAX_VALUE_CHARS)),
        None => "the end of the row".to_string(),
    }
}

/// An empty window set reads as `nothing` rather than an empty quote.
fn window_set(v: Option<&str>) -> String {
    match v {
        Some("") => "nothing".to_string(),
        other => quote(other),
    }
}

/// At most `max` characters, the last three `...` when cut.
fn cut(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max - 3).collect();
    out.push_str("...");
    out
}

#[cfg(test)]
#[path = "diff_tests.rs"]
mod tests;
