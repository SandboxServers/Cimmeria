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
    end_in(stores, launch_tickets, account_id);
    true
}

/// Ends the access tokens and launch tickets of `account_id` in the given
/// stores. [`end_live_sessions`] calls it with the registered stores.
fn end_in(stores: &Stores, launch_tickets: &LaunchTicketStore, account_id: u32) {
    // The order is load-bearing (C16.4): access tokens first, then launch
    // tickets. The ticket route checks the grant after taking its guard, so a
    // grant revoked here is refused there. Do not swap the two lines.
    stores.access.revoke_account(account_id);
    launch_tickets.revoke_account(account_id);
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn end_in_ends_the_tokens_and_tickets_of_the_account() {
        // Local stores and no `register`: the process-wide handle is shared with
        // other tests in one `cargo test` process, so this one stays off it.
        let stores = Stores::default();
        let tickets = LaunchTicketStore::default();
        let now = Instant::now();
        let kept = stores.access.mint(8, "family-b", false, now);
        let token = stores.access.mint(7, "family-a", false, now);
        tickets.mint(7, "player", "launch-7", now);
        assert!(stores.access.check(&token, now).is_some());
        assert_eq!(tickets.live_count(), 1);

        end_in(&stores, &tickets, 7);

        assert!(stores.access.check(&token, now).is_none());
        assert!(stores.access.check(&kept, now).is_some());
        assert_eq!(tickets.live_count(), 0);
    }
}
