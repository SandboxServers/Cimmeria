//! CPU stressor for lab chaos mode: busy threads that take a fixed share of
//! every slice, so frame timing in the client drifts under load.

use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// The scheduling unit for duty cycling. Short enough that a stop request
/// is noticed within a frame, long enough that `sleep` overhead stays small.
const SLICE: Duration = Duration::from_millis(10);

/// A CPU load: `cores` busy threads, each spinning for `duty_percent` of
/// every slice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuStress {
    /// Busy threads to start. `start` clamps it to `1..=available`.
    pub cores: usize,
    /// Busy share of each slice, 1 to 100. `start` clamps it to that range.
    pub duty_percent: u8,
}

/// Owns the stressor threads. Dropping the guard stops and joins them, so a
/// chaos run that panics or returns early cannot leave the machine loaded.
pub struct CpuGuard {
    stop: Arc<AtomicBool>,
    handles: Vec<JoinHandle<()>>,
}

impl CpuStress {
    /// `"max"` and `"max-N"` resolve against the machine's parallelism, so one
    /// profile works on laptops and CI runners alike. `cores` is clamped to
    /// `1..=available`; `"0"` and garbage are refused rather than guessed.
    pub fn from_knobs(cores: &str, duty: &str) -> Result<CpuStress, String> {
        let available = available_cores();
        Ok(CpuStress {
            cores: parse_cores(cores, available)?,
            duty_percent: parse_duty(duty)?,
        })
    }

    /// Spawns the threads and returns the guard that owns them. A thread the
    /// OS refuses to create is logged and skipped, not fatal: a partial load
    /// is still a useful chaos run.
    pub fn start(&self) -> CpuGuard {
        // The fields are public, so a hand-built value can be out of range;
        // a duty over 100 would underflow the idle time and kill every thread.
        let cores = self.cores.clamp(1, available_cores());
        let duty = self.duty_percent.clamp(1, 100);
        let stop = Arc::new(AtomicBool::new(false));
        let handles = (0..cores)
            .filter_map(|n| {
                let stop = Arc::clone(&stop);
                let spawned = thread::Builder::new()
                    .name(format!("lab-chaos-cpu-{n}"))
                    .spawn(move || burn(&stop, duty));
                match spawned {
                    Ok(handle) => Some(handle),
                    Err(e) => {
                        tracing::warn!(thread = n, error = %e, "cpu chaos thread not started");
                        None
                    }
                }
            })
            .collect();
        CpuGuard { stop, handles }
    }
}

impl CpuGuard {
    /// Threads that have been started and have not yet exited.
    pub fn running(&self) -> usize {
        self.handles.iter().filter(|h| !h.is_finished()).count()
    }

    /// Stops and joins every thread. Also called by `Drop`. Safe to call
    /// twice: the second call finds no handles left to join.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        for handle in self.handles.drain(..) {
            // A panicked thread has nothing to clean up, but the run had
            // less load than it asked for, so say so.
            if handle.join().is_err() {
                tracing::warn!("cpu chaos thread panicked; the run had less load than asked");
            }
        }
    }
}

impl Drop for CpuGuard {
    fn drop(&mut self) {
        self.stop()
    }
}

/// Spins for the busy share of each slice, then sleeps the rest. The stop
/// flag is read once per slice, so stopping takes at most one slice. At 100%
/// there is no sleep at all.
fn burn(stop: &AtomicBool, duty_percent: u8) {
    let busy = SLICE * u32::from(duty_percent) / 100;
    let idle = SLICE - busy;
    while !stop.load(Ordering::Relaxed) {
        let slice_end = Instant::now() + busy;
        while Instant::now() < slice_end {
            std::hint::spin_loop();
        }
        if !idle.is_zero() {
            thread::sleep(idle);
        }
    }
}

fn available_cores() -> usize {
    thread::available_parallelism().map_or(1, NonZeroUsize::get)
}

