//! The session state one auth host keeps (CT-86, F08).
//!
//! Every [`SignIn`](super::session::SignIn) the driver makes for one host shares
//! one [`HostSlot`], so a refresh and a sign-out on any of them serialize on one
//! lock and one refresh token is spent once. The store helpers take what they
//! use, so the rules for the stored sign-in change in one place.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use url::Url;

use super::api::SignedIn;
use super::session::AccountSummary;
use super::store::{CredentialStore, StoreError, Stored};

/// What one auth host's sessions share: the access token, the account and the
/// store state (CT-88): a sign-in the store refused, a sign-out it refused, and
/// the warning the status shows.
#[derive(Default)]
pub(super) struct HostState {
    /// The access token and the instant it stops being valid.
    pub(super) access: Option<(String, Instant)>,
    pub(super) account: Option<AccountSummary>,
    /// A sign-in the store refused to keep: it works until the launcher exits.
    pub(super) session_only: Option<Stored>,
    /// A sign-out the store refused to delete: the host reads as signed out.
    pub(super) tombstone: bool,
    /// The C18.1 `storage_warning` value, when one applies.
    pub(super) warning: Option<&'static str>,
}

/// The shared session state of one auth host. Held across a refresh and a sign-out,
/// so the store is read and written under the lock.
pub struct HostSlot {
    pub(super) state: tokio::sync::Mutex<HostState>,
}

impl HostSlot {
    /// A slot with no session in memory.
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: tokio::sync::Mutex::new(HostState::default()),
        })
    }

    /// The `storage_warning` of this host, or `None`.
    pub(super) async fn warning(&self) -> Option<&'static str> {
        self.state.lock().await.warning
    }
}

/// The operation epoch a sign-in step started under (CT-87). A sign-out or a Cancel
/// bumps the counter, which voids every step still in flight.
pub struct Epoch<'a> {
    counter: &'a AtomicU64,
    expected: u64,
}

impl<'a> Epoch<'a> {
    /// The fence of a step that started while the counter read `expected`.
    pub fn new(counter: &'a AtomicU64, expected: u64) -> Self {
        Self { counter, expected }
    }

    /// Whether no sign-out or Cancel ran since the step started.
    pub fn is_current(&self) -> bool {
        self.counter.load(Ordering::SeqCst) == self.expected
    }
}

/// The auth host, which keys the store. An auth base always has one.
pub(super) fn host_of(base: &Url) -> &str {
    base.host_str().unwrap_or_default()
}

/// Writes the rotated refresh token to the store and keeps the access token in memory.
pub(super) fn adopt(
    store: &dyn CredentialStore,
    host: &str,
    state: &mut HostState,
    signed: SignedIn,
) -> AccountSummary {
    let summary = AccountSummary {
        account_name: signed.account.account_name,
        display_name: signed.account.display_name,
    };
    let stored = Stored {
        refresh_token: signed.refresh_token,
        account_name: summary.account_name.clone(),
        display_name: summary.display_name.clone(),
    };
    match store.set(host, &stored) {
        Ok(()) => {
            state.session_only = None;
            state.tombstone = false;
            state.warning = None;
        }
        Err(_) => {
            state.session_only = Some(stored);
            state.tombstone = false;
            state.warning = Some("session_only");
        }
    }
    let until = Instant::now() + Duration::from_secs(signed.expires_in);
    state.access = Some((signed.access_token, until));
    state.account = Some(summary.clone());
    summary
}

/// The stored sign-in for `host` as this slot sees it: a tombstone is `None`, a
/// sign-in the store refused comes before the store, and a store that cannot be
/// read is `None` with the `store_unreadable` warning.
pub(super) fn read(
    store: &dyn CredentialStore,
    host: &str,
    state: &mut HostState,
) -> Option<Stored> {
    if state.tombstone {
        return None;
    }
    if let Some(stored) = &state.session_only {
        return Some(stored.clone());
    }
    match store.get(host) {
        Ok(stored) => stored,
        Err(_) => {
            state.warning = Some("store_unreadable");
            None
        }
    }
}

/// Records the outcome of a delete: a refused delete leaves a tombstone, so the
/// next launch does not restore the account the player signed out of.
fn settle_delete(result: Result<(), StoreError>, state: &mut HostState) {
    state.session_only = None;
    match result {
        Ok(()) => {
            state.tombstone = false;
            state.warning = None;
        }
        Err(_) => {
            state.tombstone = true;
            state.warning = Some("sign_out_incomplete");
        }
    }
}

/// Clears the stored sign-in and the memory.
pub(super) fn forget(store: &dyn CredentialStore, host: &str, state: &mut HostState) {
    settle_delete(store.delete(host), state);
    state.access = None;
    state.account = None;
}

/// Clears the memory and the stored sign-in only if the store still holds
/// `used`, the refresh token this instance used. A newer sign-in stored since
/// (a reauth on the same host) is left alone.
pub(super) fn forget_if_stored(
    store: &dyn CredentialStore,
    host: &str,
    state: &mut HostState,
    used: &str,
) {
    if read(store, host, state).is_none_or(|now| now.refresh_token == used) {
        settle_delete(store.delete(host), state);
    }
    state.access = None;
    state.account = None;
}
