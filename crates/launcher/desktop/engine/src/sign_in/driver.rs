//! The sign-in command driver (C13). It runs one `sign_in_command` request
//! against the login server's sign-in service and the desktop state, and gives
//! the reply the shell sends back.
//!
//! The shell holds one [`SignInDriver`] for the whole run. It keeps the discovery
//! cache, the `needs_link` choice the player has not finished, and the one browser
//! flow that may run. No `DesktopState` or driver lock is held across an `.await`.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use url::Url;

use super::commands::{SignInCommand, SignInFailure, SignInStatus};
use super::discovery::{self, Discovery, DiscoveryCache};
use super::flow::{Continue, FlowSlot};
use super::host_slot::{host_of, Epoch};
use super::oauth;
use super::pending::{is_retryable, PendingLink, PendingSlot};
use super::session::{HostSlot, Outcome, SignIn};
use super::store::{self, CredentialStore};
use super::SignInError;
use crate::client_setup::login_servers::LoginServer;
use crate::storage::DesktopState;

/// Opens a page in the system browser. `Err(())` when the browser could not be started.
pub trait Browser: Send + Sync {
    /// Opens `url`, or says it could not.
    #[allow(clippy::result_unit_err)] // the packet's C13 signature; the caller only needs success or not
    fn open(&self, url: &str) -> Result<(), ()>;
}

/// The sign-in driver the shell holds for the whole launcher run.
pub struct SignInDriver {
    pub(super) store: Arc<dyn CredentialStore>,
    pub(super) http: reqwest::Client,
    discovery: Mutex<DiscoveryCache>,
    pub(super) pending: PendingSlot,
    /// The browser flow that may run, and its cancel handle.
    pub(super) flow: FlowSlot,
    /// The shared session state of each auth host, keyed by [`host_of`]. Kept once made.
    hosts: Mutex<HashMap<String, Arc<HostSlot>>>,
    /// The operation epoch (CT-87): a sign-out or a Cancel bumps it, which voids
    /// every sign-in step still in flight.
    pub(super) epoch: AtomicU64,
}

/// The store a launcher session uses: the OS credential store where there is one.
pub fn default_store() -> Arc<dyn CredentialStore> {
    store::platform_store()
}

/// What a reply names: the login server (none when the launcher has none) and
/// its discovery (none while the server cannot be reached).
pub(super) struct Site<'a> {
    pub(super) state: &'a Mutex<DesktopState>,
    pub(super) server: Option<&'a str>,
    pub(super) discovery: Option<&'a Discovery>,
    /// The auth base the target resolves to (CT-88), whose store state the reply shows.
    pub(super) base: Option<&'a Url>,
}

/// How a `needs_link` choice is completed.
enum Link<'a> {
    New,
    Password { account: &'a str, password: &'a str },
    Code(&'a str),
}

impl SignInDriver {
    /// A driver with an empty cache and no pending choice. `store` keeps the sign-in.
    pub fn new(store: Arc<dyn CredentialStore>) -> Self {
        Self {
            store,
            http: discovery::client(),
            discovery: Mutex::new(DiscoveryCache::default()),
            pending: PendingSlot::new(),
            flow: FlowSlot::new(),
            hosts: Mutex::new(HashMap::new()),
            epoch: AtomicU64::new(0),
        }
    }

    /// The slot every session for `base`'s host shares. Made on first use, kept after.
    pub(super) fn slot(&self, base: &Url) -> Arc<HostSlot> {
        let mut hosts = self.hosts.lock().unwrap_or_else(|e| e.into_inner());
        hosts
            .entry(host_of(base).to_owned())
            .or_insert_with(HostSlot::new)
            .clone()
    }