fn parse_cores(raw: &str, available: usize) -> Result<usize, String> {
    let raw = raw.trim();
    let wanted = if raw == "max" {
        available
    } else if let Some(reserve) = raw.strip_prefix("max-") {
        let reserve: usize = reserve
            .parse()
            .map_err(|_| format!("cores `{raw}`: expected max-N with a whole number"))?;
        available.saturating_sub(reserve)
    } else if raw.starts_with('+') {
        return Err(format!(
            "cores `{raw}`: expected max, max-N or a positive integer"
        ));
    } else {
        let n: usize = raw
            .parse()
            .map_err(|_| format!("cores `{raw}`: expected max, max-N or a positive integer"))?;
        if n == 0 {
            return Err("cores `0`: at least one core is required".to_string());
        }
        n
    };
    Ok(wanted.clamp(1, available))
}

fn parse_duty(raw: &str) -> Result<u8, String> {
    let raw = raw.trim();
    let percent: u8 = raw
        .parse()
        .map_err(|_| format!("duty `{raw}`: expected a percent from 1 to 100"))?;
    if (1..=100).contains(&percent) {
        Ok(percent)
    } else {
        Err(format!("duty `{raw}`: expected a percent from 1 to 100"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `max-2` resolves to two fewer than the machine has, `max-999` clamps
    /// up to one core, and `0` is refused. The expected count comes from
    /// `available_parallelism` so the test holds on any CI runner.
    #[test]
    fn max_minus_two_resolves_and_clamps() {
        let available = available_cores();
        let stress = CpuStress::from_knobs("max-2", "10").unwrap();
        assert_eq!(
            stress.cores,
            available.saturating_sub(2).clamp(1, available)
        );
        assert_eq!(stress.duty_percent, 10);

        let clamped = CpuStress::from_knobs("max-999", "10").unwrap();
        assert_eq!(clamped.cores, 1);

        assert!(CpuStress::from_knobs("0", "10").is_err());
    }

    /// Dropping the guard must join every thread. The probe is a second
    /// handle to the guard's stop flag, which each thread holds a clone of;
    /// once the threads have exited their clones are gone and only the probe
    /// is left. If `Drop` did not stop them, the count would stay above one.
    #[test]
    fn guard_stops_every_thread_on_drop() {
        let stress = CpuStress {
            cores: 2,
            duty_percent: 10,
        };
        let guard = stress.start();
        let probe = Arc::clone(&guard.stop);
        assert_eq!(guard.running(), 2);

        drop(guard);
        assert_eq!(Arc::strong_count(&probe), 1);
    }

    /// The first `stop` joins the threads (their clones of the stop flag are
    /// gone, which a detach without a join would not guarantee), and a
    /// second `stop` is a no-op.
    #[test]
    fn stop_is_idempotent() {
        let mut guard = CpuStress {
            cores: 1,
            duty_percent: 10,
        }
        .start();
        let probe = Arc::clone(&guard.stop);
        guard.stop();
        assert_eq!(Arc::strong_count(&probe), 2, "guard + probe only");
        guard.stop();
        assert_eq!(guard.running(), 0);
    }

    /// A hand-built duty over 100 is clamped, not left to underflow the
    /// idle time and panic every thread.
    #[test]
    fn out_of_range_fields_are_clamped_at_start() {
        let guard = CpuStress {
            cores: 1,
            duty_percent: 255,
        }
        .start();
        assert_eq!(guard.running(), 1);
    }

    #[test]
    fn bad_knob_text_is_refused() {
        for duty in ["0", "101", "256", "-1", "50%", ""] {
            assert!(CpuStress::from_knobs("1", duty).is_err(), "duty {duty:?}");
        }
        assert_eq!(CpuStress::from_knobs("1", " 10 ").unwrap().duty_percent, 10);
        for cores in ["+3", "max- 2", "-1", "lots", ""] {
            assert!(
                CpuStress::from_knobs(cores, "10").is_err(),
                "cores {cores:?}"
            );
        }
        assert_eq!(
            CpuStress::from_knobs("max", "10").unwrap().cores,
            available_cores()
        );
    }
}
