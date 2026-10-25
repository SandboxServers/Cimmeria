//! The OpenID Connect discovery document and the signing-key cache (contract
//! C4). Discovery is fetched from `{issuer}/.well-known/openid-configuration`
//! and checked: the issuer must equal the configured one byte for byte, and
//! every endpoint must be `https://`. The key cache keeps the issuer's JWKS for
//! an hour and refetches it at most once a minute when a token's `kid` is not
//! in it.

use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use jsonwebtoken::jwk::{Jwk, JwkSet};
use serde::Deserialize;

use crate::http::{read_capped, transport_error, PROVIDER_BODY_CAP};
use crate::provider::ProviderError;

/// How long a discovery document or a JWKS stays fresh.
pub const CACHE_LIFETIME: Duration = Duration::from_secs(60 * 60);

/// The shortest gap between two JWKS refetches for unknown `kid`s.
pub const REFETCH_GAP: Duration = Duration::from_secs(60);

/// The fields of a discovery document the provider uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discovery {
    /// Where the user is sent to sign in.
    pub authorization_endpoint: String,
    /// Where the authorization code is traded for tokens.
    pub token_endpoint: String,
    /// Where the issuer publishes its signing keys.
    pub jwks_uri: String,
    /// The issuer, equal to the configured one.
    pub issuer: String,
}

/// The discovery document as the issuer sends it.
#[derive(Deserialize)]
struct RawDiscovery {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    jwks_uri: String,
}

/// Whether an endpoint may be contacted: `https://`, or a loopback `http://`
/// address under test.
fn endpoint_allowed(url: &str) -> bool {
    url.starts_with("https://") || (cfg!(test) && url.starts_with("http://127.0.0.1"))
}

impl Discovery {
    /// Fetches the discovery document of `issuer` and checks it against that
    /// issuer. A document naming another issuer is refused (mix-up defence).
    pub async fn fetch(http: &reqwest::Client, issuer: &str) -> Result<Discovery, ProviderError> {
        let url = format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        );
        let body = get_capped(http, &url).await?;
        let raw: RawDiscovery =
            serde_json::from_slice(&body).map_err(|_| ProviderError::BadResponse("json"))?;
        if raw.issuer != issuer {
            return Err(ProviderError::BadResponse("issuer_mismatch"));
        }
        let endpoints = [
            &raw.authorization_endpoint,
            &raw.token_endpoint,
            &raw.jwks_uri,
        ];
        if !endpoints.iter().all(|url| endpoint_allowed(url)) {
            return Err(ProviderError::BadResponse("endpoint_not_https"));
        }
        Ok(Discovery {
            authorization_endpoint: raw.authorization_endpoint,
            token_endpoint: raw.token_endpoint,
            jwks_uri: raw.jwks_uri,
            issuer: raw.issuer,
        })
    }
}

/// What the cache says about one `kid`.
enum Lookup {
    /// The key is cached and fresh.
    Found(Box<Jwk>),
    /// The cache is fresh and does not hold the key.
    Unknown,
}

/// The JWKS of one issuer and when it was fetched.
struct Keys {
    /// The published signing keys.
    set: JwkSet,
    /// When `set` was fetched.
    fetched: Instant,
}

/// The issuer's signing keys, cached for an hour (contract C4).
#[derive(Default)]
pub struct KeyCache {
    /// `None` until the first fetch.
    state: Mutex<Option<Keys>>,
    /// When the last refetch for an unknown `kid` started, if one has. Set
    /// before the fetch runs, so a failed refetch also starts the gap.
    last_refetch: Mutex<Option<Instant>>,
}

impl KeyCache {
    /// Returns the key `kid` names. A stale or empty cache is fetched first.
    /// A `kid` the fresh set lacks triggers one refetch, and at most one per
    /// minute; a `kid` still unknown after that is `Token("unknown_kid")`.
    pub async fn key(
        &self,
        http: &reqwest::Client,
        jwks_uri: &str,
        kid: &str,
        now: Instant,
    ) -> Result<Jwk, ProviderError> {
        if let Some(jwk) = self.fresh_key(kid, now) {
            return Ok(jwk);
        }
        if !self.is_fresh(now) {
            self.fetch(http, jwks_uri, now).await?;
            if let Some(jwk) = self.fresh_key(kid, now) {
                return Ok(jwk);
            }
        }
        if !self.claim_refetch(now) {
            return Err(ProviderError::Token("unknown_kid"));
        }
        self.fetch(http, jwks_uri, now).await?;
        self.fresh_key(kid, now)
            .ok_or(ProviderError::Token("unknown_kid"))
    }