    /// Runs one request. The reply is the status, or the failure the panel shows.
    pub async fn run(
        &self,
        state: &Mutex<DesktopState>,
        request: SignInCommand,
        browser: &dyn Browser,
    ) -> Result<SignInStatus, SignInFailure> {
        if request.schema_version() != 1 {
            return Err(SignInFailure::UnsupportedSchema);
        }
        // The epoch this request runs under: a sign-out or Cancel that comes later voids it.
        let epoch = self.epoch.load(Ordering::SeqCst);
        if matches!(request, SignInCommand::Cancel { .. }) {
            self.supersede(state)?;
        }
        let listed = guard(state)?.sign_in_login_server();
        let server = match listed {
            Ok(server) => server,
            Err(_) => {
                tracing::debug!(
                    target: "launcher.sign_in",
                    reason = "installation_unreadable",
                    "sign-in server list unreadable"
                );
                // Sign out and Cancel must end a sign-in even while the installed copy is busy.
                if matches!(
                    request,
                    SignInCommand::Cancel { .. } | SignInCommand::SignOut { .. }
                ) {
                    None
                } else {
                    return Err(SignInFailure::Unavailable);
                }
            }
        };
        let Some(server) = server else {
            return match request {
                SignInCommand::Inspect { .. } | SignInCommand::Cancel { .. } => {
                    let site = Site {
                        state,
                        server: None,
                        discovery: None,
                        base: None,
                    };
                    self.reply(&site, None, None).await
                }
                SignInCommand::SignOut { .. } => {
                    let site = Site {
                        state,
                        server: None,
                        discovery: None,
                        base: None,
                    };
                    let session = guard(state)?.sign_in();
                    self.sign_out(&site, session).await
                }
                _ => Err(SignInFailure::Unavailable),
            };
        };
        let Some(discovery) = self.discover(&server.url).await else {
            return self.offline(state, &server, request).await;
        };
        let session = self.session_for(state, &discovery.auth_base).await?;
        self.pending.drop_unless(&discovery.auth_base)?;
        let site = Site {
            state,
            server: Some(&server.name),
            discovery: Some(&discovery),
            base: Some(&discovery.auth_base),
        };
        match request {
            SignInCommand::Inspect { .. } | SignInCommand::Cancel { .. } => {
                self.reply(&site, session.as_ref(), None).await
            }
            SignInCommand::Begin { provider, .. } => {
                self.begin(&site, &discovery, browser, &provider, epoch)
                    .await
            }
            SignInCommand::LinkNew { .. } => self.link(&site, Link::New, epoch).await,
            SignInCommand::LinkExistingPassword {
                account_name,
                password,
                acknowledge_password_removal,
                ..
            } => {
                if !acknowledge_password_removal {
                    return Err(SignInFailure::PasswordRemovalNotAcknowledged);
                }
                let link = Link::Password {
                    account: &account_name,
                    password: &password,
                };
                self.link(&site, link, epoch).await
            }
            SignInCommand::LinkExistingCode {
                link_code,
                acknowledge_password_removal,
                ..
            } => {
                if !acknowledge_password_removal {
                    return Err(SignInFailure::PasswordRemovalNotAcknowledged);
                }
                self.link(&site, Link::Code(&link_code), epoch).await
            }
            SignInCommand::Account { .. } => {
                let session = session.ok_or_else(signed_out)?;
                self.account_reply(&site, &session).await
            }
            SignInCommand::AddMethod { provider, .. } => {
                self.add_method(&site, &discovery, browser, session, &provider, epoch)
                    .await
            }
            SignInCommand::Unlink { identity_id, .. } => {
                let session = session.ok_or_else(signed_out)?;
                session
                    .unlink(identity_id)
                    .await
                    .map_err(SignInFailure::from)?;
                self.account_reply(&site, &session).await
            }
            SignInCommand::SignOut { .. } => self.sign_out(&site, session).await,
            SignInCommand::Resume { .. } => {
                let (grant, cont, parked_epoch) = self.flow.resume().await?;
                self.continue_flow(&site, &discovery, grant, cont, parked_epoch)
                    .await
            }
        }
    }

    /// The discovery of `login_url`, through the cache. The probe reads the clock here.
    async fn discover(&self, login_url: &str) -> Option<Discovery> {
        let now = Instant::now();
        let cached = self.discovery.lock().ok()?.get(login_url, now);
        if let Some(hit) = cached {
            return Some(hit);
        }
        let found = discovery::probe(&self.http, login_url).await?;
        if let Ok(mut cache) = self.discovery.lock() {
            cache.insert(login_url, now, found.clone());
        }
        Some(found)
    }

