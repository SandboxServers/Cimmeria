//! Test-only seams in the login handlers. Outside `cfg(test)` every hook is an
//! empty function the optimiser removes, so production behaviour is unchanged.
//!
//! The hooks run on the calling thread. The handler tests use the
//! current-thread `#[tokio::test]` runtime, so a hook set by a test runs on
//! that test's thread, at the point the handler reaches it.

#[cfg(test)]
use std::cell::RefCell;

#[cfg(test)]
thread_local! {
    static AFTER_TICKET_LOOKUP: RefCell<Option<Box<dyn FnOnce()>>> = const { RefCell::new(None) };
}

/// Arm a one-shot action to run right after the UserAuth ticket lookup. The
/// test uses it to revoke the ticket while the login is in flight.
#[cfg(test)]
pub(crate) fn set_after_ticket_lookup(f: impl FnOnce() + 'static) {
    AFTER_TICKET_LOOKUP.with(|slot| *slot.borrow_mut() = Some(Box::new(f)));
}

/// Run and clear the armed UserAuth lookup action, if any.
#[cfg(test)]
pub(crate) fn after_ticket_lookup() {
    let hook = AFTER_TICKET_LOOKUP.with(|slot| slot.borrow_mut().take());
    if let Some(hook) = hook {
        hook();
    }
}

/// Production build: nothing runs between the ticket lookup and the SID insert.
#[cfg(not(test))]
#[inline]
pub(crate) fn after_ticket_lookup() {}
