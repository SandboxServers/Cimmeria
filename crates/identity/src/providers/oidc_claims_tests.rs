//! Tests for the ID-token claim policy (D-CT24). `iat` is required and may not
//! be more than the skew allowance in the future; `nbf` is checked with the
//! same allowance; `azp` is required with several audiences and must name the
//! client. Each test signs the good claims with one change and runs `verify`.

use std::time::Instant;

use jsonwebtoken::get_current_timestamp;
use serde_json::{json, Value};

use super::tests::{
    good_claims, oidc_provider, serve_jwks, start_issuer, TestKey, CLIENT_ID, NONCE,
};
use crate::provider::{ProviderError, VerifiedIdentity};

/// Signs the good claims, after `edit` changes them, and runs the `id_token`
/// check against a fresh issuer.
async fn verify_edited(edit: impl FnOnce(&mut Value)) -> Result<VerifiedIdentity, ProviderError> {
    let key = TestKey::new("k1");
    let server = start_issuer(None).await;
    let issuer = server.uri();
    serve_jwks(&server, &[&key]).await;
    let mut claims = good_claims(&issuer);
    edit(&mut claims);
    oidc_provider(&issuer)
        .verify(&key.sign(&claims), NONCE, Instant::now())
        .await
}

#[tokio::test]
async fn a_token_without_iat_is_refused() {
    let refused = verify_edited(|claims| {
        claims
            .as_object_mut()
            .expect("the claims are an object")
            .remove("iat");
    })
    .await
    .expect_err("the token is refused");

    assert_eq!(refused, ProviderError::Token("missing_claim"));
}

#[tokio::test]
async fn an_iat_five_minutes_ahead_is_refused() {
    let refused = verify_edited(|claims| claims["iat"] = json!(get_current_timestamp() + 300))
        .await
        .expect_err("the token is refused");

    assert_eq!(refused, ProviderError::Token("iat_future"));
}

#[tokio::test]
async fn an_iat_thirty_seconds_ahead_is_accepted() {
    verify_edited(|claims| claims["iat"] = json!(get_current_timestamp() + 30))
        .await
        .expect("an iat inside the 60 s skew is accepted");
}

#[tokio::test]
async fn a_token_with_nbf_five_minutes_ahead_is_refused() {
    let refused = verify_edited(|claims| claims["nbf"] = json!(get_current_timestamp() + 300))
        .await
        .expect_err("the token is refused");

    assert_eq!(refused, ProviderError::Token("not_yet_valid"));
}

#[tokio::test]
async fn a_token_with_nbf_thirty_seconds_ahead_is_accepted() {
    verify_edited(|claims| claims["nbf"] = json!(get_current_timestamp() + 30))
        .await
        .expect("an nbf inside the 60 s leeway is accepted");
}

#[tokio::test]
async fn two_audiences_without_azp_are_refused() {
    let refused = verify_edited(|claims| claims["aud"] = json!([CLIENT_ID, "other"]))
        .await
        .expect_err("the token is refused");

    assert_eq!(refused, ProviderError::Token("azp_missing"));
}

#[tokio::test]
async fn two_audiences_with_azp_equal_to_the_client_id_are_accepted() {
    verify_edited(|claims| {
        claims["aud"] = json!([CLIENT_ID, "other"]);
        claims["azp"] = json!(CLIENT_ID);
    })
    .await
    .expect("the multi-audience token with its azp is accepted");
}

#[tokio::test]
async fn an_azp_naming_another_client_is_refused() {
    let refused = verify_edited(|claims| claims["azp"] = json!("other"))
        .await
        .expect_err("the token is refused");

    assert_eq!(refused, ProviderError::Token("azp_mismatch"));
}
