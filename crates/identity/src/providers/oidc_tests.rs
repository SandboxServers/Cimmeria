//! Tests for the OIDC provider. A `wiremock` issuer serves the discovery
//! document, the JWKS and the token endpoint; each test generates its own
//! ES256 key. Nothing reaches a real identity provider.

use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use jsonwebtoken::{encode, get_current_timestamp, Algorithm, EncodingKey, Header};
use rcgen::KeyPair;
use serde_json::{json, Value};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::OidcProvider;
use crate::config;
use crate::http::provider_client;
use crate::provider::{IdentityProvider, ProviderError};

pub(super) const CLIENT_ID: &str = "example-client";
pub(super) const NONCE: &str = "nonce-under-test";
pub(super) const REDIRECT: &str = "https://play.example.org/auth/v1/callback/oidc";
pub(super) const CLIENT_SECRET: &str = "test-secret";

/// An ES256 signing key and the kid it publishes under.
pub(super) struct TestKey {
    /// The key id the JWKS and the token header carry.
    kid: String,
    /// The private key as PKCS#8 DER, for signing.
    pkcs8: Vec<u8>,
    /// The public key as an uncompressed SEC1 point (`04 || x || y`).
    public: Vec<u8>,
}

impl TestKey {
    /// Generates a fresh P-256 key under `kid`.
    pub(super) fn new(kid: &str) -> TestKey {
        let pair = KeyPair::generate().expect("a P-256 key generates");
        TestKey {
            kid: kid.to_owned(),
            pkcs8: pair.serialize_der(),
            public: pair.public_key_raw().to_vec(),
        }
    }

    /// The public key as a JWK.
    fn jwk(&self) -> Value {
        let (x, y) = self.public[1..].split_at(32);
        json!({
            "kty": "EC",
            "crv": "P-256",
            "kid": self.kid,
            "alg": "ES256",
            "use": "sig",
            "x": URL_SAFE_NO_PAD.encode(x),
            "y": URL_SAFE_NO_PAD.encode(y),
        })
    }

    /// Signs `claims` as an ES256 `id_token` naming this key.
    pub(super) fn sign(&self, claims: &Value) -> String {
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.kid.clone());
        encode(&header, claims, &EncodingKey::from_ec_der(&self.pkcs8)).expect("the token signs")
    }
}

/// Starts a `wiremock` issuer. Its discovery names `claimed`, or the server's
/// own URI when `claimed` is `None`.
pub(super) async fn start_issuer(claimed: Option<&str>) -> MockServer {
    let server = MockServer::start().await;
    let uri = server.uri();
    let issuer = claimed.unwrap_or(uri.as_str());
    let document = json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{uri}/authorize"),
        "token_endpoint": format!("{uri}/token"),
        "jwks_uri": format!("{uri}/jwks"),
    });
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(document))
        .mount(&server)
        .await;
    server
}

/// Serves `keys` as the JWKS, for every request.
pub(super) async fn serve_jwks(server: &MockServer, keys: &[&TestKey]) {
    let set = json!({ "keys": keys.iter().map(|key| key.jwk()).collect::<Vec<_>>() });
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(set))
        .mount(server)
        .await;
}

/// How many times the issuer has been asked for its JWKS.
async fn jwks_fetches(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .unwrap_or_default()
        .iter()
        .filter(|request| request.url.path() == "/jwks")
        .count()
}

/// Builds the provider from a config table, with its issuer pointed at the
/// local `wiremock` server.
pub(super) fn oidc_provider(issuer: &str) -> OidcProvider {
    oidc_provider_with(issuer, "")
}

/// As [`oidc_provider`], with `extra_toml` lines appended to the provider table.
pub(super) fn oidc_provider_with(issuer: &str, extra_toml: &str) -> OidcProvider {
    let text = format!(
        r#"
public_base_url = "https://play.example.org"

[[provider]]
id = "oidc"
kind = "oidc"
display_name = "Test OIDC"
issuer = "https://issuer.example.org"
client_id = "example-client"
client_secret_env = "CIMMERIA_IDP_TEST_SECRET"
trusted_email = true
{extra_toml}
"#
    );
    let env = |name: &str| (name == "CIMMERIA_IDP_TEST_SECRET").then(|| CLIENT_SECRET.to_owned());
    let (mut loaded, _) = config::load(&text, &env, false).expect("the test config loads");
    let mut cfg = loaded.providers.remove(0);
    cfg.issuer = Some(issuer.to_owned());
    OidcProvider::new(&cfg, provider_client())
}

/// The claims of a good token from `issuer`, issued now and expiring in five
/// minutes.
pub(super) fn good_claims(issuer: &str) -> Value {
    json!({
        "iss": issuer,
        "aud": CLIENT_ID,
        "sub": "user-1",
        "iat": get_current_timestamp(),
        "exp": get_current_timestamp() + 300,
        "nonce": NONCE,
        "email": "Player@Example.ORG",
        "email_verified": true,
        "name": "Ada Player",
    })
}

