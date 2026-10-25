//! Counts the relays that are running, so the close guard knows a launch is live.
//!
//! A [`RelayLease`] is held by a launch's client login for as long as its relay
//! runs. The count goes down when the lease drops, so an early return or a
//! panic cannot leave it raised.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Relays that are running in this process, counted by [`RelayLease`].
pub static ACTIVE_RELAYS: AtomicUsize = AtomicUsize::new(0);

/// How many relays are running now.
pub fn active_relays() -> usize {
    ACTIVE_RELAYS.load(Ordering::SeqCst)
}

/// Holds one count of a relay counter while it lives.
pub struct RelayLease(&'static AtomicUsize);

impl RelayLease {
    /// Takes one count on `counter`; the count is given back when the lease drops.
    pub fn on(counter: &'static AtomicUsize) -> RelayLease {
        counter.fetch_add(1, Ordering::SeqCst);
        RelayLease(counter)
    }
}

impl Drop for RelayLease {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
