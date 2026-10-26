//! The generic OpenID Connect provider (contract C3, C4). Discovery runs on
//! first use and is cached for an hour. The sign-in is the authorization-code
//! flow with PKCE and a nonce; the `id_token` is verified against the issuer's
//! keys before any identity comes back. Only `RS256` and `ES256` are accepted,
//! so `none` and the HMAC algorithms, which a client secret could sign, fail.

use std::fmt;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use base64::Engine;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::Deserialize;

use super::oidc_keys::{Discovery, KeyCache, CACHE_LIFETIME};
use crate::config::{ProviderConfig, Secret, TokenAuthMethod};
use crate::http::{read_capped, transport_error, PROVIDER_BODY_CAP};
use crate::provider::{IdentityProvider, ProviderError, ProviderId, VerifiedIdentity};

/// The clock skew the `exp` check allows, in seconds.
const LEEWAY_SECS: u64 = 60;

/// The longest subject the server stores, in characters.
const MAX_SUBJECT_CHARS: usize = 255;

/// One OpenID Connect issuer, configured by a `kind = "oidc"` provider table.
pub struct OidcProvider {
    /// The config id.
    id: ProviderId,
    /// The name the login screen shows.
    display_name: String,
    /// The issuer URL; the discovery document and the `iss` claim must match it.
    issuer: String,
    /// The OAuth client id registered at the issuer.
    client_id: String,
    /// The client secret, sent in the `Authorization` header or the form body,
    /// as `token_auth` says.
    client_secret: Secret,
    /// How the secret reaches the token endpoint.
    token_auth: TokenAuthMethod,
    /// The scopes asked for, joined with spaces on the authorize URL.
    scopes: Vec<String>,
    /// Whether the config says the issuer's `email` claim is trusted.
    trusted_email: bool,
    /// The shared provider client.
    http: reqwest::Client,
    /// The discovery document and when it was fetched.
    discovery: Mutex<Option<(Instant, Arc<Discovery>)>>,
    /// The issuer's signing keys.
    keys: KeyCache,
}

/// The fields of the token endpoint's answer the provider reads.
#[derive(Deserialize)]
struct TokenResponse {
    /// The signed identity token.
    id_token: String,
}

/// The `id_token` claims the provider reads. Fields the provider does not use
/// are ignored.
#[derive(Deserialize)]
struct Claims {
    /// The stable user id.
    sub: String,
    /// The nonce the sign-in sent.
    nonce: Option<String>,
    /// The user's email, when the issuer shares it.
    email: Option<String>,
    /// The verification flag; a bool or the strings `"true"`/`"false"`.
    email_verified: Option<serde_json::Value>,
    /// The display name, preferred over `preferred_username`.
    name: Option<String>,
    /// The fallback display name.
    preferred_username: Option<String>,
}

impl OidcProvider {
    /// Builds the provider from its config table. Nothing is fetched yet;
    /// discovery runs on the first [`IdentityProvider::prepare`] or exchange.
    pub fn new(cfg: &ProviderConfig, http: reqwest::Client) -> OidcProvider {
        OidcProvider {
            id: cfg.id.clone(),
            display_name: cfg.display_name.clone(),
            issuer: cfg.issuer.clone().unwrap_or_default(),
            client_id: cfg.client_id.clone(),
            client_secret: cfg.client_secret.clone(),
            token_auth: cfg.token_auth,
            scopes: cfg.scopes.clone(),
            trusted_email: cfg.trusted_email,
            http,
            discovery: Mutex::new(None),
            keys: KeyCache::default(),
        }
    }

    /// The discovery document: cached while fresh, fetched otherwise.
    async fn discovery(&self, now: Instant) -> Result<Arc<Discovery>, ProviderError> {
        let cached = self
            .discovery
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Some((_, doc)) = cached.filter(|(at, _)| now.duration_since(*at) < CACHE_LIFETIME) {
            return Ok(doc);
        }
        let doc = Arc::new(Discovery::fetch(&self.http, &self.issuer).await?);
        *self
            .discovery
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some((now, Arc::clone(&doc)));
        Ok(doc)
    }

