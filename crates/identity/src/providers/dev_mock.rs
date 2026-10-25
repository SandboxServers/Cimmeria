//! The developer-mode mock provider (contract C4). It stands in for a real
//! provider in lab UAT: the "authorize" page is one form this server serves,
//! and the codes it hands out are single-use and live for 60 seconds.

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngExt;

use super::encode_pairs;
use crate::config::ProviderConfig;
use crate::provider::{IdentityProvider, ProviderError, ProviderId, VerifiedIdentity};

/// How long an issued code stays valid.
const CODE_LIFE: Duration = Duration::from_secs(60);

/// The issuer the mock stamps on the identities it verifies.
const ISSUER: &str = "dev-mock";

/// The developer-mode mock. Only built when developer mode is on (C4).
pub struct DevMockProvider {
    id: ProviderId,
    display_name: String,
    trusted_email: bool,
    /// The public base URL without a trailing slash.
    base_url: String,
    /// Issued codes, each with the identity the form posted and its issue time.
    codes: Mutex<HashMap<String, (VerifiedIdentity, Instant)>>,
}

/// Hand-written so that `{:?}` never prints the live single-use codes (R7, C11).
impl std::fmt::Debug for DevMockProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DevMockProvider")
            .field("id", &self.id)
            .field("base_url", &self.base_url)
            .finish_non_exhaustive()
    }
}

impl DevMockProvider {
    /// Builds the mock from its config table; `public_base_url` is the site's base.
    pub fn new(cfg: &ProviderConfig, public_base_url: &str) -> DevMockProvider {
        DevMockProvider {
            id: cfg.id.clone(),
            display_name: cfg.display_name.clone(),
            trusted_email: cfg.trusted_email,
            base_url: public_base_url.trim_end_matches('/').to_owned(),
            codes: Mutex::new(HashMap::new()),
        }
    }

    /// Stores `identity` under a fresh code and returns the code. Codes issued
    /// more than 60 seconds ago are dropped on the way.
    pub fn issue(&self, identity: VerifiedIdentity) -> String {
        self.issue_at(identity, Instant::now())
    }

    /// The body of `issue`, with the clock passed in so tests can drive it.
    fn issue_at(&self, mut identity: VerifiedIdentity, now: Instant) -> String {
        identity.issuer = ISSUER.to_owned();
        let mut rng = rand::rng();
        let bytes: [u8; 32] = std::array::from_fn(|_| rng.random());
        let code = URL_SAFE_NO_PAD.encode(bytes);
        let mut codes = self.codes.lock().unwrap_or_else(PoisonError::into_inner);
        codes.retain(|_, (_, issued)| now.saturating_duration_since(*issued) < CODE_LIFE);
        codes.insert(code.clone(), (identity, now));
        code
    }

    /// The body of `exchange`, with the clock passed in so tests can drive it.
    fn take_at(&self, code: &str, now: Instant) -> Result<VerifiedIdentity, ProviderError> {
        // Removing the entry makes the code single-use, even when it is replayed.
        let taken = self
            .codes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(code);
        match taken {
            Some((identity, issued)) if now.saturating_duration_since(issued) < CODE_LIFE => {
                Ok(identity)
            }
            _ => Err(ProviderError::Token("invalid_grant")),
        }
    }
}

#[async_trait::async_trait]
impl IdentityProvider for DevMockProvider {
    fn id(&self) -> &ProviderId {
        &self.id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn trusted_email(&self) -> bool {
        self.trusted_email
    }

    fn authorize_url(
        &self,
        state: &str,
        _pkce_challenge: &str,
        _nonce: &str,
        redirect_uri: &str,
    ) -> String {
        let query = encode_pairs(&[("state", state), ("redirect_uri", redirect_uri)]);
        format!("{}/auth/v1/dev-mock/authorize?{query}", self.base_url)
    }

    async fn exchange(
        &self,
        code: &str,
        _pkce_verifier: &str,
        _nonce: &str,
        _redirect_uri: &str,
    ) -> Result<VerifiedIdentity, ProviderError> {
        self.take_at(code, Instant::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use crate::providers::build;

    /// A developer-mode config with one mock provider.
    const CONFIG: &str = r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "dev"
kind = "dev_mock"
display_name = "Developer sign-in"
client_id = "dev-client"
client_secret_env = "CIMMERIA_IDP_DEV_SECRET"
trusted_email = true
"#;

    /// The mock built through `build`, the way the server builds it.
    fn mock() -> std::sync::Arc<DevMockProvider> {
        let env = |name: &str| (name == "CIMMERIA_IDP_DEV_SECRET").then(|| "dev-secret".to_owned());
        let (cfg, _) = config::load(CONFIG, &env, true).expect("the fixture loads");
        build(&cfg, crate::http::provider_client())
            .dev_mock()
            .cloned()
            .expect("developer mode builds the mock")
    }

    fn identity() -> VerifiedIdentity {
        VerifiedIdentity {
            provider: ProviderId::new("dev").expect("a valid id"),
            issuer: ISSUER.to_owned(),
            subject: "dev-subject-1".to_owned(),
            display_name: "Player_1234".to_owned(),
            email: Some("player@example.org".to_owned()),
            email_verified: true,
            provider_account_created_ms: None,
        }
    }

    #[test]
    fn authorize_url_points_at_the_site() {
        let url = mock().authorize_url(
            "state-1",
            "challenge",
            "nonce",
            "https://play.example.org/cb",
        );
        assert!(url.starts_with("https://play.example.org/auth/v1/dev-mock/authorize?"));
        assert!(url.contains("state=state-1"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fplay.example.org%2Fcb"));
    }

    #[tokio::test]
    async fn dev_mock_codes_are_single_use() {
        let mock = mock();
        let code = mock.issue(identity());
        let first = mock
            .exchange(&code, "", "", "")
            .await
            .expect("the first use succeeds");
        assert_eq!(first.subject, "dev-subject-1");
        assert_eq!(
            mock.exchange(&code, "", "", "").await.err(),
            Some(ProviderError::Token("invalid_grant"))
        );
    }

    #[tokio::test]
    async fn dev_mock_refuses_an_unknown_code() {
        let mock = mock();
        assert_eq!(
            mock.exchange("never-issued", "", "", "").await.err(),
            Some(ProviderError::Token("invalid_grant"))
        );
    }

    #[test]
    fn dev_mock_debug_hides_codes() {
        let m = mock();
        let code = m.issue(identity());
        assert!(!format!("{m:?}").contains(&code));
    }

    #[test]
    fn dev_mock_codes_expire_after_sixty_seconds() {
        let m = mock();
        let t0 = Instant::now();
        let old = m.issue_at(identity(), t0);
        assert!(m.take_at(&old, t0 + Duration::from_secs(61)).is_err());
        let kept = m.issue_at(identity(), t0);
        assert!(m.take_at(&kept, t0 + Duration::from_secs(59)).is_ok());
        let stale = m.issue_at(identity(), t0);
        let _fresh = m.issue_at(identity(), t0 + Duration::from_secs(61));
        assert!(m.take_at(&stale, t0 + Duration::from_secs(61)).is_err());
    }

    #[test]
    fn issued_codes_are_long_and_distinct() {
        let mock = mock();
        let a = mock.issue(identity());
        let b = mock.issue(identity());
        assert_ne!(a, b);
        assert_eq!(a.len(), 43, "32 random bytes in base64url without padding");
    }
}
