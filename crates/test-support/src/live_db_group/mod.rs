//! The fail-closed guards behind the live-DB tier's per-slot databases.
//!
//! `tools/test-live-db.sh` runs the lib tests of every live-DB crate under
//! nextest's `ci-live-db` profile. That profile puts only the tests whose
//! name contains [`MARKER`] (`live_db`) in the `live-db` test group, whose
//! tests each get their slot's own database clone ([`crate::database_url`]);
//! every other test runs in parallel beside them. A live-DB test whose name
//! missed the marker would get no slot and run against the shared template
//! database, colliding on shared sentinel rows, so this guard fails when any
//! test that reaches the [`require_db_or_skip!`](crate::require_db_or_skip)
//! gate is outside the filter. [`url_guard`] fails on test code that
//! resolves the database URL around the slot.
//!
//! A test reaches the gate when its body calls it, or calls a non-test
//! helper whose body does (transitively). Its nextest name is its module
//! path plus its fn name, so `live_db` can sit in either: a `live_db_*`
//! fn, or any fn inside a `*live_db*` module.
//!
//! The scan uses a small lexer ([`lex`]) and follows `mod x;` declarations
//! from each crate's `src/lib.rs` ([`module_tree`]); the tier runs `--lib`
//! only. See TESTING.md, "Live-DB tests".

mod items;
mod lex;
mod module_tree;
mod url_guard;

#[cfg(test)]
mod tests;

use std::collections::BTreeSet;
use std::path::PathBuf;

use module_tree::{CrateScan, LibFn};

/// The substring that puts a test in the serial `live-db` group.
pub(crate) const MARKER: &str = "live_db";

/// One test (or gate call) outside the serial filter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Violation {
    pub(crate) file: PathBuf,
    pub(crate) line: usize,
    /// The nextest name, or `None` for a gate the guard cannot attribute.
    pub(crate) test_name: Option<String>,
    pub(crate) why: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.file.display(), self.line)?;
        if let Some(name) = &self.test_name {
            write!(f, "  {name}")?;
        }
        write!(f, "  ({})", self.why)
    }
}

/// The live-DB tests found and the violations, for one crate's lib target.
#[derive(Debug, Default)]
pub(crate) struct Report {
    pub(crate) db_tests: Vec<String>,
    pub(crate) violations: Vec<Violation>,
}

/// Apply the rule to the fns of one lib target.
pub(crate) fn check(fns: &[LibFn], stray_gates: &[(PathBuf, usize)]) -> Report {
    let mut report = Report::default();
    for (file, line) in stray_gates {
        report.violations.push(Violation {
            file: file.clone(),
            line: *line,
            test_name: None,
            why: "gate call outside a fn body (a macro_rules! expansion?); \
                  the guard cannot name the test it lands in"
                .into(),
        });
    }

    // Close over helpers: a non-test fn that reaches the gate taints every
    // fn that mentions its name.
    let mut reaches: Vec<Option<String>> = fns
        .iter()
        .map(|f| (!f.item.gate_lines.is_empty()).then(|| "calls the gate".to_string()))
        .collect();
    let mut queue: Vec<usize> = (0..fns.len()).filter(|&i| reaches[i].is_some()).collect();
    let mut seen_helpers = BTreeSet::new();
    while let Some(i) = queue.pop() {
        let helper = &fns[i];
        if helper.item.is_test || !seen_helpers.insert(helper.item.name.clone()) {
            continue;
        }
        for (j, caller) in fns.iter().enumerate() {
            if j != i && reaches[j].is_none() && caller.item.idents.contains(&helper.item.name) {
                reaches[j] = Some(format!(
                    "calls `{}`, which reaches the gate",
                    helper.item.name
                ));
                queue.push(j);
            }
        }
    }

    for (f, why) in fns.iter().zip(&reaches) {
        let (true, Some(why)) = (f.item.is_test, why) else {
            continue;
        };
        report.db_tests.push(f.test_name.clone());
        if !f.test_name.contains(MARKER) {
            report.violations.push(Violation {
                file: f.file.clone(),
                line: f.item.line,
                test_name: Some(f.test_name.clone()),
                why: format!("{why}, but the name has no `{MARKER}`"),
            });
        }
    }
    report
}

/// Scan every crate under `crates/` with a `src/lib.rs`.
pub(crate) fn check_workspace() -> (Report, Vec<CrateScan>) {
    let mut report = Report::default();
    let mut scans = Vec::new();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(crate::source_scan::crates_dir())
        .expect("read crates/")
        .flatten()
        .map(|e| e.path())
        .collect();
    dirs.sort();
    for dir in dirs {
        let lib = dir.join("src").join("lib.rs");
        if !lib.is_file() {
            continue;
        }
        let scan = module_tree::scan_lib(&lib);
        let r = check(&scan.fns, &scan.stray_gates);
        report.db_tests.extend(r.db_tests);
        report.violations.extend(r.violations);
        scans.push(scan);
    }
    (report, scans)
}

/// Gate-call lines by a line-based text search, independent of the lexer:
/// `require_db_or_skip!(` on a line that is not a `//` comment.
pub(crate) fn naive_gate_lines(src: &str) -> Vec<usize> {
    src.lines()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim_start();
            !t.starts_with("//") && t.contains(concat!("require_db_or_skip", "!("))
        })
        .map(|(i, _)| i + 1)
        .collect()
}
