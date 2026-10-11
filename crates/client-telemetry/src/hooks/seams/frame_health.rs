//! Frame hitches and process memory, sampled from the engine tick.
//!
//! `client.engine.tick` samples `FEngineLoop::Tick` at 1/100, which is
//! enough to see that frames are happening and not enough to see a frame
//! that took a second. This runs on every tick instead, costing one clock
//! read and a compare:
//!
//! - **`client.engine.hitch`**: the time between two consecutive tick
//!   entries exceeded the threshold (200 ms; 100 ms under `firehose`), with
//!   the frame number and the process's working set at that moment. The
//!   first tick after a long gap (a level load, an alt-tab) is a hitch by this
//!   definition, which is the point: the gap is when the player saw a freeze.
//!   Rate-limited to the default burst.
//! - **`client.engine.memory`**: every 30 seconds, the working set, the
//!   private bytes, the peak working set, and (the one that matters for a
//!   32-bit process) the free address space: an `SGW.exe` that has used up its
//!   4 GB of address space (the stock exe is Large Address Aware) fails
//!   allocations while the machine still has gigabytes of RAM free, and a
//!   steadily falling `avail_virtual_mb` is the warning. Each sample also carries the frame-time percentiles of the
//!   window since the previous one (`frames`, `frame_p50_ms`, `frame_p95_ms`,
//!   `frame_max_ms`), from the tick gaps the hitch check already measures.
//!
//! Both are read on the main thread. The memory numbers come from
//! `K32GetProcessMemoryInfo` and `GlobalMemoryStatusEx`, declared here
//! directly (a `kernel32` link the DLL already has) so the crate does not
//! grow a `windows-sys` feature, and with it a workspace-hack change, for
//! two functions.

#[cfg(all(target_os = "windows", target_arch = "x86"))]
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use serde_json::json;

use crate::hooks::sinks::emit::Fields;

/// A tick-to-tick gap at or above this is a hitch.
pub const HITCH_MS: f64 = 200.0;
/// The same under `firehose`.
pub const HITCH_MS_FIREHOSE: f64 = 100.0;
/// Milliseconds between memory samples.
pub const MEMORY_INTERVAL_MS: u64 = 30_000;

/// Telemetry target of a hitch.
pub const HITCH_TARGET: &str = "client.engine.hitch";
/// Telemetry target of a memory sample.
pub const MEMORY_TARGET: &str = "client.engine.memory";

/// The hitch threshold in force.
pub fn hitch_threshold(firehose: bool) -> f64 {
    if firehose {
        HITCH_MS_FIREHOSE
    } else {
        HITCH_MS
    }
}

/// Whether a gap is a hitch.
pub fn is_hitch(gap_ms: f64, threshold_ms: f64) -> bool {
    gap_ms >= threshold_ms
}

/// Whether a memory sample is due.
pub fn memory_due(now_ms: u64, last_sample_ms: u64) -> bool {
    now_ms.saturating_sub(last_sample_ms) >= MEMORY_INTERVAL_MS
}

/// Bytes as megabytes, one decimal.
pub fn mb(bytes: u64) -> f64 {
    (bytes as f64 / (1024.0 * 1024.0) * 10.0).round() / 10.0
}

/// What the OS says about this process's memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MemorySnapshot {
    /// Working set, bytes.
    pub working_set: u64,
    /// Peak working set, bytes.
    pub peak_working_set: u64,
    /// Private (committed) bytes.
    pub private_bytes: u64,
    /// Free virtual address space of this process, bytes.
    pub avail_virtual: u64,
    /// Total virtual address space of this process, bytes.
    pub total_virtual: u64,
    /// Machine-wide physical memory load, percent.
    pub load_percent: u32,
}

