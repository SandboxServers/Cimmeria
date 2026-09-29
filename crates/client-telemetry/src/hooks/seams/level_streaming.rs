//! Level streaming: when a streamed level becomes visible, and which
//! streaming step blew its time budget.
//!
//! `client.engine.update_level_streaming` (in `inline_hooks`) samples the
//! per-level, per-frame helper `UWorld::UpdateLevelStreamingInner`
//! (`0x0054e9c0`) at 1/10, which shows that streaming is running but says
//! nothing about what it did. The helper is a resumable state machine: each
//! call does the next step of loading a level (serialise, associate, init
//! actors, route `BeginPlay`) until the frame's time budget
//! (`_DAT_01b4c0b0` against a `QueryPerformanceCounter` delta) runs out, and
//! resumes next frame. Its flags live on the loaded level (`ULevel + 0x164`
//! and on: `piVar11[0x59]` "making visible", `[0x5a]`..`[0x65]` the phases).
//! The end of the function sets bit 0 of `StreamingLevel + 0x60`
//! (`bIsVisible`) from `piVar11[0x59] == 0`: the level is visible when the
//! last phase has cleared the in-progress flag.
//!
//! So two things are observable from the outside, without decoding the
//! phases:
//!
//! - **visible**: `bIsVisible` is clear on entry (the function asserts it,
//!   `"!StreamingLevel->bIsVisible"`, `UnWorld.cpp:0x446`) and set on exit.
//!   That is the moment a streamed level appeared, reported once per level
//!   with the package name and how long that final step took.
//! - **slow**: a step whose wall time exceeds the budget. A hitch in the
//!   frame timeline that lines up with one of these is streaming, not
//!   rendering.
//!
//! Layout (from the function's own reads): `StreamingLevel + 0x44` is
//! `LoadedLevel`; `+ 0x60` bit 0 is `bIsVisible`. The package is the
//! outermost `Outer` of the loaded level ([`objects::outermost`]).
//!
//! Static evidence only (2026-09-28: full decompile of `0x0054e9c0`); not
//! yet seen from a live client.

use serde_json::json;

use super::objects;
use crate::hooks::sinks::emit::Fields;
use crate::hooks::sinks::mem::{self, Reader};

/// Offset of `LoadedLevel` in `ULevelStreaming`.
pub const LOADED_LEVEL_OFFSET: usize = 0x44;
/// Offset of the flags word holding `bIsVisible` (bit 0).
pub const FLAGS_OFFSET: usize = 0x60;

/// A step slower than this many milliseconds is reported.
pub const SLOW_STEP_MS: f64 = 30.0;
/// The same under `firehose`.
pub const SLOW_STEP_MS_FIREHOSE: f64 = 10.0;

/// Target of a level becoming visible.
pub const VISIBLE_TARGET: &str = "client.engine.level_visible";
/// Target of a slow step.
pub const SLOW_TARGET: &str = "client.engine.level_stream_slow";

/// What one call of the helper amounted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepOutcome {
    /// The level went from not visible to visible.
    pub became_visible: bool,
    /// The step exceeded the budget.
    pub slow: bool,
}

/// Classify a step from what was seen around it.
pub fn classify(
    visible_before: bool,
    visible_after: bool,
    elapsed_ms: f64,
    slow_ms: f64,
) -> StepOutcome {
    StepOutcome {
        became_visible: !visible_before && visible_after,
        slow: elapsed_ms >= slow_ms,
    }
}

/// The slow-step threshold in force.
pub fn slow_threshold(firehose: bool) -> f64 {
    if firehose {
        SLOW_STEP_MS_FIREHOSE
    } else {
        SLOW_STEP_MS
    }
}

/// Whether a streaming level's flags word has `bIsVisible` set.
pub fn visible_flag(flags: u32) -> bool {
    flags & 1 != 0
}

/// Read `bIsVisible` of a streaming level.
pub fn is_visible(read: Reader, streaming: usize) -> Option<bool> {
    mem::read_u32(read, streaming + FLAGS_OFFSET).map(visible_flag)
}

/// The package a streaming level loads: the outermost `Outer` of its loaded
/// level.
pub fn package_name(read: Reader, streaming: usize) -> Option<String> {
    let level = mem::read_u32(read, streaming + LOADED_LEVEL_OFFSET)? as usize;
    if level == 0 {
        return None;
    }
    let package = objects::outermost(read, level)?;
    objects::object_name(read, package)
}

