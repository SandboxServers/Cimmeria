//! Whole-process activity of the client, read from outside it: bytes read
//! and CPU time of every thread. The load grace ([`super::stall_grace`])
//! uses it for a stall the main thread spends waiting on the disk: a client
//! starting while the drive is saturated by builds sits in page-in waits at
//! 0 ms of main-thread CPU for minutes (2026-10-12, `docs/analysis/haiku-lab-uat`),
//! which the main-thread CPU test alone reads as a hang.

use super::stall_grace::main_thread_busy;

/// Bytes read per sample interval at or above which the process counts as
/// making IO progress (256 KiB: far above a hung client's idle reads).
pub const IO_PROGRESS_BYTES: i64 = 256 * 1024;

/// What the process did between two samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Activity {
    /// Bytes read (all threads) since the previous sample.
    pub read_bytes: i64,
    /// Kernel + user CPU time of all threads since the previous sample, in ms.
    pub cpu_ms: i64,
    /// Wall clock since the previous sample, in ms.
    pub wall_ms: i64,
}

/// Whether the interval shows the process working or reading: at least
/// [`IO_PROGRESS_BYTES`] read, or the busy share of CPU
/// ([`main_thread_busy`]'s threshold, applied to all threads).
pub fn io_progress(a: Activity) -> bool {
    a.wall_ms > 0 && (a.read_bytes >= IO_PROGRESS_BYTES || main_thread_busy(a.cpu_ms, a.wall_ms))
}

/// The sampler for one launch: remembers the previous counters.
#[derive(Debug, Default)]
pub struct ProcessActivity {
    /// `(read_bytes, cpu_ms, wall_ms)` of the previous sample.
    prev: Option<(i64, i64, i64)>,
}

impl ProcessActivity {
    /// Sample `pid` at `now_ms`. `None` on the first sample or when the
    /// process cannot be read (the caller then treats the stall as no
    /// progress, the behaviour before this existed).
    pub fn sample(&mut self, pid: u32, now_ms: i64) -> Option<Activity> {
        let Some((read, cpu)) = read_counters(pid) else {
            self.prev = None;
            return None;
        };
        let out = self.prev.map(|(prev_read, prev_cpu, prev_wall)| Activity {
            read_bytes: read - prev_read,
            cpu_ms: cpu - prev_cpu,
            wall_ms: now_ms - prev_wall,
        });
        self.prev = Some((read, cpu, now_ms));
        out
    }
}

#[cfg(not(windows))]
fn read_counters(_pid: u32) -> Option<(i64, i64)> {
    None
}

/// `(bytes read, cpu ms)` of the whole process.
#[cfg(windows)]
fn read_counters(pid: u32) -> Option<(i64, i64)> {
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE, FILETIME};
    use windows_sys::Win32::System::Threading::{
        GetProcessIoCounters, GetProcessTimes, OpenProcess, IO_COUNTERS, PROCESS_QUERY_INFORMATION,
    };

    let ticks = |ft: &FILETIME| (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime);
    // SAFETY: OpenProcess/GetProcessIoCounters/GetProcessTimes/CloseHandle
    // with valid out-params; the handle is closed on every path.
    unsafe {
        let h = OpenProcess(PROCESS_QUERY_INFORMATION, FALSE, pid);
        if h.is_null() {
            return None;
        }
        let mut io: IO_COUNTERS = std::mem::zeroed();
        let zero = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
        let io_ok = GetProcessIoCounters(h, &mut io) != 0;
        let times_ok = GetProcessTimes(h, &mut created, &mut exited, &mut kernel, &mut user) != 0;
        CloseHandle(h);
        (io_ok && times_ok).then(|| {
            (
                io.ReadTransferCount as i64,
                ((ticks(&kernel) + ticks(&user)) / 10_000) as i64,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 256 KiB read in the interval is progress with an idle CPU; 2 % CPU
    /// is progress with no reads; a hung client's trickle is neither.
    #[test]
    fn progress_is_reads_or_cpu_not_a_trickle() {
        let a = |read_bytes, cpu_ms| Activity {
            read_bytes,
            cpu_ms,
            wall_ms: 6_000,
        };
        assert!(io_progress(a(IO_PROGRESS_BYTES, 0)));
        assert!(io_progress(a(0, 120)));
        assert!(!io_progress(a(IO_PROGRESS_BYTES - 1, 119)));
        assert!(!io_progress(a(0, 0)));
        assert!(!io_progress(Activity {
            read_bytes: 10 << 20,
            cpu_ms: 1_000,
            wall_ms: 0
        }));
    }

    /// The sampler reads this very process: the first sample has no
    /// interval, the second does. Windows only (a no-op elsewhere).
    #[cfg(windows)]
    #[test]
    fn the_sampler_reads_a_live_process() {
        let pid = std::process::id();
        let mut s = ProcessActivity::default();
        assert_eq!(s.sample(pid, 0), None);
        let a = s.sample(pid, 1_000).expect("own process is readable");
        assert_eq!(a.wall_ms, 1_000);
        assert!(a.read_bytes >= 0 && a.cpu_ms >= 0);
    }

    #[test]
    fn a_missing_process_has_no_sample() {
        let mut s = ProcessActivity::default();
        assert_eq!(s.sample(0xFFFF_FFF1, 0), None);
        assert_eq!(s.sample(0xFFFF_FFF1, 1_000), None);
    }
}
