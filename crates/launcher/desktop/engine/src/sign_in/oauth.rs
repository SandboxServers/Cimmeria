//! The browser half of sign-in (C9): PKCE, the loopback redirect and the code.
//!
//! The launcher makes a PKCE pair of its own (verifier 48 random bytes, S256
//! challenge) and a `state` of 32 random bytes. It opens `{base}/auth/v1/authorize`
//! in the system browser, waits up to five minutes for the redirect to its
//! loopback listener, and hands the code and verifier to `/token` (C6).
//! Adding a sign-in method uses [`prepare`] first, because `link/start` needs the
//! listener's `redirect_uri`, the `state` and the challenge before it answers with
//! the page to open.

use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use cimmeria_launcher_relay::callback::{CallbackResult, CallbackServer};
use sha2::{Digest, Sha256};
use url::Url;

use super::SignInError;

/// How long the player has to finish the sign-in in the browser.
pub const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// A PKCE pair: the secret verifier and its S256 challenge (RFC 7636).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

/// A fresh PKCE pair. The verifier is 48 random bytes, base64url without padding.
pub fn new_pkce() -> Pkce {
    let verifier = random_b64(48);
    let challenge = challenge_for(&verifier);
    Pkce {
        verifier,
        challenge,
    }
}

/// The S256 challenge for a verifier: base64url of its SHA-256, without padding.
pub fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// A fresh `state` value: 32 random bytes, base64url without padding.
pub fn new_state() -> String {
    random_b64(32)
}

fn random_b64(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    getrandom::fill(&mut bytes).expect("the OS random source is available");
    URL_SAFE_NO_PAD.encode(bytes)
}

/// A sign-in in progress: the URL to open and the listener that waits for the redirect.
pub struct BrowserFlow {
    /// The page the system browser opens.
    pub authorize_url: String,
    server: CallbackServer,
    state: String,
    verifier: String,
}

/// What `/token` needs after a successful redirect (C6 `authorization_code`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeGrant {
    pub code: String,
    pub verifier: String,
    pub redirect_uri: String,
}

/// A sign-in whose loopback listener is bound and whose PKCE pair and `state`
/// exist, before the browser page is chosen. A plain sign-in calls
/// [`PendingFlow::authorize`]; adding a method sends [`PendingFlow::redirect_uri`],
/// [`PendingFlow::state`] and [`PendingFlow::code_challenge`] to `link/start` (C6)
/// and hands its `authorize_url` to [`PendingFlow::with_intent`].
pub struct PendingFlow {
    server: CallbackServer,
    state: String,
    pkce: Pkce,
}

/// Binds the loopback listener and makes the PKCE pair and `state`.
pub async fn prepare() -> Result<PendingFlow, SignInError> {
    let server = CallbackServer::bind().await.map_err(|e| {
        tracing::debug!(kind = ?e.kind(), "sign-in: the loopback listener did not bind");
        SignInError::Protocol
    })?;
    Ok(PendingFlow {
        server,
        state: new_state(),
        pkce: new_pkce(),
    })
}

impl PendingFlow {
    /// `http://127.0.0.1:<port>/cb`, the listener this flow waits on.
    pub fn redirect_uri(&self) -> String {
        self.server.redirect_uri()
    }

    /// The `state` the redirect must carry.
    pub fn state(&self) -> &str {
        &self.state
    }

    /// The S256 challenge of this flow's verifier.
    pub fn code_challenge(&self) -> &str {
        &self.pkce.challenge
    }

    /// A plain sign-in: `{base}auth/v1/authorize?..` with this flow's values.
    pub fn authorize(self, base: &Url, provider: &str) -> Result<BrowserFlow, SignInError> {
        let redirect_uri = self.redirect_uri();
        let mut url = base
            .join("auth/v1/authorize")
            .map_err(|_| SignInError::Protocol)?;
        url.query_pairs_mut()
            .append_pair("provider", provider)
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("state", &self.state)
            .append_pair("code_challenge", &self.pkce.challenge)
            .append_pair("code_challenge_method", "S256");
        Ok(self.into_flow(url.into()))
    }

    /// Adding a method: `authorize_url` is the `link/start` reply, which the server
    /// built from this flow's values. It must begin with this server's
    /// `/auth/v1/authorize?`; anything else gives [`SignInError::Protocol`].
    pub fn with_intent(
        self,
        base: &Url,
        authorize_url: String,
    ) -> Result<BrowserFlow, SignInError> {
        let prefix = format!("{base}auth/v1/authorize?");
        if !authorize_url.starts_with(&prefix) {
            return Err(SignInError::Protocol);
        }
        Ok(self.into_flow(authorize_url))
    }

    fn into_flow(self, authorize_url: String) -> BrowserFlow {
        BrowserFlow {
            authorize_url,
            server: self.server,
            state: self.state,
            verifier: self.pkce.verifier,
        }
    }
}

/// Starts a plain sign-in with `provider` on the server at `base`: [`prepare`],
/// then [`PendingFlow::authorize`].
pub async fn begin(base: &Url, provider: &str) -> Result<BrowserFlow, SignInError> {
    prepare().await?.authorize(base, provider)
}

impl BrowserFlow {
    /// Waits for the redirect (up to [`SIGN_IN_TIMEOUT`]) and returns the code grant.
    pub async fn finish(self) -> Result<CodeGrant, SignInError> {
        let redirect_uri = self.server.redirect_uri();
        match self.server.wait(&self.state, SIGN_IN_TIMEOUT).await {
            CallbackResult::Code(code) => Ok(CodeGrant {
                code,
                verifier: self.verifier,
                redirect_uri,
            }),
            CallbackResult::Denied => Err(SignInError::Denied),
            CallbackResult::StateMismatch => Err(SignInError::StateMismatch),
            CallbackResult::TimedOut => Err(SignInError::TimedOut),
        }
    }
}