/// The fields of a hitch.
pub fn hitch_fields(gap_ms: f64, frame: u64, working_set: Option<u64>, suppressed: u64) -> Fields {
    let mut f: Fields = vec![("gap_ms", json!(gap_ms.round())), ("frame", json!(frame))];
    if let Some(ws) = working_set {
        f.push(("working_set_mb", json!(mb(ws))));
    }
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

/// The fields of a memory sample.
pub fn memory_fields(m: &MemorySnapshot) -> Fields {
    vec![
        ("working_set_mb", json!(mb(m.working_set))),
        ("peak_working_set_mb", json!(mb(m.peak_working_set))),
        ("private_mb", json!(mb(m.private_bytes))),
        ("avail_virtual_mb", json!(mb(m.avail_virtual))),
        ("total_virtual_mb", json!(mb(m.total_virtual))),
        ("machine_load_percent", json!(m.load_percent)),
    ]
}

/// Telemetry level of a memory sample: `warn` when less than 256 MB of the
/// address space is left (an allocation of a large texture or a level will
/// start to fail), else `info`.
pub fn memory_level(m: &MemorySnapshot) -> &'static str {
    if m.total_virtual != 0 && m.avail_virtual < 256 * 1024 * 1024 {
        "warn"
    } else {
        "info"
    }
}

/// Upper bounds in ms of the frame-gap buckets; the last is open-ended.
pub const FRAME_BUCKETS_MS: [u32; 12] = [8, 12, 17, 25, 33, 50, 67, 100, 150, 200, 500, u32::MAX];

/// Tick-to-tick gaps of one memory-sample window, bucketed; no allocation.
/// Not `Copy`, so a window can only leave its slot through [`FrameGaps::take`].
#[derive(Debug, Clone, PartialEq)]
pub struct FrameGaps {
    counts: [u32; 12],
    max_ms: f64,
    frames: u32,
}

impl FrameGaps {
    /// An empty window.
    pub const fn new() -> Self {
        Self {
            counts: [0; 12],
            max_ms: 0.0,
            frames: 0,
        }
    }

    /// Counts one tick-to-tick gap.
    pub fn record(&mut self, gap_ms: f64) {
        let bucket = FRAME_BUCKETS_MS
            .iter()
            .position(|&bound| f64::from(bound) >= gap_ms)
            .unwrap_or(FRAME_BUCKETS_MS.len() - 1);
        self.counts[bucket] += 1;
        self.frames += 1;
        self.max_ms = self.max_ms.max(gap_ms);
    }

    /// Bucket upper bound at or above the percentile `p` (0.0..=1.0); `None`
    /// when the window is empty.
    pub fn percentile_ms(&self, p: f64) -> Option<u32> {
        if self.frames == 0 {
            return None;
        }
        let rank = ((p * f64::from(self.frames)).ceil() as u32).clamp(1, self.frames);
        let mut seen = 0;
        for (bound, count) in FRAME_BUCKETS_MS.iter().zip(self.counts.iter()) {
            seen += count;
            if seen >= rank {
                return Some(*bound);
            }
        }
        None
    }

    /// The window so far, leaving this one empty.
    pub fn take(&mut self) -> FrameGaps {
        std::mem::take(self)
    }
}

impl Default for FrameGaps {
    fn default() -> Self {
        Self::new()
    }
}

/// The frame fields of a memory sample: `frames`, `frame_p50_ms`, `frame_p95_ms`,
/// `frame_max_ms`; empty when no frame was recorded (no nulls).
pub fn frame_fields(g: &FrameGaps) -> Fields {
    // The open-ended last bucket has no bound to report; the window's
    // longest gap stands in for it.
    let bound = |b: u32| {
        if b == u32::MAX {
            g.max_ms.ceil() as u32
        } else {
            b
        }
    };
    let (Some(p50), Some(p95)) = (g.percentile_ms(0.5), g.percentile_ms(0.95)) else {
        return Vec::new();
    };
    let (p50, p95) = (bound(p50), bound(p95));
    vec![
        ("frames", json!(g.frames)),
        ("frame_p50_ms", json!(p50)),
        ("frame_p95_ms", json!(p95)),
        ("frame_max_ms", json!((g.max_ms * 10.0).round() / 10.0)),
    ]
}

/// Counts one gap into a shared window. A poisoned lock is recovered, so a
/// window is never dropped.
pub fn record_gap(window: &Mutex<FrameGaps>, gap_ms: f64) {
    window
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .record(gap_ms);
}

