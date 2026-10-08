//! The process copy of the start profiles, for consumers that cannot await a
//! database read (the GM-only redirect, the console entry point, respawn).

use std::sync::{Arc, RwLock};

use super::StartProfiles;

static REGISTRY: RwLock<Option<Arc<StartProfiles>>> = RwLock::new(None);

/// Replace the process copy. Called by [`super::load_at_boot`], and by tests
/// with a fixture.
pub fn install(profiles: StartProfiles) -> Arc<StartProfiles> {
    let profiles = Arc::new(profiles);
    // A poisoned lock only means a writer panicked between two whole
    // stores; the value is still a complete set.
    let mut guard = REGISTRY.write().unwrap_or_else(|p| p.into_inner());
    *guard = Some(Arc::clone(&profiles));
    profiles
}

/// The process copy, or `None` before the boot load (or when it failed).
pub fn installed() -> Option<Arc<StartProfiles>> {
    REGISTRY.read().unwrap_or_else(|p| p.into_inner()).clone()
}
