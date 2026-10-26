//! The launcher's sign-in session (C6 and C9).
//!
//! The refresh token lives in the [`CredentialStore`] and rotates on every
//! refresh. The access token lives only in memory, for 15 minutes at most, and is
//! refreshed when less than a minute of it remains. Every request goes through
//! the one HTTP client that [`discovery::client`] builds, so redirects stay off
//! and every reply is read through [`discovery::read_capped`].

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use reqwest::header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Method;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use url::Url;

use super::api::{
    AccountView, ApiErrorBody, LinkStart, NeedsLink, SignedIn, TicketView, TokenReply,
};
use super::discovery::{self, MAX_BODY};
pub use super::host_slot::HostSlot;
use super::host_slot::{self, host_of, Epoch, HostState};
use super::oauth::{self, BrowserFlow, CodeGrant};
use super::store::CredentialStore;
use super::SignInError;

/// An access token is reused while more than this remains.
const REFRESH_MARGIN: Duration = Duration::from_secs(60);

/// The account's names, as the sign-in panel shows them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountSummary {
    pub account_name: String,
    pub display_name: String,
}

/// A provider identity with no account yet (C6 `needs_link`). Its `Debug` hides the grant.
#[derive(Clone, PartialEq, Eq)]
pub struct NeedsLinkView {
    /// The single-use grant the link calls take. It is never shown or logged.
    pub link_grant: String,
    pub provider: String,
    pub display_name: String,
    /// `none` or `email_match`: whether the server saw a likely existing account.
    pub hint: String,
    pub signup_open: bool,
}

impl fmt::Debug for NeedsLinkView {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NeedsLinkView")
            .field("link_grant", &"<hidden>")
            .field("provider", &self.provider)
            .field("hint", &self.hint)
            .field("signup_open", &self.signup_open)
            .finish()
    }
}

impl From<NeedsLink> for NeedsLinkView {
    fn from(link: NeedsLink) -> Self {
        Self {
            link_grant: link.link_grant,
            provider: link.identity.provider,
            display_name: link.identity.display_name,
            hint: link.hint,
            signup_open: link.signup_open,
        }
    }
}

/// How a sign-in step ended: a session, or a link the player must choose.
#[derive(Debug)]
pub enum Outcome {
    SignedIn(AccountSummary),
    NeedsLink(NeedsLinkView),
}

/// One launcher's sign-in against one auth host. Its state is the host's
/// [`HostSlot`], shared with every other sign-in for that host: a refresh and a
/// sign-out serialize on the slot's lock.
pub struct SignIn {
    base: Url,
    http: reqwest::Client,
    store: Arc<dyn CredentialStore>,
    slot: Arc<HostSlot>,
}

impl SignIn {
    /// A session for the auth server at `base` (`https://<host>/`), with no stored sign-in yet.
    /// It has a slot of its own.
    pub fn new(base: Url, http: reqwest::Client, store: Arc<dyn CredentialStore>) -> Self {
        Self::new_in(base, http, store, HostSlot::new())
    }

    /// A session for `base` on `slot`, the host's shared state.
    pub fn new_in(
        base: Url,
        http: reqwest::Client,
        store: Arc<dyn CredentialStore>,
        slot: Arc<HostSlot>,
    ) -> Self {
        Self {
            base,
            http,
            store,
            slot,
        }
    }

    /// The session a stored entry for `base` gives, with no network call. `None`
    /// when nothing is stored. The access token is fetched on first use. It has a
    /// slot of its own.
    pub fn restore(
        base: Url,
        http: reqwest::Client,
        store: Arc<dyn CredentialStore>,
    ) -> Option<Self> {
        let stored = store.get(host_of(&base))?;
        let account = AccountSummary {
            account_name: stored.account_name,
            display_name: stored.display_name,
        };
        let slot = Arc::new(HostSlot {
            state: tokio::sync::Mutex::new(HostState {
                access: None,
                account: Some(account),
            }),
        });
        Some(Self::new_in(base, http, store, slot))
    }

    /// The session a stored entry for `base` gives on `slot`, under the slot's
    /// lock. A stored entry sets the slot's account when it is empty; an access
    /// token already in the slot is kept. `None` when nothing is stored.
    pub async fn restore_in(
        base: Url,
        http: reqwest::Client,
        store: Arc<dyn CredentialStore>,
        slot: Arc<HostSlot>,
    ) -> Option<Self> {
        {
            let mut state = slot.state.lock().await;
            let stored = store.get(host_of(&base))?;
            if state.account.is_none() {
                state.account = Some(AccountSummary {
                    account_name: stored.account_name,
                    display_name: stored.display_name,
                });
            }
        }
        Some(Self::new_in(base, http, store, slot))
    }

