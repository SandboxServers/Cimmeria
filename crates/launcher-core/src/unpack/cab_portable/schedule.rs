//! Spread independent folders over worker threads. A folder is one stream
//! (its MSZIP history runs through all its blocks), so a folder is the unit
//! of work and never splits across threads.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use super::super::UnpackError;

/// Assign folder indices to `workers` lanes. Folders go largest first (ties
/// in plan order), each to the lane with the fewest bytes so far (ties to the
/// lower lane). Every lane lists its folders in plan order. `sizes[i]` is the
/// compressed size of folder `i`; `workers` must be at least 1.
pub(super) fn assign(sizes: &[u64], workers: usize) -> Vec<Vec<usize>> {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|&a, &b| sizes[b].cmp(&sizes[a]).then(a.cmp(&b)));
    let mut lanes = vec![Vec::new(); workers];
    let mut load = vec![0u64; workers];
    for index in order {
        let lane = (0..workers)
            .min_by_key(|&w| (load[w], w))
            .expect("at least one worker");
        load[lane] += sizes[index];
        lanes[lane].push(index);
    }
    for lane in &mut lanes {
        lane.sort_unstable();
    }
    lanes
}

/// What the workers share: a stop flag and the first error, which wins.
pub(super) struct Stop {
    failed: AtomicBool,
    first: Mutex<Option<UnpackError>>,
}

impl Stop {
    /// `Err(Cancelled)` once another worker has failed, so this one stops.
    pub fn check(&self) -> Result<(), UnpackError> {
        if self.failed.load(Ordering::Acquire) {
            Err(UnpackError::Cancelled)
        } else {
            Ok(())
        }
    }

    fn fail(&self, error: UnpackError) {
        let mut first = self.first.lock().unwrap_or_else(PoisonError::into_inner);
        if first.is_none() {
            *first = Some(error);
        }
        self.failed.store(true, Ordering::Release);
    }
}

/// Run `expand` for every folder index in `lanes`, one thread per lane. Each
/// lane stops at its first error and every other lane stops before its next
/// block. The first error recorded is returned once all threads have ended.
pub(super) fn run<F>(lanes: &[Vec<usize>], expand: F) -> Result<(), UnpackError>
where
    F: Fn(usize, &Stop) -> Result<(), UnpackError> + Sync,
{
    let stop = Stop {
        failed: AtomicBool::new(false),
        first: Mutex::new(None),
    };
    std::thread::scope(|scope| {
        for lane in lanes {
            let (stop, expand) = (&stop, &expand);
            scope.spawn(move || {
                for &folder in lane {
                    if let Err(error) = stop.check().and_then(|()| expand(folder, stop)) {
                        stop.fail(error);
                        return;
                    }
                }
            });
        }
    });
    match stop
        .first
        .into_inner()
        .unwrap_or_else(PoisonError::into_inner)
    {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
