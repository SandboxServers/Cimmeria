//! The process-wide handle to the live sign-in state (client-tls-oidc W4-1).
//!
//! Access tokens and launch tickets live in memory, not in the refresh table,
//! so the base's `.account unlink` path (which runs outside the routes) cannot
//! end them through the database. [`register`] keeps the stores the routes use
//! and [`end_live_sessions`] ends an account's tokens and tickets through them.

use std::sync::{Arc, Mutex};

/// The launch ticket store [`register`] takes, re-exported so a caller that
/// registers the handle needs no auth dependency of its own.
pub use cimmeria_auth::auth::LaunchTicketStore;

use crate::stores::Stores;

/// The stores the routes share, once [`register`] has run.
static HANDLE: Mutex<Option<(Arc<Stores>, Arc<LaunchTicketStore>)>> = Mutex::new(None);

/// Records the stores the mounted routes use. Called when the routes are
/// installed; a later call replaces the earlier stores.
pub fn register(stores: Arc<Stores>, launch_tickets: Arc<LaunchTicketStore>) {
    if let Ok(mut handle) = HANDLE.lock() {
        *handle = Some((stores, launch_tickets));
    }
}

/// Ends every access token and launch ticket of `account_id`. Returns `false`
/// when no routes are registered, so nothing could be ended.
pub fn end_live_sessions(account_id: u32) -> bool {
    let Ok(handle) = HANDLE.lock() else {
        return false;
    };
    let Some((stores, launch_tickets)) = handle.as_ref() else {
        return false;
    };
    stores.access.revoke_account(account_id);
    launch_tickets.revoke_account(account_id);
    true
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn end_live_sessions_ends_the_tokens_and_tickets_of_the_account() {
        let stores = Arc::new(Stores::default());
        let tickets = Arc::new(LaunchTicketStore::default());
        register(Arc::clone(&stores), Arc::clone(&tickets));
        let now = Instant::now();
        let kept = stores.access.mint(8, "family-b", false, now);
        let token = stores.access.mint(7, "family-a", false, now);
        tickets.mint(7, "player", "launch-7", now);
        assert!(stores.access.check(&token, now).is_some());
        assert_eq!(tickets.live_count(), 1);

        assert!(end_live_sessions(7));

        assert!(stores.access.check(&token, now).is_none());
        assert!(stores.access.check(&kept, now).is_some());
        assert_eq!(tickets.live_count(), 0);
    }
}