    /// The auth server this session signs in to (`https://<host>/`).
    pub fn base(&self) -> &Url {
        &self.base
    }

    /// The names of the signed-in account, as last stored or refreshed. `None` when signed out.
    pub async fn summary(&self) -> Option<AccountSummary> {
        self.slot.state.lock().await.account.clone()
    }

    /// Exchanges the browser grant for a session (C6 `authorization_code`). A
    /// stale `fence` gives `Superseded` and stores nothing.
    pub async fn complete(
        &self,
        grant: CodeGrant,
        fence: &Epoch<'_>,
    ) -> Result<Outcome, SignInError> {
        let body = json!({
            "grant_type": "authorization_code",
            "code": grant.code,
            "code_verifier": grant.verifier,
            "redirect_uri": grant.redirect_uri,
        });
        let result = match self.json(Method::POST, "token", None, Some(body)).await {
            Ok(TokenReply::SignedIn(signed)) => self.adopt_signed_in(signed, fence).await,
            Ok(TokenReply::NeedsLink(link)) => Ok(Outcome::NeedsLink(link.into())),
            Err(e) => Err(e),
        };
        record(result)
    }

    /// Creates a new account for a `needs_link` grant (C6 `POST /link/new`).
    pub async fn link_new(
        &self,
        link_grant: &str,
        fence: &Epoch<'_>,
    ) -> Result<Outcome, SignInError> {
        let body = json!({ "link_grant": link_grant });
        let result = self.link_call("link/new", body, fence).await;
        record(result)
    }

    /// Links the grant to an existing account by name and password (C6 `POST /link/existing`).
    pub async fn link_existing_password(
        &self,
        link_grant: &str,
        account_name: &str,
        password: &str,
        fence: &Epoch<'_>,
    ) -> Result<Outcome, SignInError> {
        let body = json!({
            "link_grant": link_grant,
            "account_name": account_name,
            "password": password,
            "password_removal_ack": true,
        });
        let result = self.link_call("link/existing", body, fence).await;
        record(result)
    }

    /// Links the grant to an existing account with a GM link code (C6 `POST /link/existing`).
    pub async fn link_existing_code(
        &self,
        link_grant: &str,
        code: &str,
        fence: &Epoch<'_>,
    ) -> Result<Outcome, SignInError> {
        let body = json!({
            "link_grant": link_grant,
            "link_code": code,
            "password_removal_ack": true,
        });
        let result = self.link_call("link/existing", body, fence).await;
        record(result)
    }

    /// A bearer token good for more than a minute: the cached one, or a refresh.
    /// A refused refresh (`invalid_grant`) clears the stored sign-in and gives
    /// `Server("signed_out")`.
    pub async fn access_token(&self) -> Result<String, SignInError> {
        let mut state = self.slot.state.lock().await;
        if let Some((token, until)) = &state.access {
            if until.saturating_duration_since(Instant::now()) > REFRESH_MARGIN {
                return Ok(token.clone());
            }
        }
        let stored = self.store.get(host_of(&self.base)).ok_or_else(signed_out)?;
        let body = json!({
            "grant_type": "refresh_token",
            "refresh_token": stored.refresh_token,
        });
        match self.json(Method::POST, "token", None, Some(body)).await {
            Ok(TokenReply::SignedIn(signed)) => {
                let token = signed.access_token.clone();
                self.adopt(&mut state, signed);
                Ok(token)
            }
            Ok(TokenReply::NeedsLink(_)) => Err(SignInError::Protocol),
            Err(SignInError::Server(code)) if code == "invalid_grant" => {
                tracing::info!(target: "launcher.sign_in", event = "failed", outcome = "failed", reason = "signed_out");
                host_slot::forget_if_stored(
                    &*self.store,
                    host_of(&self.base),
                    &mut state,
                    &stored.refresh_token,
                );
                Err(signed_out())
            }
            Err(e) => Err(e),
        }
    }

    /// The account and its linked methods (C6 `GET /account`).
    pub async fn account(&self) -> Result<AccountView, SignInError> {
        self.json_authed(Method::GET, "account", None).await
    }