/// Serves `id_token` from the token endpoint, for one exchange.
pub(super) async fn serve_token(server: &MockServer, id_token: &str) {
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id_token": id_token })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn oidc_exchange_verifies_and_maps_claims() {
    let key = TestKey::new("k1");
    let server = start_issuer(None).await;
    let issuer = server.uri();
    serve_jwks(&server, &[&key]).await;
    serve_token(&server, &key.sign(&good_claims(&issuer))).await;
    let provider = oidc_provider(&issuer);

    let identity = provider
        .exchange("code-value", "verifier-value", NONCE, REDIRECT)
        .await
        .expect("a good token exchanges");

    assert_eq!(identity.provider.as_str(), "oidc");
    assert_eq!(identity.issuer, issuer);
    assert_eq!(identity.subject, "user-1");
    assert_eq!(identity.email.as_deref(), Some("player@example.org"));
    assert!(identity.email_verified);
    assert_eq!(identity.display_name, "Ada Player");
    assert_eq!(identity.provider_account_created_ms, None);
}

/// A `alg: none` token: the header and claims, and an empty signature.
fn unsigned_token(claims: &Value) -> String {
    let header = URL_SAFE_NO_PAD.encode(json!({ "alg": "none", "kid": "k1" }).to_string());
    let payload = URL_SAFE_NO_PAD.encode(claims.to_string());
    format!("{header}.{payload}.")
}

#[tokio::test]
async fn oidc_refuses_bad_tokens() {
    let key = TestKey::new("k1");
    let server = start_issuer(None).await;
    let issuer = server.uri();
    serve_jwks(&server, &[&key]).await;
    let provider = oidc_provider(&issuer);
    let good = good_claims(&issuer);

    let mut wrong_aud = good.clone();
    wrong_aud["aud"] = json!("someone-else");
    let mut wrong_iss = good.clone();
    wrong_iss["iss"] = json!("https://evil.example.org");
    let mut expired = good.clone();
    expired["exp"] = json!(get_current_timestamp() - 3600);
    let mut wrong_nonce = good.clone();
    wrong_nonce["nonce"] = json!("another-nonce");

    let hmac = encode(
        &Header::new(Algorithm::HS256),
        &good,
        &EncodingKey::from_secret(CLIENT_SECRET.as_bytes()),
    )
    .expect("the HMAC token signs");

    let cases = [
        (key.sign(&wrong_aud), "bad_aud"),
        (key.sign(&wrong_iss), "bad_iss"),
        (key.sign(&expired), "expired"),
        (key.sign(&wrong_nonce), "bad_nonce"),
        (unsigned_token(&good), "bad_token"),
        (hmac, "bad_alg"),
    ];
    for (token, reason) in cases {
        let refused = provider
            .verify(&token, NONCE, Instant::now())
            .await
            .expect_err("the token is refused");
        assert_eq!(refused, ProviderError::Token(reason));
    }
}

#[tokio::test]
async fn oidc_string_email_verified() {
    let key = TestKey::new("k1");
    let server = start_issuer(None).await;
    let issuer = server.uri();
    serve_jwks(&server, &[&key]).await;
    let provider = oidc_provider(&issuer);

    let cases = [
        (json!(true), true),
        (json!("true"), true),
        (json!("yes"), false),
        (json!("false"), false),
    ];
    for (claim, expected) in cases {
        let mut claims = good_claims(&issuer);
        claims["email_verified"] = claim.clone();
        let identity = provider
            .verify(&key.sign(&claims), NONCE, Instant::now())
            .await
            .expect("the token verifies");
        assert_eq!(identity.email_verified, expected, "claim {claim}");
    }
}

#[tokio::test]
async fn oidc_refetches_jwks_once_for_a_new_kid() {
    // A rotated key: the first JWKS lacks k2, the next one has it.
    let old = TestKey::new("k1");
    let new = TestKey::new("k2");
    let server = start_issuer(None).await;
    let issuer = server.uri();
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "keys": [old.jwk()] })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    serve_jwks(&server, &[&old, &new]).await;
    let provider = oidc_provider(&issuer);

    provider
        .verify(&new.sign(&good_claims(&issuer)), NONCE, Instant::now())
        .await
        .expect("the rotated key verifies after one refetch");
    assert_eq!(jwks_fetches(&server).await, 2);

    // A second server, whose JWKS never gains k3: two unknown-kid tokens in
    // one minute cause one refetch in total.
    let stuck = TestKey::new("k3");
    let server = start_issuer(None).await;
    let issuer = server.uri();
    serve_jwks(&server, &[&old]).await;
    let provider = oidc_provider(&issuer);
    let unknown = stuck.sign(&good_claims(&issuer));

    provider
        .verify(&old.sign(&good_claims(&issuer)), NONCE, Instant::now())
        .await
        .expect("the known key verifies");
    assert_eq!(jwks_fetches(&server).await, 1);
    for _ in 0..2 {
        let refused = provider
            .verify(&unknown, NONCE, Instant::now())
            .await
            .expect_err("an unknown kid is refused");
        assert_eq!(refused, ProviderError::Token("unknown_kid"));
    }
    assert_eq!(jwks_fetches(&server).await, 2);
}

