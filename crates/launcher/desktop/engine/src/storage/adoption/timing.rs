//! File workload accompanies a completed full-tree scan; partial scans fail closed.
use super::*;
use crate::phase_timing::{Context, Phase};

pub(super) fn scan(
    context: Context,
    phase: Phase,
    work: impl FnOnce() -> Result<inventory::Index, Error>,
) -> Result<inventory::Index, Error> {
    let mut timer = context.start(phase);
    let result = work();
    if let Ok(index) = &result {
        timer.workload(
            index.len() as u64,
            index
                .values()
                .fold(0u64, |sum, e| sum.saturating_add(e.size)),
        );
    }
    if matches!(result, Err(Error::Cancelled)) {
        timer.finish(crate::phase_timing::Outcome::Cancelled);
        result
    } else {
        timer.result(result)
    }
}