    /// Removes one linked method (C6 `DELETE /identities/{identity_id}`).
    pub async fn unlink(&self, identity_id: i64) -> Result<(), SignInError> {
        let path = format!("identities/{identity_id}");
        self.send_authed(Method::DELETE, &path, None)
            .await
            .map(drop)
    }

    /// Signs out. The stored sign-in and the memory are cleared first, under the
    /// slot's lock, so no refresh can spend the token after the server hears of
    /// it. Then the server is told when it can be reached (C6 `POST /logout`).
    pub async fn sign_out(&self) {
        let stored = {
            let mut state = self.slot.state.lock().await;
            let stored = self.store.get(host_of(&self.base));
            host_slot::forget(&*self.store, host_of(&self.base), &mut state);
            stored
        };
        if let Some(stored) = stored {
            let body = json!({ "refresh_token": stored.refresh_token });
            if self
                .send(Method::POST, "logout", None, Some(body))
                .await
                .is_err()
            {
                tracing::debug!("sign-in: the server could not be told of the sign-out");
            }
        }
    }

    /// Adds a sign-in method to the signed-in account (C6 `POST /link/start`).
    ///
    /// The listener is bound first, because `link/start` needs its `redirect_uri`,
    /// the `state` and the challenge, and the server echoes them into the page
    /// it returns. Any error drops the listener.
    pub async fn add_method(&self, provider: &str) -> Result<BrowserFlow, SignInError> {
        let pending = oauth::prepare().await?;
        let body = json!({
            "provider": provider,
            "redirect_uri": pending.redirect_uri(),
            "state": pending.state(),
            "code_challenge": pending.code_challenge(),
            "code_challenge_method": "S256",
        });
        let start: LinkStart = self
            .json_authed(Method::POST, "link/start", Some(body))
            .await?;
        pending.with_intent(&self.base, start.authorize_url)
    }

    /// A launch ticket for `launch_id` (C6 `POST /launch-ticket`).
    pub async fn launch_ticket(&self, launch_id: &str) -> Result<TicketView, SignInError> {
        let body = json!({ "launch_id": launch_id });
        self.json_authed(Method::POST, "launch-ticket", Some(body))
            .await
    }