    /// The reply when the server cannot be reached. A stored sign-in is still
    /// restored from the credential store, so Play knows the player is signed
    /// in (ruling CT-35-1). Only `inspect`, `cancel` and `sign_out` work here.
    async fn offline(
        &self,
        state: &Mutex<DesktopState>,
        server: &LoginServer,
        request: SignInCommand,
    ) -> Result<SignInStatus, SignInFailure> {
        // A target with no auth base gives no session, so the previous host's
        // session is dropped (F11): it must not show under the new server's name.
        let base = discovery::auth_base(&server.url);
        match &base {
            Some(base) => {
                self.session_for(state, base).await?;
            }
            None => guard(state)?.set_sign_in(None),
        }
        let session = guard(state)?.sign_in();
        let site = Site {
            state,
            server: Some(&server.name),
            discovery: None,
            base: base.as_ref(),
        };
        match request {
            SignInCommand::SignOut { .. } => self.sign_out(&site, session).await,
            SignInCommand::Inspect { .. } | SignInCommand::Cancel { .. } => {
                self.reply(&site, session.as_ref(), None).await
            }
            _ => Err(SignInFailure::Unavailable),
        }
    }

    /// The session for `base`: the one in the state when it is for that auth
    /// server, else the stored sign-in restored on the host's slot, which is then
    /// kept in the state. No `DesktopState` guard is held across the await.
    async fn session_for(
        &self,
        state: &Mutex<DesktopState>,
        base: &Url,
    ) -> Result<Option<Arc<SignIn>>, SignInFailure> {
        let current = guard(state)?.sign_in();
        if let Some(session) = current {
            if session.base() == base {
                return Ok(Some(session));
            }
        }
        let restored = SignIn::restore_in(
            base.clone(),
            self.http.clone(),
            self.store.clone(),
            self.slot(base),
        )
        .await
        .map(Arc::new);
        guard(state)?.set_sign_in(restored.clone());
        Ok(restored)
    }

    /// Starts a plain sign-in with `provider` and completes it with the grant.
    async fn begin(
        &self,
        site: &Site<'_>,
        discovery: &Discovery,
        browser: &dyn Browser,
        provider: &str,
        epoch: u64,
    ) -> Result<SignInStatus, SignInFailure> {
        if !discovery.providers.iter().any(|p| p.id == provider) {
            return Err(SignInError::Server("unknown_provider".to_owned()).into());
        }
        let base = discovery.auth_base.clone();
        let cont = Continue::Begin {
            base: base.clone(),
            provider: provider.to_owned(),
        };
        let (grant, cont, epoch) = self
            .flow
            .run(browser, oauth::begin(&base, provider), cont, epoch)
            .await?;
        self.continue_flow(site, discovery, grant, cont, epoch)
            .await
    }

    /// Adds a sign-in method to the signed-in account, then reads the account.
    async fn add_method(
        &self,
        site: &Site<'_>,
        discovery: &Discovery,
        browser: &dyn Browser,
        session: Option<Arc<SignIn>>,
        provider: &str,
        epoch: u64,
    ) -> Result<SignInStatus, SignInFailure> {
        let session = session.ok_or_else(signed_out)?;
        let cont = Continue::AddMethod {
            session: Arc::clone(&session),
            provider: provider.to_owned(),
        };
        let (grant, cont, epoch) = self
            .flow
            .run(browser, session.add_method(provider), cont, epoch)
            .await?;
        self.continue_flow(site, discovery, grant, cont, epoch)
            .await
    }

