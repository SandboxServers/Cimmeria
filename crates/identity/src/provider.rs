//! The provider contract (contract C3): the ids and kinds the config names,
//! the verified identity a provider hands back, the errors a provider call
//! can fail with, and the [`IdentityProvider`] trait every provider implements.

use std::fmt;

/// A provider's stable id, as config names it and as the callback URL carries
/// it. Matches `^[a-z0-9_-]{1,32}$`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProviderId(String);

impl ProviderId {
    /// Returns the id when `raw` matches `^[a-z0-9_-]{1,32}$`, without a regex.
    pub fn new(raw: &str) -> Option<ProviderId> {
        let well_formed = (1..=32).contains(&raw.len())
            && raw
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-');
        well_formed.then(|| ProviderId(raw.to_owned()))
    }

    /// The id as the config and the callback path spell it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The three provider kinds the config's `kind` field accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    /// Discord's OAuth2 flow with its user endpoint.
    Discord,
    /// A generic OpenID Connect issuer, discovered from its `issuer` URL.
    Oidc,
    /// The developer-mode form that stands in for a real provider in lab UAT.
    DevMock,
}

impl ProviderKind {
    /// Parses a config `kind` value (`discord`, `oidc`, `dev_mock`).
    pub fn parse(raw: &str) -> Option<ProviderKind> {
        match raw {
            "discord" => Some(ProviderKind::Discord),
            "oidc" => Some(ProviderKind::Oidc),
            "dev_mock" => Some(ProviderKind::DevMock),
            _ => None,
        }
    }
}

/// The identity a provider verified for the person who just signed in.
///
/// `Debug` is written by hand so that the subject and the email never reach a
/// log line through `{:?}` (R7).
pub struct VerifiedIdentity {
    /// The provider that verified the identity.
    pub provider: ProviderId,
    /// Discord: `https://discord.com`. OIDC: the issuer URL.
    pub issuer: String,
    /// The provider's stable user id, 1-255 characters.
    pub subject: String,
    /// Sanitized with [`VerifiedIdentity::sanitized_display_name`], max 64.
    pub display_name: String,
    /// Lowercased. `None` unless the provider returned one.
    pub email: Option<String>,
    /// Whether the provider vouches for the email.
    pub email_verified: bool,
    /// When the provider account was created, in Unix milliseconds, if known.
    pub provider_account_created_ms: Option<i64>,
}

impl VerifiedIdentity {
    /// Keeps printable ASCII, trims, cuts to 64 characters, and falls back to
    /// `"Player"` when nothing is left.
    pub fn sanitized_display_name(raw: &str) -> String {
        let printable: String = raw
            .chars()
            .filter(|c| c.is_ascii_graphic() || *c == ' ')
            .collect();
        let cut: String = printable.trim().chars().take(64).collect();
        if cut.is_empty() {
            "Player".to_owned()
        } else {
            cut
        }
    }
}

impl fmt::Debug for VerifiedIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VerifiedIdentity")
            .field("provider", &self.provider)
            .field("issuer", &self.issuer)
            .finish_non_exhaustive()
    }
}

/// Why a provider call failed. Values become the `reason` span field; they
/// never carry a code, token, subject or email (C3).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderError {
    /// The user refused the sign-in at the provider.
    #[error("the provider denied the sign-in")]
    Denied,
    /// The provider did not answer within the client timeout.
    #[error("the provider timed out")]
    Timeout,
    /// The provider answered with this HTTP status.
    #[error("the provider answered HTTP {0}")]
    Http(u16),
    /// The provider's answer was not usable; the static string names the check.
    #[error("the provider's answer was unusable: {0}")]
    BadResponse(&'static str),
    /// The token exchange or its verification failed; the static string names the check.
    #[error("the token exchange failed: {0}")]
    Token(&'static str),
}

impl ProviderError {
    /// The short, fixed reason string for logs and span fields.
    pub fn reason(&self) -> &'static str {
        match self {
            ProviderError::Denied => "denied",
            ProviderError::Timeout => "timeout",
            ProviderError::Http(status) if (400..500).contains(status) => "http_4xx",
            ProviderError::Http(status) if (500..600).contains(status) => "http_5xx",
            ProviderError::Http(_) => "http_other",
            ProviderError::BadResponse(what) | ProviderError::Token(what) => what,
        }
    }
}

/// One configured identity provider (contract C3).
#[async_trait::async_trait]
pub trait IdentityProvider: Send + Sync {
    /// The config id, which is also the callback path segment.
    fn id(&self) -> &ProviderId;
    /// The name the login screen shows.
    fn display_name(&self) -> &str;
    /// Whether the provider's `email` claim may be trusted as verified.
    fn trusted_email(&self) -> bool;
    /// `redirect_uri` is `{public_base_url}/auth/v1/callback/{id}`.
    fn authorize_url(
        &self,
        state: &str,
        pkce_challenge: &str,
        nonce: &str,
        redirect_uri: &str,
    ) -> String;
    /// Trades the authorization code for a verified identity.
    async fn exchange(
        &self,
        code: &str,
        pkce_verifier: &str,
        nonce: &str,
        redirect_uri: &str,
    ) -> Result<VerifiedIdentity, ProviderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_id_pattern() {
        assert!(ProviderId::new("discord").is_some());
        assert!(ProviderId::new("my-idp_2").is_some());
        assert!(ProviderId::new("").is_none());
        assert!(ProviderId::new("Discord").is_none());
        assert!(ProviderId::new(&"a".repeat(33)).is_none());
        assert!(ProviderId::new(&"a".repeat(32)).is_some());
        assert!(ProviderId::new("a.b").is_none());
    }

    #[test]
    fn display_name_is_sanitized() {
        assert_eq!(
            VerifiedIdentity::sanitized_display_name("\u{1F600}"),
            "Player"
        );
        assert_eq!(VerifiedIdentity::sanitized_display_name("  Ada  "), "Ada");
        assert_eq!(
            VerifiedIdentity::sanitized_display_name(&"a".repeat(80)).len(),
            64
        );
    }

    #[test]
    fn reason_names_each_failure() {
        assert_eq!(ProviderError::Denied.reason(), "denied");
        assert_eq!(ProviderError::Timeout.reason(), "timeout");
        assert_eq!(ProviderError::Http(404).reason(), "http_4xx");
        assert_eq!(ProviderError::Http(503).reason(), "http_5xx");
        assert_eq!(
            ProviderError::BadResponse("too_large").reason(),
            "too_large"
        );
        assert_eq!(ProviderError::Token("bad_nonce").reason(), "bad_nonce");
    }
}