/// The fields of a visible or slow event.
pub fn step_fields(package: &str, elapsed_ms: f64, visible: bool, suppressed: u64) -> Fields {
    let mut f: Fields = vec![
        ("package", json!(package)),
        ("elapsed_ms", json!((elapsed_ms * 10.0).round() / 10.0)),
        ("visible", json!(visible)),
    ];
    if suppressed > 0 {
        f.push(("suppressed", json!(suppressed)));
    }
    f
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
mod x86 {
    use std::ffi::c_void;
    use std::time::Instant;

    use super::*;
    use crate::hooks::name_throttle::Decision;
    use crate::hooks::sinks::emit::emit;
    use crate::hooks::sinks::mem::process_reader;
    use crate::hooks::sinks::throttle::SinkThrottle;

    static THROTTLE: SinkThrottle = SinkThrottle::new();

    /// State captured before the helper runs.
    pub struct Step {
        visible_before: bool,
        started: Instant,
    }

    /// Note the streaming level's visibility and the time. The helper
    /// dereferences its argument unconditionally, so a null one is skipped
    /// here and left to fail where it always did.
    pub fn begin(streaming: *const c_void) -> Option<Step> {
        if streaming.is_null() {
            return None;
        }
        Some(Step {
            visible_before: is_visible(&process_reader, streaming as usize)?,
            started: Instant::now(),
        })
    }

    /// Report what the step did.
    pub fn end(streaming: *const c_void, step: Step) {
        let elapsed_ms = step.started.elapsed().as_secs_f64() * 1000.0;
        let read = &process_reader;
        let visible_after = is_visible(read, streaming as usize).unwrap_or(step.visible_before);
        let outcome = classify(
            step.visible_before,
            visible_after,
            elapsed_ms,
            slow_threshold(crate::capture::current().firehose),
        );
        if !outcome.became_visible && !outcome.slow {
            return;
        }
        let package = package_name(read, streaming as usize).unwrap_or_else(|| "<unknown>".into());
        if outcome.became_visible {
            if let Decision::Emit { suppressed } = THROTTLE.check(&format!("visible:{package}")) {
                emit(
                    VISIBLE_TARGET,
                    "info",
                    "engine.level_visible",
                    step_fields(&package, elapsed_ms, true, suppressed),
                );
            }
        }
        if outcome.slow {
            if let Decision::Emit { suppressed } = THROTTLE.check(&format!("slow:{package}")) {
                emit(
                    SLOW_TARGET,
                    "warn",
                    "engine.level_stream_slow",
                    step_fields(&package, elapsed_ms, visible_after, suppressed),
                );
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::hooks::sinks::emit::take_captured;

        /// A streaming level in real memory whose flags the test flips
        /// between `begin` and `end`, as the helper's last step does.
        #[test]
        fn a_step_that_sets_the_visible_bit_is_reported_once() {
            let mut streaming = vec![0u8; 0x80];
            let ptr = streaming.as_mut_ptr();
            let _ = take_captured();

            let step = begin(ptr.cast()).expect("readable");
            // The helper's final action.
            streaming[FLAGS_OFFSET] |= 1;
            end(ptr.cast(), step);

            let events = take_captured();
            assert_eq!(events.len(), 1, "{events:?}");
            assert_eq!(events[0].target, VISIBLE_TARGET);
            assert_eq!(events[0].get("visible"), Some(&json!(true)));
            // No name table in a test process.
            assert_eq!(events[0].get("package"), Some(&json!("<unknown>")));

            // A step that leaves an already visible level alone reports
            // nothing (fast, no transition).
            let step = begin(ptr.cast()).unwrap();
            end(ptr.cast(), step);
            assert!(take_captured().is_empty());
        }

        #[test]
        fn a_null_streaming_level_is_skipped() {
            assert!(begin(std::ptr::null()).is_none());
        }
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86"))]
pub(in crate::hooks) use x86::{begin, end};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hooks::seams::objects::fake::{name_table, object};
    use crate::hooks::sinks::mem::fake::FakeMemory;

    #[test]
    fn a_level_becomes_visible_on_the_step_that_sets_the_bit() {
        assert_eq!(
            classify(false, true, 5.0, 30.0),
            StepOutcome {
                became_visible: true,
                slow: false
            }
        );
        // Already visible before: no new transition.
        assert!(!classify(true, true, 5.0, 30.0).became_visible);
        // Not yet visible after: still loading.
        assert!(!classify(false, false, 5.0, 30.0).became_visible);
    }

    #[test]
    fn a_step_is_slow_at_the_threshold_and_above() {
        assert!(!classify(false, false, 29.9, 30.0).slow);
        assert!(classify(false, false, 30.0, 30.0).slow);
        assert!(classify(true, true, 250.0, 30.0).slow);
    }

    #[test]
    fn firehose_lowers_the_slow_threshold() {
        assert!(slow_threshold(true) < slow_threshold(false));
    }

    #[test]
    fn visibility_is_bit_zero_only() {
        assert!(visible_flag(1));
        assert!(visible_flag(0x41));
        assert!(!visible_flag(0x40));
        assert!(!visible_flag(0));
    }

    /// The package is the outermost outer of the loaded level, not the
    /// level's own name (`PersistentLevel`).
    #[test]
    fn the_package_is_the_outermost_outer_of_the_loaded_level() {
        let mut m = FakeMemory::new();
        name_table(&mut m, &["None", "sg1_p9q", "PersistentLevel", "Package"]);
        object(&mut m, 0x3000, 0, (1, 0), 0); // package
        object(&mut m, 0x4000, 0x3000, (2, 0), 0); // level
        m.put(0x1000 + LOADED_LEVEL_OFFSET, &0x4000u32.to_le_bytes());
        m.put(0x1000 + FLAGS_OFFSET, &1u32.to_le_bytes());
        let r = m.reader();
        assert_eq!(package_name(&r, 0x1000).as_deref(), Some("sg1_p9q"));
        assert_eq!(is_visible(&r, 0x1000), Some(true));
        drop(r);
        // No loaded level yet: no package.
        m.put(0x1000 + LOADED_LEVEL_OFFSET, &0u32.to_le_bytes());
        assert_eq!(package_name(&m.reader(), 0x1000), None);
    }

    #[test]
    fn fields_round_the_time_and_flag_the_state() {
        let f = step_fields("sg1_p9q", 41.26, true, 3);
        let get = |k: &str| f.iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone());
        assert_eq!(get("package"), Some(json!("sg1_p9q")));
        assert_eq!(get("elapsed_ms"), Some(json!(41.3)));
        assert_eq!(get("visible"), Some(json!(true)));
        assert_eq!(get("suppressed"), Some(json!(3)));
    }

    #[test]
    fn the_layout_constants_are_the_recovered_ones() {
        assert_eq!(LOADED_LEVEL_OFFSET, 0x44);
        assert_eq!(FLAGS_OFFSET, 0x60);
    }
}
