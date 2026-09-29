//! "This thread is inside a sink the client already reports."
//!
//! The BigWorld message sink's default output calls `OutputDebugStringA`;
//! `FOutputDeviceDebug` calls `OutputDebugStringW`. Both are also hooked
//! (they may carry lines from sinks nobody else sees), so without a guard
//! every BigWorld or UE3 line would arrive twice: once with its priority or
//! category, once as a bare debug string. A thread that runs a known sink's
//! original function holds a [`SinkGuard`]; the debug-string hooks skip
//! anything reported while one is held.
//!
//! A guard is per thread and re-entrant (a BigWorld message can be logged
//! from inside another sink's original), and it releases on drop, so a C++
//! exception unwinding out of the original cannot leave it set.

use std::cell::Cell;

thread_local! {
    static DEPTH: Cell<u32> = const { Cell::new(0) };
}

/// Held while a known sink's original function runs on this thread.
#[must_use = "the guard marks the sink as running only while it is held"]
pub struct SinkGuard(());

impl SinkGuard {
    /// Enter a known sink on this thread.
    pub fn enter() -> Self {
        DEPTH.with(|d| d.set(d.get().saturating_add(1)));
        SinkGuard(())
    }
}

impl Drop for SinkGuard {
    fn drop(&mut self) {
        DEPTH.with(|d| d.set(d.get().saturating_sub(1)));
    }
}

/// Whether this thread is inside a known sink.
pub fn in_known_sink() -> bool {
    DEPTH.with(|d| d.get() > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_guard_marks_the_thread_until_it_drops() {
        assert!(!in_known_sink());
        {
            let _g = SinkGuard::enter();
            assert!(in_known_sink());
        }
        assert!(!in_known_sink());
    }

    #[test]
    fn guards_nest() {
        let a = SinkGuard::enter();
        let b = SinkGuard::enter();
        drop(b);
        assert!(in_known_sink(), "the outer guard still holds");
        drop(a);
        assert!(!in_known_sink());
    }

    /// A C++ exception (a Rust panic in the tests) leaving the original must
    /// not leave the thread marked, or every later debug string on it would
    /// be dropped.
    #[test]
    fn unwinding_releases_the_guard() {
        let r = std::panic::catch_unwind(|| {
            let _g = SinkGuard::enter();
            panic!("engine error");
        });
        assert!(r.is_err());
        assert!(!in_known_sink());
    }

    /// The mark is per thread: another thread's sink does not hide this
    /// thread's debug strings.
    #[test]
    fn the_mark_is_per_thread() {
        let _g = SinkGuard::enter();
        let other = std::thread::spawn(in_known_sink).join().unwrap();
        assert!(!other);
        assert!(in_known_sink());
    }
}