    /// The discovery document as last cached, without fetching.
    fn cached_discovery(&self) -> Option<Arc<Discovery>> {
        self.discovery
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .map(|(_, doc)| Arc::clone(doc))
    }

    /// Refuses a configured token endpoint auth method the issuer does not
    /// list. Absent metadata is not refused.
    fn check_auth_method(&self, doc: &Discovery) -> Result<(), ProviderError> {
        match &doc.auth_methods {
            Some(list) if !list.iter().any(|m| m == self.token_auth.as_str()) => {
                tracing::warn!(
                    provider = %self.id.as_str(),
                    method = self.token_auth.as_str(),
                    reason = "auth_method_unsupported",
                    "oidc issuer does not list the configured token endpoint auth method"
                );
                Err(ProviderError::BadResponse("auth_method_unsupported"))
            }
            _ => Ok(()),
        }
    }

    /// Verifies the `id_token` and maps its claims to a verified identity.
    /// `nonce` is the one the sign-in sent; `now` is the clock the `exp`
    /// check reads.
    async fn verify(
        &self,
        id_token: &str,
        nonce: &str,
        now: Instant,
    ) -> Result<VerifiedIdentity, ProviderError> {
        let header = decode_header(id_token).map_err(|_| ProviderError::Token("bad_token"))?;
        if !matches!(header.alg, Algorithm::RS256 | Algorithm::ES256) {
            return Err(ProviderError::Token("bad_alg"));
        }
        let kid = header.kid.ok_or(ProviderError::Token("no_kid"))?;
        let doc = self.discovery(now).await?;
        let jwk = self.keys.key(&self.http, &doc.jwks_uri, &kid, now).await?;
        let key = DecodingKey::from_jwk(&jwk).map_err(|_| ProviderError::Token("bad_key"))?;

        let mut validation = Validation::new(header.alg);
        validation.leeway = LEEWAY_SECS;
        validation.set_audience(&[self.client_id.as_str()]);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        let claims = decode::<Claims>(id_token, &key, &validation)
            .map_err(|err| ProviderError::Token(claim_failure(err.kind())))?
            .claims;

        if claims.nonce.as_deref() != Some(nonce) {
            return Err(ProviderError::Token("bad_nonce"));
        }
        let subject_chars = claims.sub.chars().count();
        if !(1..=MAX_SUBJECT_CHARS).contains(&subject_chars) {
            return Err(ProviderError::Token("bad_sub"));
        }
        let email = claims
            .email
            .filter(|email| !email.is_empty())
            .map(|email| email.to_lowercase());
        let email_verified =
            email.is_some() && email_verified_claim(claims.email_verified.as_ref());
        let raw_name = claims
            .name
            .or(claims.preferred_username)
            .unwrap_or_default();
        Ok(VerifiedIdentity {
            provider: self.id.clone(),
            issuer: self.issuer.clone(),
            subject: claims.sub,
            display_name: VerifiedIdentity::sanitized_display_name(&raw_name),
            email,
            email_verified,
            provider_account_created_ms: None,
        })
    }
}

/// RFC 6749 2.3.1: each part form-urlencoded, then `id:secret` in standard base64.
fn basic_credentials(id: &str, secret: &str) -> String {
    let enc = |s: &str| url::form_urlencoded::byte_serialize(s.as_bytes()).collect::<String>();
    format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", enc(id), enc(secret)))
    )
}

/// Reads the `email_verified` claim: a bool, or the strings `"true"` and
/// `"false"`. Anything else is not verified.
fn email_verified_claim(value: Option<&serde_json::Value>) -> bool {
    match value {
        Some(serde_json::Value::Bool(verified)) => *verified,
        Some(serde_json::Value::String(text)) => text == "true",
        _ => false,
    }
}