    /// Keeps a launch alive. `Ok(false)` when the server no longer knows it (C6 `404`).
    pub async fn heartbeat(&self, launch_id: &str) -> Result<bool, SignInError> {
        let body = json!({ "launch_id": launch_id });
        match self
            .send_authed(Method::POST, "launch-ticket/heartbeat", Some(body))
            .await
        {
            Ok(_) => Ok(true),
            Err(SignInError::Server(code)) if code == "not_found" || code == "404" => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Ends a launch. The server answers the same when it is already gone (C6 `204`).
    pub async fn end_ticket(&self, launch_id: &str) -> Result<(), SignInError> {
        let body = json!({ "launch_id": launch_id });
        self.send_authed(Method::POST, "launch-ticket/end", Some(body))
            .await
            .map(drop)
    }

    /// The link calls share one reply shape: a `signed_in` body (C6).
    async fn link_call(
        &self,
        path: &str,
        body: Value,
        fence: &Epoch<'_>,
    ) -> Result<Outcome, SignInError> {
        let signed: SignedIn = self.json(Method::POST, path, None, Some(body)).await?;
        self.adopt_signed_in(signed, fence).await
    }

    /// Stores a new session from a `signed_in` reply and returns its account. When
    /// a sign-out or Cancel ran meanwhile, nothing is stored: the server is told to
    /// end the session just received (best effort) and the step is `Superseded`.
    async fn adopt_signed_in(
        &self,
        signed: SignedIn,
        fence: &Epoch<'_>,
    ) -> Result<Outcome, SignInError> {
        let mut state = self.slot.state.lock().await;
        if !fence.is_current() {
            drop(state);
            let body = json!({ "refresh_token": signed.refresh_token });
            if self
                .send(Method::POST, "logout", None, Some(body))
                .await
                .is_err()
            {
                tracing::debug!("sign-in: the server could not be told of the sign-out");
            }
            return Err(SignInError::Superseded);
        }
        let summary = self.adopt(&mut state, signed);
        Ok(Outcome::SignedIn(summary))
    }

    /// Writes the rotated refresh token to the store and keeps the access token in memory.
    fn adopt(&self, state: &mut HostState, signed: SignedIn) -> AccountSummary {
        host_slot::adopt(&*self.store, host_of(&self.base), state, signed)
    }

    /// A Bearer request. The server keeps access tokens in memory only, so a restart
    /// voids the cached one: on `invalid_token` the cached token is dropped, a
    /// refresh runs once and the request is sent once more.
    async fn send_authed(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<reqwest::Response, SignInError> {
        let token = self.access_token().await?;
        match self
            .send(method.clone(), path, Some(&token), body.clone())
            .await
        {
            Err(SignInError::Server(code)) if code == "invalid_token" => {
                let mut state = self.slot.state.lock().await;
                if state
                    .access
                    .as_ref()
                    .is_some_and(|(cached, _)| *cached == token)
                {
                    state.access = None;
                }
                drop(state);
                let fresh = self.access_token().await?;
                self.send(method, path, Some(&fresh), body).await
            }
            other => other,
        }
    }

    /// [`Self::send_authed`], then the body read as `T`.
    async fn json_authed<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Result<T, SignInError> {
        let response = self.send_authed(method, path, body).await?;
        let bytes = discovery::read_capped(response, MAX_BODY)
            .await
            .ok_or(SignInError::Protocol)?;
        serde_json::from_slice(&bytes).map_err(|_| SignInError::Protocol)
    }

    /// Sends one request and reads its body as `T`.
    async fn json<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        bearer: Option<&str>,
        body: Option<Value>,
    ) -> Result<T, SignInError> {
        let response = self.send(method, path, bearer, body).await?;
        let bytes = discovery::read_capped(response, MAX_BODY)
            .await
            .ok_or(SignInError::Protocol)?;
        serde_json::from_slice(&bytes).map_err(|_| SignInError::Protocol)
    }

    /// Sends one request under `auth/v1/`. A non-success reply becomes its C6 error.
    async fn send(
        &self,
        method: Method,
        path: &str,
        bearer: Option<&str>,
        body: Option<Value>,
    ) -> Result<reqwest::Response, SignInError> {
        if !scheme_allowed(&self.base, cfg!(any(test, debug_assertions))) {
            return Err(SignInError::Protocol);
        }
        let url = self
            .base
            .join(&format!("auth/v1/{path}"))
            .map_err(|_| SignInError::Protocol)?;
        let mut request = self
            .http
            .request(method, url)
            .header(ACCEPT, "application/json");
        if let Some(token) = bearer {
            request = request.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(body) = body {
            request = request
                .header(CONTENT_TYPE, "application/json")
                .body(body.to_string());
        }
        let response = request.send().await.map_err(|_| SignInError::Offline)?;
        if response.status().is_success() {
            return Ok(response);
        }
        Err(server_error(response).await)
    }
}

/// Tokens and passwords go over TLS only. Debug builds and tests may use a plain
/// local server (`CIMMERIA_LAUNCHER_AUTH_BASE`, `wiremock`).
pub(super) fn scheme_allowed(base: &Url, relaxed: bool) -> bool {
    base.scheme() == "https" || relaxed
}

fn signed_out() -> SignInError {
    SignInError::Server("signed_out".to_owned())
}

/// The C6 error of a non-success reply. A `429` keeps `retry_after` in the text,
/// for the sign-in panel. A reply with no readable error body gives its status.
async fn server_error(response: reqwest::Response) -> SignInError {
    let status = response.status();
    let body = discovery::read_capped(response, MAX_BODY)
        .await
        .and_then(|bytes| serde_json::from_slice::<ApiErrorBody>(&bytes).ok());
    match body {
        Some(body) if body.error == "rate_limited" => SignInError::Server(format!(
            "rate_limited retry_after={}",
            body.retry_after.unwrap_or(0)
        )),
        Some(body) => SignInError::Server(body.error),
        None => SignInError::Server(status.as_u16().to_string()),
    }
}

/// Logs how a sign-in step ended: the outcome, or the error's fixed reason text.
/// No code, token, grant or account value goes into the log.
fn record(result: Result<Outcome, SignInError>) -> Result<Outcome, SignInError> {
    match &result {
        Ok(Outcome::SignedIn(_)) => {
            tracing::info!(target: "launcher.sign_in", event = "completed", outcome = "signed_in")
        }
        Ok(Outcome::NeedsLink(_)) => {
            tracing::info!(target: "launcher.sign_in", event = "completed", outcome = "needs_link")
        }
        Err(e) => {
            tracing::info!(target: "launcher.sign_in", event = "failed", outcome = "failed", reason = e.reason())
        }
    }
    result
}
