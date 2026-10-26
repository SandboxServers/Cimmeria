//! Tests for the token endpoint auth method (CT-83): the Basic default, the
//! Post path, RFC 6749 encoding and the discovery refusal. They reuse the
//! helpers of `oidc_tests`, served by the same `wiremock` issuer.

use std::sync::{Arc, Mutex};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Value};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use super::basic_credentials;
use super::tests::*;
use crate::provider::{IdentityProvider, ProviderError};

/// Starts an issuer whose discovery lists `methods` as its token endpoint auth
/// methods; `None` leaves the field out.
async fn issuer_listing(methods: Option<Value>) -> MockServer {
    let server = MockServer::start().await;
    let uri = server.uri();
    let mut document = json!({
        "issuer": uri,
        "authorization_endpoint": format!("{uri}/authorize"),
        "token_endpoint": format!("{uri}/token"),
        "jwks_uri": format!("{uri}/jwks"),
    });
    if let Some(methods) = methods {
        document["token_endpoint_auth_methods_supported"] = methods;
    }
    Mock::given(method("GET"))
        .and(path("/.well-known/openid-configuration"))
        .respond_with(ResponseTemplate::new(200).set_body_json(document))
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn basic_default_sends_the_secret_in_the_authorization_header() {
    let key = TestKey::new("k1");
    let server = start_issuer(None).await;
    let issuer = server.uri();
    serve_jwks(&server, &[&key]).await;
    let expected = format!(
        "Basic {}",
        STANDARD.encode(format!("{CLIENT_ID}:{CLIENT_SECRET}"))
    );
    let id_token = key.sign(&good_claims(&issuer));
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(header("authorization", expected.as_str()))
        .and(|req: &Request| !String::from_utf8_lossy(&req.body).contains("client_secret="))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id_token": id_token })))
        .mount(&server)
        .await;
    let provider = oidc_provider(&issuer);

    provider
        .exchange("code-value", "verifier-value", NONCE, REDIRECT)
        .await
        .expect("the Basic exchange succeeds");
}

#[tokio::test]
async fn post_sends_the_secret_in_the_form() {
    let key = TestKey::new("k1");
    let server = start_issuer(None).await;
    let issuer = server.uri();
    serve_jwks(&server, &[&key]).await;
    let id_token = key.sign(&good_claims(&issuer));
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(|req: &Request| {
            let body = String::from_utf8_lossy(&req.body);
            body.contains("client_secret=")
                && !req
                    .headers
                    .iter()
                    .any(|(name, _)| name.as_str() == "authorization")
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "id_token": id_token })))
        .mount(&server)
        .await;
    let provider = oidc_provider_with(
        &issuer,
        "token_endpoint_auth_method = \"client_secret_post\"",
    );

    provider
        .exchange("code-value", "verifier-value", NONCE, REDIRECT)
        .await
        .expect("the Post exchange succeeds");
}

#[test]
fn basic_credentials_form_encode_each_part() {
    let expected = format!("Basic {}", STANDARD.encode("a%3Ab:s+p%26"));
    assert_eq!(basic_credentials("a:b", "s p&"), expected);
}

/// Keeps each event as one text line (level, then `name=value` fields), for
/// the WARN assertion. Crate-local, so no subscriber dependency is needed.
#[derive(Clone, Default)]
struct Events(Arc<Mutex<Vec<String>>>);

/// Appends each field of an event to the line being built.
struct Fields<'a>(&'a mut String);

impl tracing::field::Visit for Fields<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.push_str(&format!(" {}={:?}", field.name(), value));
    }
}

impl tracing::Subscriber for Events {
    fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        let mut line = event.metadata().level().to_string();
        event.record(&mut Fields(&mut line));
        self.0.lock().expect("the events lock").push(line);
    }
    fn enter(&self, _: &tracing::span::Id) {}
    fn exit(&self, _: &tracing::span::Id) {}
}

#[tokio::test]
async fn prepare_refuses_a_method_the_issuer_does_not_list() {
    let server = issuer_listing(Some(json!(["client_secret_post"]))).await;
    let provider = oidc_provider(&server.uri());
    let events = Events::default();
    let _guard = tracing::subscriber::set_default(events.clone());

    let refused = provider
        .prepare()
        .await
        .expect_err("a basic-only config is refused by a post-only issuer");
    assert_eq!(
        refused,
        ProviderError::BadResponse("auth_method_unsupported")
    );

    let logged = events.0.lock().expect("the events lock").join("\n");
    assert!(logged.contains("WARN"), "no WARN row: {logged}");
    assert!(logged.contains("auth_method_unsupported"), "{logged}");
    assert!(logged.contains("client_secret_basic"), "{logged}");
    assert!(
        !logged.contains(CLIENT_SECRET),
        "the secret reached the log"
    );
}

#[tokio::test]
async fn absent_metadata_is_not_refused() {
    let key = TestKey::new("k1");
    let server = start_issuer(None).await;
    let issuer = server.uri();
    serve_jwks(&server, &[&key]).await;
    serve_token(&server, &key.sign(&good_claims(&issuer))).await;
    let provider = oidc_provider_with(
        &issuer,
        "token_endpoint_auth_method = \"client_secret_post\"",
    );

    provider.prepare().await.expect("prepare succeeds");
    provider
        .exchange("code-value", "verifier-value", NONCE, REDIRECT)
        .await
        .expect("the exchange succeeds without the metadata");
}
