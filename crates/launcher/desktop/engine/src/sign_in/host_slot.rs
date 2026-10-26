//! The session state one auth host keeps (CT-86, F08).
//!
//! Every [`SignIn`](super::session::SignIn) the driver makes for one host shares
//! one [`HostSlot`], so a refresh and a sign-out on any of them serialize on one
//! lock and one refresh token is spent once. The store helpers take what they
//! use, so the rules for the stored sign-in change in one place.

use std::sync::Arc;
use std::time::{Duration, Instant};

use url::Url;

use super::api::SignedIn;
use super::session::AccountSummary;
use super::store::{CredentialStore, Stored};

/// What one auth host's sessions share: the access token and the account.
#[derive(Default)]
pub(super) struct HostState {
    /// The access token and the instant it stops being valid.
    pub(super) access: Option<(String, Instant)>,
    pub(super) account: Option<AccountSummary>,
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
    store.set(
        host,
        &Stored {
            refresh_token: signed.refresh_token,
            account_name: summary.account_name.clone(),
            display_name: summary.display_name.clone(),
        },
    );
    let until = Instant::now() + Duration::from_secs(signed.expires_in);
    state.access = Some((signed.access_token, until));
    state.account = Some(summary.clone());
    summary
}

/// Clears the stored sign-in and the memory.
pub(super) fn forget(store: &dyn CredentialStore, host: &str, state: &mut HostState) {
    store.delete(host);
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
    if store.get(host).is_none_or(|now| now.refresh_token == used) {
        store.delete(host);
    }
    state.access = None;
    state.account = None;
}
