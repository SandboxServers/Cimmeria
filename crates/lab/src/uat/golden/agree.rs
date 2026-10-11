//! Whether a set of run fingerprints may become a golden: the owner's rule
//! is that no single run is golden, only five that agree (README D-GD6,
//! option a), each from a complete tap (README D-GD8).

use std::fmt;

use super::build::RunMeta;
use super::diff::{diff_run, Divergence};
use super::event::RunFingerprint;

/// How many runs must agree before their fingerprint becomes a golden: the
/// owner's five-run rule (README D-GD6). A single run can pass by luck.
pub const MIN_AGREEING_RUNS: usize = 5;

/// Run `run` (1-based) of `of` differs from run 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disagreement {
    pub run: usize,
    pub of: usize,
    pub divergence: Divergence,
}

impl Disagreement {
    /// One line: `run 4 of 5 disagrees with run 1: <divergence line>`.
    pub fn line(&self) -> String {
        format!(
            "run {} of {} disagrees with run 1: {}",
            self.run,
            self.of,
            self.divergence.line()
        )
    }
}

/// Why runs cannot become a golden. Run numbers are 1-based, in the order
/// the fingerprints were given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgreeError {
    /// Fewer than [`MIN_AGREEING_RUNS`] fingerprints were given.
    TooFew { have: usize },
    /// Run 1 fingerprinted no rows. A golden with no rows would accept any
    /// run, so it is refused rather than recorded.
    NoRows,
    /// Run `run` was fingerprinted for a different section, or under a
    /// different fingerprint schema, than run 1, so its rows are not
    /// comparable. `field` is `section` or `schema`.
    Mismatch {
        run: usize,
        field: &'static str,
        expected: String,
        got: String,
    },
    /// A tap that dropped, was replaced, did not stop, could not be read,
    /// or had no lab-mcp to bind to: its fingerprint looks deterministic but
    /// may be missing messages (README D-GD8).
    TapUnreliable {
        run: usize,
        row: String,
        reason: String,
    },
    /// With the pass rule on, run `run` row `row` ended `result`, not PASS
    /// (README D-GD6 option a).
    NotPassing {
        run: usize,
        row: String,
        result: String,
    },
    /// A later run's fingerprint differs from run 1's.
    Disagree(Disagreement),
    /// Run `run` fingerprinted a row that run 1 did not reach.
    ExtraRow { run: usize, row: String },
}

impl fmt::Display for AgreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgreeError::TooFew { have } => write!(
                f,
                "{have} runs; a golden needs {MIN_AGREEING_RUNS} that agree"
            ),
            AgreeError::NoRows => f.write_str("run 1 has no rows; a golden needs at least one"),
            AgreeError::Mismatch {
                run,
                field,
                expected,
                got,
            } => write!(
                f,
                "run {run} has {field} `{got}`, run 1 has `{expected}`; the runs are not comparable"
            ),
            AgreeError::TapUnreliable { run, row, reason } => write!(
                f,
                "run {run} row {row}: tap {reason}; a golden needs a complete tap"
            ),
            AgreeError::NotPassing { run, row, result } => {
                write!(f, "run {run} row {row} is {result}, not PASS")
            }
            AgreeError::Disagree(d) => f.write_str(&d.line()),
            AgreeError::ExtraRow { run, row } => {
                write!(f, "run {run} has row {row}, which run 1 does not")
            }
        }
    }
}

/// The golden fingerprint when at least five runs agree. Taps are checked
/// before results so a run whose failure came with a broken tap is reported
/// for the tap, the thing a re-run fixes.
pub fn agree(fps: &[RunFingerprint], require_pass: bool) -> Result<RunFingerprint, AgreeError> {
    if fps.len() < MIN_AGREEING_RUNS {
        return Err(AgreeError::TooFew { have: fps.len() });
    }
    if fps[0].rows.is_empty() {
        return Err(AgreeError::NoRows);
    }
    for (k, fp) in fps.iter().enumerate().skip(1) {
        let fields = [
            ("section", fps[0].section.clone(), fp.section.clone()),
            ("schema", fps[0].schema.to_string(), fp.schema.to_string()),
        ];
        for (field, expected, got) in fields {
            if expected != got {
                return Err(AgreeError::Mismatch {
                    run: k + 1,
                    field,
                    expected,
                    got,
                });
            }
        }
    }
    for (k, fp) in fps.iter().enumerate() {
        for row in &fp.rows {
            let reason = if !row.tap_faults.is_empty() {
                Some(row.tap_faults.join(", "))
            } else if row.tap == "unavailable" {
                Some("unavailable".to_string())
            } else {
                None
            };
            if let Some(reason) = reason {
                return Err(AgreeError::TapUnreliable {
                    run: k + 1,
                    row: row.row.clone(),
                    reason,
                });
            }
        }
    }
    if require_pass {
        for (k, fp) in fps.iter().enumerate() {
            if let Some(row) = fp.rows.iter().find(|r| r.result != "PASS") {
                return Err(AgreeError::NotPassing {
                    run: k + 1,
                    row: row.row.clone(),
                    result: row.result.clone(),
                });
            }
        }
    }
    let golden = &fps[0];
    for (k, fp) in fps.iter().enumerate().skip(1) {
        let report = diff_run(golden, fp);
        if let Some(divergence) = report.first {
            return Err(AgreeError::Disagree(Disagreement {
                run: k + 1,
                of: fps.len(),
                divergence,
            }));
        }
        // A row only a later run reached disagrees too: otherwise run 1
        // alone would decide which rows the golden holds.
        if let Some(row) = report.not_in_golden.into_iter().next() {
            return Err(AgreeError::ExtraRow { run: k + 1, row });
        }
    }
    Ok(golden.clone())
}

/// The runs are distinct and drove the same spec, server build and SGW.exe.
/// An empty value counts as missing: it identifies nothing, so two runs
/// that both recorded `""` prove nothing about agreeing.
pub fn check_runs(metas: &[RunMeta]) -> Result<(), String> {
    for (i, a) in metas.iter().enumerate() {
        if let Some(b) = metas[i + 1..].iter().find(|b| b.run_id == a.run_id) {
            return Err(format!(
                "run id `{}` appears twice: `{}` and `{}`",
                a.run_id, a.run_dir, b.run_dir
            ));
        }
    }
    let Some(first) = metas.first() else {
        return Ok(());
    };
    let fields: [(&str, fn(&RunMeta) -> Option<&str>); 3] = [
        ("spec_sha256", |m| present(&m.spec_sha256)),
        ("server_build", |m| present(&m.server_build)),
        ("sgw_sha256", |m| present(&m.sgw_sha256)),
    ];
    for (name, get) in fields {
        let Some(want) = get(first) else {
            return Err(format!("{name} is missing in run {}", first.run_id));
        };
        for m in metas {
            match get(m) {
                None => return Err(format!("{name} is missing in run {}", m.run_id)),
                Some(have) if have != want => {
                    return Err(format!(
                        "{name} differs: `{want}` in run {} and `{have}` in run {}",
                        first.run_id, m.run_id
                    ))
                }
                Some(_) => {}
            }
        }
    }
    Ok(())
}

/// The value, unless it is absent or empty.
fn present(v: &Option<String>) -> Option<&str> {
    v.as_deref().filter(|s| !s.is_empty())
}

#[cfg(test)]
#[path = "agree_tests.rs"]
mod tests;
