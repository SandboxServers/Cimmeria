//! Sign-in through the server's identity service (contract C6 and C9).
//!
//! The launcher asks its login server where sign-in lives (`discovery`), then
//! runs the browser flow (`oauth`): it opens the provider's page in the
//! system browser and receives the redirect on a loopback listener. Only the
//! first login server in the list is offered sign-in (D-CT21); the other
//! servers keep the relay password. The ledger of decisions is in
//! `docs/analysis/client-tls-oidc/design.md`.

pub mod discovery;
pub mod oauth;

#[cfg(test)]
mod tests;

/// Why a sign-in attempt ended without a code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignInError {
    /// The sign-in server could not be reached, or the probe failed.
    Offline,
    /// The server does not offer sign-in (no `providers` route, or `api` is not 1).
    Unsupported,
    /// The player closed the sign-in before it finished.
    Cancelled,
    /// The provider refused the sign-in (`error=access_denied`).
    Denied,
    /// No redirect arrived within the sign-in timeout.
    TimedOut,
    /// The redirect carried a state other than the one this launch sent.
    StateMismatch,
    /// The server answered with this C6 error code.
    Server(String),
    /// The reply or the requested URL is not one the launcher will follow.
    Protocol,
}

impl SignInError {
    /// A short text for the sign-in panel. It never carries a code or a token.
    pub fn reason(&self) -> &'static str {
        match self {
            SignInError::Offline => "Could not reach the sign-in server.",
            SignInError::Unsupported => "This server does not offer sign-in.",
            SignInError::Cancelled => "Sign-in was cancelled.",
            SignInError::Denied => "The provider refused the sign-in.",
            SignInError::TimedOut => "Sign-in timed out. Try again.",
            SignInError::StateMismatch => "The sign-in reply did not match this launch.",
            SignInError::Server(_) => "The sign-in server refused the request.",
            SignInError::Protocol => "The sign-in server sent a reply the launcher does not use.",
        }
    }
}
