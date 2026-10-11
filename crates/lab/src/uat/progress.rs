//! The running UAT row, for `GET /status` (LW-01, D-LW1): the cell that
//! `lab watch` reads to show which row an instance is on. It carries no
//! lease data; the lease stays in the lease book. The guard clears the cell
//! on every exit path of the run (a return, an error, a lost lease), so a
//! stale "running" row never stays on `/status`.

use std::sync::{Arc, Mutex, MutexGuard};

use serde_json::Value;

use super::evidence::RowEvidence;
use super::invoke::ToolInvoker;
use super::runner::{now_ms, utc_of, Runner};

/// One instance's running `lab_uat_run`, as `/status` shows it (D-LW1).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RunProgress {
    pub run_id: String,
    pub run_dir: String,
    pub started_at: String,
    pub rows_total: usize,
    pub rows_done: usize,
    pub section: Option<String>,
    pub current_row: Option<String>,
    pub row_index: Option<usize>,
    pub row_started_at: Option<String>,
    pub last_row: Option<String>,
    pub last_result: Option<String>,
}

/// Shared between an instance's `Supervisor` (read by `/status`) and the
/// runner of that instance's current `lab_uat_run` (writes).
#[derive(Debug, Clone, Default)]
pub struct ProgressCell(Arc<Mutex<Option<RunProgress>>>);

impl ProgressCell {
    /// Starts a run's progress; the guard clears it when dropped.
    pub fn begin(
        &self,
        run_id: &str,
        run_dir: &str,
        rows_total: usize,
        now_ms: i64,
    ) -> ProgressGuard {
        *self.slot() = Some(RunProgress {
            run_id: run_id.into(),
            run_dir: run_dir.into(),
            started_at: utc_of(now_ms),
            rows_total,
            rows_done: 0,
            section: None,
            current_row: None,
            row_index: None,
            row_started_at: None,
            last_row: None,
            last_result: None,
        });
        ProgressGuard { cell: self.clone() }
    }

    /// Sets section, current_row, row_index (= rows_done + 1) and row_started_at.
    pub fn row_started(&self, section: &str, row: &str, now_ms: i64) {
        if let Some(p) = self.slot().as_mut() {
            p.section = Some(section.into());
            p.current_row = Some(row.into());
            p.row_index = Some(p.rows_done + 1);
            p.row_started_at = Some(utc_of(now_ms));
        }
    }

    /// rows_done += 1; last_row/last_result set; current_row, row_index, row_started_at cleared.
    pub fn row_finished(&self, row: &str, result: &str) {
        if let Some(p) = self.slot().as_mut() {
            p.rows_done += 1;
            p.last_row = Some(row.into());
            p.last_result = Some(result.into());
            p.current_row = None;
            p.row_index = None;
            p.row_started_at = None;
        }
    }

    pub fn snapshot(&self) -> Option<RunProgress> {
        self.slot().clone()
    }

    /// `serde_json::to_value(snapshot)`, or `Value::Null` when idle.
    pub fn to_json(&self) -> Value {
        self.snapshot()
            .and_then(|p| serde_json::to_value(p).ok())
            .unwrap_or(Value::Null)
    }

    /// A poisoned lock still holds the last progress written: keep reading it.
    fn slot(&self) -> MutexGuard<'_, Option<RunProgress>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Clears the cell on drop, so an early return, an error or a lost lease
/// never leaves a stale "running" row on `/status`.
#[must_use]
pub struct ProgressGuard {
    cell: ProgressCell,
}

impl Drop for ProgressGuard {
    fn drop(&mut self) {
        *self.cell.slot() = None;
    }
}

impl<'a, I: ToolInvoker> Runner<'a, I> {
    /// Show this run's progress on `/status` through `cell`.
    pub fn with_progress(mut self, cell: ProgressCell) -> Self {
        self.progress = Some(cell);
        self
    }

    /// None for a plan_only run or when no cell was given: a plan drives
    /// nothing, so `/status` must not show it as running (D-LW1).
    pub(crate) fn begin_progress(&self) -> Option<ProgressGuard> {
        if self.req.plan_only {
            return None;
        }
        let cell = self.progress.as_ref()?;
        let run_dir = self.run.root.display().to_string();
        Some(cell.begin(
            &self.manifest.run_id,
            &run_dir,
            self.rows_to_run(),
            now_ms(),
        ))
    }

    /// The rows `run_all` will run: every row of every section that passes
    /// the same `rows` filter.
    fn rows_to_run(&self) -> usize {
        self.req
            .sections
            .iter()
            .flat_map(|s| &s.spec.rows)
            .filter(|r| match &self.req.rows {
                Some(only) => only.contains(&r.id),
                None => true,
            })
            .count()
    }

    pub(crate) fn progress_row_started(&self, section: &str, row: &str) {
        if let Some(cell) = &self.progress {
            cell.row_started(section, row, now_ms());
        }
    }

    pub(crate) fn progress_row_finished(&self, ev: &RowEvidence) {
        if let Some(cell) = &self.progress {
            cell.row_finished(&ev.row_id, ev.result.as_str());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The guard clears the cell on drop, and `row_index` is always
    /// `rows_done + 1`. Fails if the guard does not clear or the index drifts.
    #[test]
    fn begin_then_rows_report_the_current_row_and_clear_on_drop() {
        let cell = ProgressCell::default();
        let g = cell.begin("r1", "C:\\runs\\r1", 2, 1_000);
        cell.row_started("s", "A", 2_000);
        let p = cell.snapshot().unwrap();
        assert_eq!(p.current_row.as_deref(), Some("A"));
        assert_eq!(p.row_index, Some(1));
        assert_eq!(p.rows_done, 0);

        cell.row_finished("A", "PASS");
        let p = cell.snapshot().unwrap();
        assert_eq!(p.current_row, None);
        assert_eq!(p.rows_done, 1);
        assert_eq!(p.last_row.as_deref(), Some("A"));
        assert_eq!(p.last_result.as_deref(), Some("PASS"));

        cell.row_started("s", "B", 3_000);
        assert_eq!(cell.snapshot().unwrap().row_index, Some(2));

        drop(g);
        assert_eq!(cell.snapshot(), None);
        assert_eq!(cell.to_json(), Value::Null);
    }

    /// Row updates before any `begin` are ignored.
    #[test]
    fn an_idle_cell_ignores_row_updates() {
        let cell = ProgressCell::default();
        cell.row_started("s", "A", 1_000);
        cell.row_finished("A", "PASS");
        assert_eq!(cell.snapshot(), None);
    }
}