    /// The cached key `kid` names, when the cache is fresh and holds it.
    fn fresh_key(&self, kid: &str, now: Instant) -> Option<Jwk> {
        match self.lookup(kid, now) {
            Lookup::Found(jwk) => Some(*jwk),
            Lookup::Unknown => None,
        }
    }

    /// What the fresh cache says about `kid`. An empty or stale cache says
    /// `Unknown` too, which sends the caller to fetch.
    fn lookup(&self, kid: &str, now: Instant) -> Lookup {
        let guard = self.lock();
        match guard.as_ref() {
            Some(keys) if now.duration_since(keys.fetched) < CACHE_LIFETIME => keys
                .set
                .find(kid)
                .cloned()
                .map_or(Lookup::Unknown, |jwk| Lookup::Found(Box::new(jwk))),
            _ => Lookup::Unknown,
        }
    }

    /// Whether the cache holds a JWKS that is not yet an hour old.
    fn is_fresh(&self, now: Instant) -> bool {
        self.lock()
            .as_ref()
            .is_some_and(|keys| now.duration_since(keys.fetched) < CACHE_LIFETIME)
    }

    /// Claims the refetch for an unknown `kid`. Returns `false` when a refetch
    /// started less than `REFETCH_GAP` ago; otherwise it records `now` as the
    /// start of a new refetch and returns `true`. The check and the record
    /// happen under one lock, so concurrent callers cannot both claim it.
    fn claim_refetch(&self, now: Instant) -> bool {
        let mut last = self
            .last_refetch
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(at) = *last {
            if now.duration_since(at) < REFETCH_GAP {
                return false;
            }
        }
        *last = Some(now);
        true
    }

    /// Fetches the JWKS and stores it.
    async fn fetch(
        &self,
        http: &reqwest::Client,
        jwks_uri: &str,
        now: Instant,
    ) -> Result<(), ProviderError> {
        let body = get_capped(http, jwks_uri).await?;
        let set: JwkSet =
            serde_json::from_slice(&body).map_err(|_| ProviderError::BadResponse("json"))?;
        *self.lock() = Some(Keys { set, fetched: now });
        Ok(())
    }

    /// The state lock. A poisoned lock still holds a whole value, so it is
    /// taken anyway.
    fn lock(&self) -> MutexGuard<'_, Option<Keys>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// GETs `url` and returns the body, capped at 64 KiB. A non-2xx status is
/// `Http(status)`.
pub(crate) async fn get_capped(
    http: &reqwest::Client,
    url: &str,
) -> Result<Vec<u8>, ProviderError> {
    let resp = http.get(url).send().await.map_err(transport_error)?;
    let status = resp.status();
    if !status.is_success() {
        return Err(ProviderError::Http(status.as_u16()));
    }
    read_capped(resp, PROVIDER_BODY_CAP).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoints_must_be_https_outside_tests() {
        assert!(endpoint_allowed("https://issuer.example.org/token"));
        assert!(endpoint_allowed("http://127.0.0.1:8080/token"));
        assert!(!endpoint_allowed("http://issuer.example.org/token"));
        assert!(!endpoint_allowed("ftp://127.0.0.1/token"));
    }

    #[test]
    fn empty_cache_is_unknown_and_refetch_is_open() {
        let cache = KeyCache::default();
        let now = Instant::now();
        assert!(!cache.is_fresh(now));
        assert!(matches!(cache.lookup("k1", now), Lookup::Unknown));
        assert!(cache.claim_refetch(now));
        assert!(!cache.claim_refetch(now));
        assert!(cache.claim_refetch(now + REFETCH_GAP));
    }
}
