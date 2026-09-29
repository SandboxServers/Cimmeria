//! Chaining our top-level exception filter under whatever the game sets.
//!
//! We install our filter with the real `SetUnhandledExceptionFilter` and
//! keep the filter it displaced as `next`. The game's own calls go
//! through its IAT slot, which we point at a detour: the detour records
//! the game's filter as the new `next` and returns the previous `next`,
//! which is exactly what the real API would have returned to the game.
//! Our filter stays on top and hands every exception on to `next`, so
//! the game's (and the CRT's) handling is unchanged.
//!
//! Pointers are `usize` (0 = none) so this is host-testable.

use std::sync::atomic::{AtomicUsize, Ordering};

/// `EXCEPTION_CONTINUE_SEARCH`: let the next handler (or the OS
/// default, Windows Error Reporting) deal with it.
pub const EXCEPTION_CONTINUE_SEARCH: i32 = 0;

#[derive(Debug, Default)]
pub struct FilterChain {
    next: AtomicUsize,
}

impl FilterChain {
    pub const fn new() -> Self {
        Self {
            next: AtomicUsize::new(0),
        }
    }

    /// We just installed ours with the real API; `displaced` is what it
    /// returned.
    pub fn installed_over(&self, displaced: usize) {
        self.next.store(displaced, Ordering::Release);
    }

    /// The game called `SetUnhandledExceptionFilter(new)` through its
    /// IAT. Returns what the call returns to the game: the filter the
    /// game would have seen as current.
    pub fn game_sets(&self, new: usize) -> usize {
        self.next.swap(new, Ordering::AcqRel)
    }

    /// The filter ours hands on to, 0 for none.
    pub fn next(&self) -> usize {
        self.next.load(Ordering::Acquire)
    }
}

/// What our filter returns: whatever the next filter said, or
/// `EXCEPTION_CONTINUE_SEARCH` when there is none. Never
/// `EXCEPTION_EXECUTE_HANDLER` of its own making: we observe a crash,
/// we do not swallow it.
pub fn filter_result(next_result: Option<i32>) -> i32 {
    next_result.unwrap_or(EXCEPTION_CONTINUE_SEARCH)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game's view of the API must be unchanged: each call returns
    /// the filter the game itself set before (or the one that was there
    /// when we installed).
    #[test]
    fn game_sees_the_filters_it_set() {
        let chain = FilterChain::new();
        chain.installed_over(0xC0DE);
        assert_eq!(chain.game_sets(0xA), 0xC0DE);
        assert_eq!(chain.next(), 0xA);
        assert_eq!(chain.game_sets(0xB), 0xA);
        assert_eq!(chain.next(), 0xB);
    }

    /// The CRT's `__report_gsfailure` clears the filter with NULL before
    /// calling `UnhandledExceptionFilter`: ours stays on top and then
    /// chains to nothing, i.e. the OS default, as the CRT intended.
    #[test]
    fn clearing_the_filter_chains_to_the_os_default() {
        let chain = FilterChain::new();
        chain.installed_over(0xC0DE);
        assert_eq!(chain.game_sets(0), 0xC0DE);
        assert_eq!(chain.next(), 0);
        assert_eq!(filter_result(None), EXCEPTION_CONTINUE_SEARCH);
    }

    #[test]
    fn result_is_the_next_filters_verdict() {
        assert_eq!(filter_result(Some(1)), 1); // EXCEPTION_EXECUTE_HANDLER
        assert_eq!(filter_result(Some(-1)), -1); // EXCEPTION_CONTINUE_EXECUTION
        assert_eq!(filter_result(Some(0)), 0);
    }
}