    /// Completes a `needs_link` choice. A retryable failure keeps the choice.
    async fn link(
        &self,
        site: &Site<'_>,
        link: Link<'_>,
        epoch: u64,
    ) -> Result<SignInStatus, SignInFailure> {
        let pending = self.pending.take()?.ok_or(SignInFailure::NoPendingLink)?;
        // The password or code goes to the server the choice came from, so a
        // server switch since then ends the choice.
        if site.discovery.map(|d| &d.auth_base) != Some(&pending.base) {
            return Err(SignInFailure::NoPendingLink);
        }
        let session = Arc::new(SignIn::new_in(
            pending.base.clone(),
            self.http.clone(),
            self.store.clone(),
            self.slot(&pending.base),
        ));
        let grant = pending.view.link_grant.as_str();
        let fence = Epoch::new(&self.epoch, epoch);
        let result = match link {
            Link::New => session.link_new(grant, &fence).await,
            Link::Password { account, password } => {
                session
                    .link_existing_password(grant, account, password, &fence)
                    .await
            }
            Link::Code(code) => session.link_existing_code(grant, code, &fence).await,
        };
        match result {
            Ok(Outcome::SignedIn(_)) => self.signed_in(site, &session, epoch).await,
            Ok(Outcome::NeedsLink(_)) => Err(SignInFailure::Protocol),
            Err(error) => {
                if is_retryable(&error) && self.epoch.load(Ordering::SeqCst) == epoch {
                    self.pending.keep(pending)?;
                }
                Err(error.into())
            }
        }
    }

    /// Stores the outcome of a browser flow: a session, or a choice to make.
    pub(super) async fn finish(
        &self,
        site: &Site<'_>,
        discovery: &Discovery,
        session: Arc<SignIn>,
        outcome: Outcome,
        provider: &str,
        epoch: u64,
    ) -> Result<SignInStatus, SignInFailure> {
        match outcome {
            Outcome::SignedIn(_) => self.signed_in(site, &session, epoch).await,
            Outcome::NeedsLink(link) => {
                let provider_name = discovery
                    .providers
                    .iter()
                    .find(|p| p.id == provider)
                    .map_or_else(|| provider.to_owned(), |p| p.display_name.clone());
                let choice = PendingLink {
                    view: link,
                    provider_name,
                    base: session.base().clone(),
                    since: Instant::now(),
                };
                let shown = self.keep_choice(site.state, &session, epoch, choice)?;
                self.reply(site, shown.as_ref(), None).await
            }
        }
    }

    /// Makes `session` the player's sign-in and drops any pending choice, unless a
    /// sign-out or Cancel ran since `epoch`, or the target moved on.
    async fn signed_in(
        &self,
        site: &Site<'_>,
        session: &Arc<SignIn>,
        epoch: u64,
    ) -> Result<SignInStatus, SignInFailure> {
        let shown = self.install(site.state, session, epoch)?;
        self.reply(site, shown.as_ref(), None).await
    }

    /// The status with the account's linked methods, read from the server.
    async fn account_reply(
        &self,
        site: &Site<'_>,
        session: &Arc<SignIn>,
    ) -> Result<SignInStatus, SignInFailure> {
        let view = session.account().await.map_err(SignInFailure::from)?;
        self.reply(site, Some(session), Some(view.into())).await
    }

    /// Signs out (it works offline), then drops the sign-in and any pending choice.
    /// The epoch moves first, so a sign-in step still in flight is voided.
    pub(super) async fn sign_out(
        &self,
        site: &Site<'_>,
        session: Option<Arc<SignIn>>,
    ) -> Result<SignInStatus, SignInFailure> {
        self.supersede(site.state)?;
        let session_base = session.as_ref().map(|s| s.base().clone());
        if let Some(session) = session {
            session.sign_out().await;
        }
        guard(site.state)?.set_sign_in(None);
        self.pending.clear()?;
        // The session is gone, so its base carries the warning when `site` has none.
        let site = Site {
            state: site.state,
            server: site.server,
            discovery: site.discovery,
            base: site.base.or(session_base.as_ref()),
        };
        self.reply(&site, None, None).await
    }
}

/// Locks a driver or state mutex, or says the lock is broken (`Io`).
pub(super) fn guard<T>(lock: &Mutex<T>) -> Result<MutexGuard<'_, T>, SignInFailure> {
    lock.lock().map_err(|_| SignInFailure::Io)
}

/// The failure for a command that needs a signed-in player.
fn signed_out() -> SignInFailure {
    SignInError::Server("signed_out".to_owned()).into()
}