#[tokio::test]
async fn oidc_unknown_kid_refetch_is_claimed_once() {
    let known = TestKey::new("k1");
    let stranger = TestKey::new("k3");
    let server = start_issuer(None).await;
    let issuer = server.uri();
    // A slow JWKS, so the refetches of concurrent exchanges overlap.
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "keys": [known.jwk()] }))
                .set_delay(Duration::from_millis(200)),
        )
        .mount(&server)
        .await;
    let provider = Arc::new(oidc_provider(&issuer));
    provider
        .verify(&known.sign(&good_claims(&issuer)), NONCE, Instant::now())
        .await
        .expect("the known key verifies");
    assert_eq!(jwks_fetches(&server).await, 1);

    let unknown = stranger.sign(&good_claims(&issuer));
    let tasks: Vec<_> = (0..8)
        .map(|_| {
            let provider = Arc::clone(&provider);
            let token = unknown.clone();
            tokio::spawn(async move { provider.verify(&token, NONCE, Instant::now()).await })
        })
        .collect();
    for task in tasks {
        let refused = task
            .await
            .expect("the task completes")
            .expect_err("an unknown kid is refused");
        assert_eq!(refused, ProviderError::Token("unknown_kid"));
    }
    assert_eq!(jwks_fetches(&server).await, 2);
}

#[tokio::test]
async fn oidc_failed_refetch_starts_the_gap() {
    let known = TestKey::new("k1");
    let stranger = TestKey::new("k3");
    let server = start_issuer(None).await;
    let issuer = server.uri();
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "keys": [known.jwk()] })))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/jwks"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let provider = oidc_provider(&issuer);
    provider
        .verify(&known.sign(&good_claims(&issuer)), NONCE, Instant::now())
        .await
        .expect("the known key verifies");
    assert_eq!(jwks_fetches(&server).await, 1);

    let unknown = stranger.sign(&good_claims(&issuer));
    let failed = provider
        .verify(&unknown, NONCE, Instant::now())
        .await
        .expect_err("the failed refetch refuses the token");
    assert_eq!(failed, ProviderError::Http(500));
    let gapped = provider
        .verify(&unknown, NONCE, Instant::now())
        .await
        .expect_err("the gap refuses the token");
    assert_eq!(gapped, ProviderError::Token("unknown_kid"));
    assert_eq!(jwks_fetches(&server).await, 2);
}

#[tokio::test]
async fn oidc_issuer_with_trailing_slash() {
    let server = MockServer::start().await;
    let uri = server.uri();
    let issuer = format!("{uri}/");
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "issuer": issuer,
            "authorization_endpoint": format!("{uri}/authorize"),
            "token_endpoint": format!("{uri}/token"),
            "jwks_uri": format!("{uri}/jwks"),
        })))
        .mount(&server)
        .await;
    let provider = oidc_provider(&issuer);

    provider
        .prepare()
        .await
        .expect("discovery is fetched without a doubled slash");
}

#[tokio::test]
async fn oidc_discovery_issuer_must_match() {
    let server = start_issuer(Some("http://127.0.0.1:1/other-issuer")).await;
    let provider = oidc_provider(&server.uri());

    let refused = provider
        .prepare()
        .await
        .expect_err("the discovery is refused");
    assert_eq!(refused, ProviderError::BadResponse("issuer_mismatch"));
}

#[tokio::test]
async fn oidc_authorize_url_carries_pkce_and_nonce() {
    let server = start_issuer(None).await;
    let issuer = server.uri();
    let provider = oidc_provider(&issuer);
    provider.prepare().await.expect("discovery succeeds");

    let url =
        url::Url::parse(&provider.authorize_url("state-value", "challenge-value", NONCE, REDIRECT))
            .expect("the authorize URL parses");
    let query: std::collections::HashMap<String, String> = url.query_pairs().into_owned().collect();

    assert_eq!(url.path(), "/authorize");
    assert_eq!(query["response_type"], "code");
    assert_eq!(query["client_id"], CLIENT_ID);
    assert_eq!(query["scope"], "openid email profile");
    assert_eq!(query["state"], "state-value");
    assert_eq!(query["redirect_uri"], REDIRECT);
    assert_eq!(query["code_challenge"], "challenge-value");
    assert_eq!(query["code_challenge_method"], "S256");
    assert_eq!(query["nonce"], NONCE);
}
