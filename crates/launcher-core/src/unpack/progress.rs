//! How an unpack step becomes a progress event (C-7). Cabinet expansion is
//! its own phase; every other step is extraction.
use std::path::Path;

use super::UnpackSink;
use crate::install::Progress;
use crate::install_progress::ProgressReporter;

/// The step `cab_portable` and `fdi` report while they expand cabinets.
pub(super) const CABINET_STEP: &str = "expanding cabinets";

/// How many listed RAR entries pass between two index reports.
const INDEX_EVERY: usize = 256;

impl UnpackSink {
    pub(super) fn report(&self, step: &str, current: usize, total: usize, path: &Path) {
        let label = format!("{} ({step})", self.label);
        let filename = path.display().to_string();
        let event = if step == CABINET_STEP {
            Progress::Expanding {
                label,
                current,
                total,
                filename,
            }
        } else {
            Progress::Extracting {
                label,
                current,
                total,
                filename,
            }
        };
        self.progress.report(event);
    }

    /// The RAR listing pass: `Checking { read: entries, total: 0 }` every 256
    /// entries (no known total, so the UI shows no percent).
    pub(super) fn report_index(&self, entries: usize) {
        if entries.is_multiple_of(INDEX_EVERY) {
            self.progress.report(Progress::Checking {
                read: entries as u64,
                total: 0,
            });
        }
    }
}

#[cfg(test)]
#[path = "progress_tests.rs"]
mod tests;