/// The fixed reason string for a failed `id_token` check. It names the check
/// and never the token or its claims.
fn claim_failure(kind: &ErrorKind) -> &'static str {
    match kind {
        ErrorKind::ExpiredSignature => "expired",
        ErrorKind::ImmatureSignature => "not_yet_valid",
        ErrorKind::InvalidIssuer => "bad_iss",
        ErrorKind::InvalidAudience => "bad_aud",
        ErrorKind::InvalidSignature => "bad_signature",
        ErrorKind::MissingRequiredClaim(_) => "missing_claim",
        _ => "bad_token",
    }
}

impl fmt::Debug for OidcProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OidcProvider")
            .field("id", &self.id)
            .field("issuer", &self.issuer)
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl IdentityProvider for OidcProvider {
    fn id(&self) -> &ProviderId {
        &self.id
    }

    fn display_name(&self) -> &str {
        &self.display_name
    }

    fn trusted_email(&self) -> bool {
        self.trusted_email
    }

    async fn prepare(&self) -> Result<(), ProviderError> {
        let doc = self.discovery(Instant::now()).await?;
        self.check_auth_method(&doc)
    }

    /// Built from the cached discovery document, so [`IdentityProvider::prepare`]
    /// must have run first. Without it the URL is empty, which the browser
    /// cannot follow; the warning names the missed step.
    fn authorize_url(
        &self,
        state: &str,
        pkce_challenge: &str,
        nonce: &str,
        redirect_uri: &str,
    ) -> String {
        let Some(doc) = self.cached_discovery() else {
            tracing::warn!(provider = %self.id.as_str(), reason = "not_prepared", "oidc authorize before discovery");
            return String::new();
        };
        let Ok(mut url) = url::Url::parse(&doc.authorization_endpoint) else {
            return String::new();
        };
        url.query_pairs_mut()
            .append_pair("response_type", "code")
            .append_pair("client_id", &self.client_id)
            .append_pair("scope", &self.scopes.join(" "))
            .append_pair("state", state)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("code_challenge", pkce_challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("nonce", nonce);
        url.to_string()
    }

    async fn exchange(
        &self,
        code: &str,
        pkce_verifier: &str,
        nonce: &str,
        redirect_uri: &str,
    ) -> Result<VerifiedIdentity, ProviderError> {
        let now = Instant::now();
        let doc = self.discovery(now).await?;
        self.check_auth_method(&doc)?;
        // The form is finished before the first await: its serializer is not `Send`.
        let body = {
            let mut form = url::form_urlencoded::Serializer::new(String::new());
            form.append_pair("grant_type", "authorization_code")
                .append_pair("code", code)
                .append_pair("redirect_uri", redirect_uri)
                .append_pair("code_verifier", pkce_verifier)
                .append_pair("client_id", &self.client_id);
            if self.token_auth == TokenAuthMethod::ClientSecretPost {
                form.append_pair("client_secret", self.client_secret.expose());
            }
            form.finish()
        };
        let mut request = self.http.post(&doc.token_endpoint).header(
            reqwest::header::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        );
        if self.token_auth == TokenAuthMethod::ClientSecretBasic {
            request = request.header(
                reqwest::header::AUTHORIZATION,
                basic_credentials(&self.client_id, self.client_secret.expose()),
            );
        }
        let resp = request.body(body).send().await.map_err(transport_error)?;
        let status = resp.status();
        if status == reqwest::StatusCode::BAD_REQUEST || status == reqwest::StatusCode::UNAUTHORIZED
        {
            return Err(ProviderError::Token("invalid_grant"));
        }
        if !status.is_success() {
            return Err(ProviderError::Http(status.as_u16()));
        }
        let bytes = read_capped(resp, PROVIDER_BODY_CAP).await?;
        let token: TokenResponse =
            serde_json::from_slice(&bytes).map_err(|_| ProviderError::BadResponse("json"))?;
        self.verify(&token.id_token, nonce, now).await
    }
}

#[cfg(test)]
#[path = "oidc_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "oidc_auth_tests.rs"]
mod auth_tests;