/// Takes a shared window, leaving it empty for the next sample.
pub fn take_window(window: &Mutex<FrameGaps>) -> FrameGaps {
    window.lock().unwrap_or_else(PoisonError::into_inner).take()
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
static FRAME: AtomicU64 = AtomicU64::new(0);
/// The previous tick's time plus one millisecond (0 = no previous tick, so a
/// tick at time 0 is not mistaken for none).
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static LAST_TICK: AtomicU64 = AtomicU64::new(0);
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static LAST_MEMORY_MS: AtomicU64 = AtomicU64::new(0);
/// The tick gaps since the previous memory sample.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
static FRAME_GAPS: Mutex<FrameGaps> = Mutex::new(FrameGaps::new());

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod os {
    use std::ffi::c_void;

    use super::MemorySnapshot;

    #[repr(C)]
    struct ProcessMemoryCountersEx {
        cb: u32,
        page_fault_count: u32,
        peak_working_set_size: usize,
        working_set_size: usize,
        quota_peak_paged_pool_usage: usize,
        quota_paged_pool_usage: usize,
        quota_peak_non_paged_pool_usage: usize,
        quota_non_paged_pool_usage: usize,
        pagefile_usage: usize,
        peak_pagefile_usage: usize,
        private_usage: usize,
    }

    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn K32GetProcessMemoryInfo(
            process: *mut c_void,
            counters: *mut ProcessMemoryCountersEx,
            cb: u32,
        ) -> i32;
        fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
    }

    /// This process's memory numbers, or `None` if either call fails.
    pub fn snapshot() -> Option<MemorySnapshot> {
        use windows_sys::Win32::System::Threading::GetCurrentProcess;

        // SAFETY: both structs are `repr(C)` mirrors of the Win32 ones with
        // their size fields set; the calls only fill them.
        unsafe {
            let mut c: ProcessMemoryCountersEx = std::mem::zeroed();
            c.cb = std::mem::size_of::<ProcessMemoryCountersEx>() as u32;
            if K32GetProcessMemoryInfo(GetCurrentProcess(), &mut c, c.cb) == 0 {
                return None;
            }
            let mut s: MemoryStatusEx = std::mem::zeroed();
            s.length = std::mem::size_of::<MemoryStatusEx>() as u32;
            if GlobalMemoryStatusEx(&mut s) == 0 {
                return None;
            }
            Some(MemorySnapshot {
                working_set: c.working_set_size as u64,
                peak_working_set: c.peak_working_set_size as u64,
                private_bytes: c.private_usage as u64,
                avail_virtual: s.avail_virtual,
                total_virtual: s.total_virtual,
                load_percent: s.memory_load,
            })
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// The mirror structs have the Win32 sizes (`PROCESS_MEMORY_COUNTERS_EX`
        /// is 44 bytes on x86 and 80 on x64; `MEMORYSTATUSEX` is 64 either way).
        #[test]
        fn the_mirror_structs_have_the_win32_sizes() {
            assert_eq!(std::mem::size_of::<MemoryStatusEx>(), 64);
            let expected = if cfg!(target_pointer_width = "32") {
                44
            } else {
                80
            };
            assert_eq!(std::mem::size_of::<ProcessMemoryCountersEx>(), expected);
        }

        /// The real calls work in this process and give sane numbers.
        #[test]
        fn a_snapshot_of_this_process_is_sane() {
            let s = snapshot().expect("both calls succeed");
            assert!(s.working_set > 0 && s.working_set <= s.peak_working_set);
            assert!(s.total_virtual > 0 && s.avail_virtual <= s.total_virtual);
            assert!(s.load_percent <= 100);
        }
    }
}

/// Called at the top of every `FEngineLoop::Tick`, on the main thread.
#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(in crate::hooks) fn on_tick() {
    use crate::hooks::name_throttle::Decision;
    use crate::hooks::sinks::emit::emit;
    use crate::hooks::sinks::throttle::{now_ms, SinkThrottle};

    static THROTTLE: SinkThrottle = SinkThrottle::new();

    let now = now_ms();
    let frame = FRAME.fetch_add(1, Ordering::Relaxed);
    let last = LAST_TICK.swap(now + 1, Ordering::Relaxed);
    if last != 0 {
        let gap = (now + 1).saturating_sub(last) as f64;
        record_gap(&FRAME_GAPS, gap);
        let threshold = hitch_threshold(crate::capture::current().firehose);
        if is_hitch(gap, threshold) {
            if let Decision::Emit { suppressed } = THROTTLE.check("hitch") {
                let ws = os::snapshot().map(|s| s.working_set);
                emit(
                    HITCH_TARGET,
                    "warn",
                    "engine.hitch",
                    hitch_fields(gap, frame, ws, suppressed),
                );
            }
        }
    }
    if memory_due(now, LAST_MEMORY_MS.load(Ordering::Relaxed)) {
        LAST_MEMORY_MS.store(now, Ordering::Relaxed);
        let taken = take_window(&FRAME_GAPS);
        if let Some(m) = os::snapshot() {
            let mut fields = memory_fields(&m);
            fields.extend(frame_fields(&taken));
            emit(MEMORY_TARGET, memory_level(&m), "engine.memory", fields);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_gap_is_a_hitch_at_the_threshold_and_above() {
        assert!(!is_hitch(199.0, HITCH_MS));
        assert!(is_hitch(200.0, HITCH_MS));
        assert!(is_hitch(1500.0, HITCH_MS));
        assert!(hitch_threshold(true) < hitch_threshold(false));
    }

    #[test]
    fn memory_samples_come_every_thirty_seconds() {
        assert!(!memory_due(29_999, 0));
        assert!(memory_due(30_000, 0));
        assert!(!memory_due(40_000, 20_000));
        assert!(memory_due(50_000, 20_000));
        // A clock that went backwards never fires.
        assert!(!memory_due(10, 20_000));
    }

    #[test]
    fn megabytes_round_to_a_tenth() {
        assert_eq!(mb(0), 0.0);
        assert_eq!(mb(1024 * 1024), 1.0);
        assert_eq!(mb(1_572_864), 1.5);
        assert_eq!(mb(3 * 1024 * 1024 * 1024), 3072.0);
    }

    #[test]
    fn the_address_space_warning_is_below_256_mb() {
        let mut m = MemorySnapshot {
            total_virtual: 2 * 1024 * 1024 * 1024,
            avail_virtual: 300 * 1024 * 1024,
            ..MemorySnapshot::default()
        };
        assert_eq!(memory_level(&m), "info");
        m.avail_virtual = 100 * 1024 * 1024;
        assert_eq!(memory_level(&m), "warn");
        // An unknown total (the call failed) is not a warning.
        assert_eq!(memory_level(&MemorySnapshot::default()), "info");
    }

    #[test]
    fn a_hitch_carries_the_gap_frame_and_working_set() {
        let f = hitch_fields(731.4, 9000, Some(1_500 * 1024 * 1024), 2);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("gap_ms"), Some(json!(731.0)));
        assert_eq!(get("frame"), Some(json!(9000)));
        assert_eq!(get("working_set_mb"), Some(json!(1500.0)));
        assert_eq!(get("suppressed"), Some(json!(2)));
        assert!(!hitch_fields(300.0, 1, None, 0)
            .iter()
            .any(|(k, _)| *k == "working_set_mb"));
    }

    #[test]
    fn a_memory_sample_reports_every_number() {
        let f = memory_fields(&MemorySnapshot {
            working_set: 2 * 1024 * 1024,
            peak_working_set: 3 * 1024 * 1024,
            private_bytes: 4 * 1024 * 1024,
            avail_virtual: 5 * 1024 * 1024,
            total_virtual: 6 * 1024 * 1024,
            load_percent: 42,
        });
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("working_set_mb"), Some(json!(2.0)));
        assert_eq!(get("peak_working_set_mb"), Some(json!(3.0)));
        assert_eq!(get("private_mb"), Some(json!(4.0)));
        assert_eq!(get("avail_virtual_mb"), Some(json!(5.0)));
        assert_eq!(get("total_virtual_mb"), Some(json!(6.0)));
        assert_eq!(get("machine_load_percent"), Some(json!(42)));
    }

    #[test]
    fn frame_percentiles_pick_the_bucket_bound() {
        let mut g = FrameGaps::new();
        for _ in 0..90 {
            g.record(16.0);
        }
        for _ in 0..10 {
            g.record(120.0);
        }
        assert_eq!(g.percentile_ms(0.5), Some(17));
        assert_eq!(g.percentile_ms(0.95), Some(150));
        g.record(120.04);
        let f = frame_fields(&g);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("frames"), Some(json!(101)));
        assert_eq!(get("frame_p50_ms"), Some(json!(17)));
        assert_eq!(get("frame_p95_ms"), Some(json!(150)));
        assert_eq!(get("frame_max_ms"), Some(json!(120.0)));
    }

    /// A gap equal to a bucket's bound lands in that bucket; just above it,
    /// in the next.
    #[test]
    fn a_gap_on_a_bound_lands_in_that_bucket() {
        for (gap, bucket) in [(8.0, 8), (8.1, 12), (17.0, 17), (17.5, 25), (500.0, 500)] {
            let mut g = FrameGaps::new();
            g.record(gap);
            assert_eq!(g.percentile_ms(1.0), Some(bucket), "gap {gap}");
        }
        let mut g = FrameGaps::new();
        g.record(500.4);
        assert_eq!(g.percentile_ms(1.0), Some(u32::MAX));
    }

    /// The shared window the tick fills: each take covers only the gaps
    /// since the previous one.
    #[test]
    fn each_sample_takes_its_own_window() {
        let window = Mutex::new(FrameGaps::new());
        record_gap(&window, 16.0);
        record_gap(&window, 33.0);
        assert!(frame_fields(&take_window(&window))
            .iter()
            .any(|(n, v)| *n == "frames" && *v == json!(2)));
        assert_eq!(take_window(&window), FrameGaps::new());
        record_gap(&window, 50.0);
        assert!(frame_fields(&take_window(&window))
            .iter()
            .any(|(n, v)| *n == "frames" && *v == json!(1)));
    }

    /// A panic while the lock is held does not lose the window.
    #[test]
    fn a_poisoned_window_is_recovered() {
        let window = Mutex::new(FrameGaps::new());
        record_gap(&window, 16.0);
        let _ = std::panic::catch_unwind(|| {
            let _guard = window.lock().unwrap();
            panic!("poison the window");
        });
        assert!(window.is_poisoned());
        record_gap(&window, 16.0);
        assert_eq!(take_window(&window).percentile_ms(1.0), Some(17));
    }

    #[test]
    fn taking_the_window_resets_it() {
        let mut g = FrameGaps::new();
        g.record(16.0);
        g.record(40.0);
        let taken = g.take();
        let f = frame_fields(&taken);
        assert!(f.iter().any(|(n, v)| *n == "frames" && *v == json!(2)));
        let again = g.take();
        assert_eq!(again, FrameGaps::new());
        assert!(frame_fields(&again).is_empty());
        assert_eq!(again.percentile_ms(0.5), None);
    }

    #[test]
    fn a_percentile_in_the_open_bucket_reports_the_longest_gap() {
        let mut g = FrameGaps::new();
        for _ in 0..10 {
            g.record(16.0);
        }
        for _ in 0..90 {
            g.record(30_000.4);
        }
        assert_eq!(g.percentile_ms(0.95), Some(u32::MAX));
        let f = frame_fields(&g);
        assert!(f
            .iter()
            .any(|(n, v)| *n == "frame_p95_ms" && *v == json!(30_001)));
    }

    #[test]
    fn an_empty_window_has_no_frame_fields() {
        let g = FrameGaps::default();
        assert!(frame_fields(&g).is_empty());
        assert_eq!(g.percentile_ms(0.95), None);
    }
}
