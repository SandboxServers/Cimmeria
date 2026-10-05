//! CPU time of the client's main thread, read from outside the process:
//! the one signal that still reports while the main thread is blocked and
//! every bridge call times out ([`super::stall_grace`]).
//!
//! The main thread runs `FEngineLoop::Tick` and is the process's first
//! thread, so it is the one with the earliest creation time. The injection
//! thread and every engine worker are created after it.

/// The CPU time sampler for one launch: finds the main thread on first
/// use and remembers the previous sample to report the interval.
#[derive(Debug, Default)]
pub struct MainThreadCpu {
    tid: Option<u32>,
    /// `(cpu_ms, wall_ms)` of the previous sample.
    prev: Option<(i64, i64)>,
}

impl MainThreadCpu {
    /// Sample the main thread of `pid` at `now_ms`. Returns
    /// `(cpu_ms, wall_ms)` spent since the previous sample, or `None` on
    /// the first sample or when the thread can't be read (the caller then
    /// treats the stall as idle, the pre-grace behaviour).
    pub fn sample(&mut self, pid: u32, now_ms: i64) -> Option<(i64, i64)> {
        if self.tid.is_none() {
            self.tid = find_main_thread(pid);
        }
        let Some(cpu) = self.tid.and_then(thread_cpu_ms) else {
            self.prev = None;
            return None;
        };
        let delta = self
            .prev
            .map(|(prev_cpu, prev_wall)| (cpu - prev_cpu, now_ms - prev_wall));
        self.prev = Some((cpu, now_ms));
        delta
    }
}

#[cfg(windows)]
use win::{find_main_thread, thread_cpu_ms};

#[cfg(not(windows))]
fn find_main_thread(_pid: u32) -> Option<u32> {
    None
}

#[cfg(not(windows))]
fn thread_cpu_ms(_tid: u32) -> Option<i64> {
    None
}

#[cfg(windows)]
mod win {
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE, FILETIME, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, TH32CS_SNAPTHREAD, THREADENTRY32,
    };
    use windows_sys::Win32::System::Threading::{
        GetThreadTimes, OpenThread, THREAD_QUERY_LIMITED_INFORMATION,
    };

    fn filetime_u64(ft: &FILETIME) -> u64 {
        (u64::from(ft.dwHighDateTime) << 32) | u64::from(ft.dwLowDateTime)
    }

    /// `(creation, kernel + user)` of a thread, in 100 ns units.
    fn thread_times(tid: u32) -> Option<(u64, u64)> {
        // SAFETY: OpenThread/GetThreadTimes/CloseHandle with valid
        // out-params; the handle is closed on every path.
        unsafe {
            let h = OpenThread(THREAD_QUERY_LIMITED_INFORMATION, FALSE, tid);
            if h.is_null() {
                return None;
            }
            let zero = FILETIME {
                dwLowDateTime: 0,
                dwHighDateTime: 0,
            };
            let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
            let ok = GetThreadTimes(h, &mut created, &mut exited, &mut kernel, &mut user);
            CloseHandle(h);
            (ok != 0).then(|| {
                (
                    filetime_u64(&created),
                    filetime_u64(&kernel) + filetime_u64(&user),
                )
            })
        }
    }

    /// The thread of `pid` created first: the main thread.
    pub fn find_main_thread(pid: u32) -> Option<u32> {
        let mut best: Option<(u64, u32)> = None;
        // SAFETY: a thread snapshot walked with a correctly sized entry;
        // the snapshot handle is closed below.
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            if snap == INVALID_HANDLE_VALUE {
                return None;
            }
            let mut entry: THREADENTRY32 = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
            let mut more = Thread32First(snap, &mut entry) != 0;
            while more {
                if entry.th32OwnerProcessID == pid {
                    let tid = entry.th32ThreadID;
                    if let Some((created, _)) = thread_times(tid) {
                        if best.is_none_or(|(c, _)| created < c) {
                            best = Some((created, tid));
                        }
                    }
                }
                more = Thread32Next(snap, &mut entry) != 0;
            }
            CloseHandle(snap);
        }
        best.map(|(_, tid)| tid)
    }

    /// Kernel + user CPU time of a thread, in ms.
    pub fn thread_cpu_ms(tid: u32) -> Option<i64> {
        thread_times(tid).map(|(_, cpu)| (cpu / 10_000) as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This test's own process: its main thread exists, and a busy loop
    /// on this thread shows up as CPU time. Windows only (the sampler is
    /// a no-op elsewhere).
    #[cfg(windows)]
    #[test]
    fn the_sampler_reads_a_live_main_thread() {
        let pid = std::process::id();
        let tid = find_main_thread(pid).expect("a running process has a main thread");
        assert!(thread_cpu_ms(tid).is_some());

        let mut s = MainThreadCpu::default();
        assert_eq!(s.sample(pid, 0), None, "the first sample has no interval");
        assert!(s.sample(pid, 1_000).is_some());
    }

    #[test]
    fn a_missing_process_has_no_sample() {
        let mut s = MainThreadCpu::default();
        assert_eq!(s.sample(0xFFFF_FFF1, 0), None);
        assert_eq!(s.sample(0xFFFF_FFF1, 1_000), None);
    }
}
