//! Login probe: the checks the container smoke tests run against a live
//! server (issue #1291). The `login-probe` binary is a thin CLI over [`run`].
//!
//! A real client's login is three hops: SOAP Phase 1 (credentials), SOAP
//! Phase 2 (shard selection, which hands back the BaseApp endpoint, ticket
//! and session key), then the Mercury `baseAppLogin` handshake against that
//! endpoint. The probe walks all three and also checks that a wrong password
//! is turned away, so a server that accepts anything cannot pass:
//!
//! 1. Phase 1 with a wrong password must come back as an `<SGWLoginError>`
//!    whose reason is [`BAD_PASSWORD_REASON`]. Any other reason (a database
//!    failure, no shards) fails the probe, and so does a success: developer
//!    mode without a database accepts every password.
//! 2. Phase 1 + 2 with the real password must succeed, with the expected
//!    account id when one is given.
//! 3. The advertised BaseApp endpoint must equal the expected one when given.
//! 4. Unless disabled, the Mercury handshake runs against the endpoint the
//!    server advertised (not one the caller supplies, which is what a client
//!    does too), and must produce the encrypted reply and time-sync packets.

use std::net::SocketAddr;

use sha1::{Digest, Sha1};

use crate::auth::{AuthClient, AuthSession, Credentials};
use crate::error::Error;
use crate::session::GameSession;

/// `ErrorStr` the auth server sends for a wrong password or an unknown
/// account (C++ `FailureCode::BadUserPassword`; `cimmeria_auth`'s
/// `handle_user_auth`).
pub const BAD_PASSWORD_REASON: &str = "The account name or password is incorrect.";

/// Request id the probe's `baseAppLogin` carries; the reply must echo it.
const PROBE_REQUEST_ID: u32 = 0x0000_1291;

/// What to probe and what to expect.
#[derive(Debug, Clone)]
pub struct ProbeConfig {
    /// Auth base URL, e.g. `http://127.0.0.1:8081`.
    pub auth_url: String,
    pub user: String,
    /// The account's plaintext password; sent as the uppercase SHA-1 hex the
    /// original client sends.
    pub password: String,
    pub shard: String,
    /// The BaseApp endpoint Phase 2 must advertise.
    pub expect_base: Option<SocketAddr>,
    /// The account id Phase 1 must return.
    pub expect_account_id: Option<u32>,
    /// Run the Mercury handshake against the advertised endpoint.
    pub handshake: bool,
}

/// What a passing probe saw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeReport {
    pub account_id: u32,
    pub base_addr: SocketAddr,
    pub handshake_done: bool,
}

/// Why a probe failed. Each variant names the step.
#[derive(Debug, thiserror::Error)]
pub enum ProbeFailure {
    #[error(
        "wrong password was ACCEPTED (account id {account_id}); auth is not checking credentials"
    )]
    WrongPasswordAccepted { account_id: u32 },

    #[error("wrong password was rejected for the wrong reason: {0:?} (want \"The account name or password is incorrect.\")")]
    WrongPasswordOtherReason(String),

    #[error("wrong-password request failed before a verdict: {0}")]
    WrongPasswordRequest(Error),

    #[error("login with the real password failed: {0}")]
    Login(Error),

    #[error("Phase 1 returned account id {got}, want {expected}")]
    AccountIdMismatch { expected: u32, got: u32 },

    #[error("Phase 2 advertised BaseApp endpoint {got}, want {expected}")]
    BaseMismatch {
        expected: SocketAddr,
        got: SocketAddr,
    },

    #[error("Mercury handshake against the advertised BaseApp {base} failed: {source}")]
    Handshake { base: SocketAddr, source: Error },
}

/// Uppercase SHA-1 hex of `password`, as the SGW client sends it.
pub fn password_sha1_hex(password: &str) -> String {
    hex::encode_upper(Sha1::digest(password.as_bytes()))
}

fn credentials(user: &str, password: &str) -> Credentials {
    Credentials {
        username: user.to_string(),
        password_sha1_hex: password_sha1_hex(password),
        ..Credentials::test_account()
    }
}

/// The account id and BaseApp endpoint checks on a completed Phase 1 + 2.
pub fn check_session(cfg: &ProbeConfig, session: &AuthSession) -> Result<(), ProbeFailure> {
    if let Some(expected) = cfg.expect_account_id {
        if session.account_id != expected {
            return Err(ProbeFailure::AccountIdMismatch {
                expected,
                got: session.account_id,
            });
        }
    }
    if let Some(expected) = cfg.expect_base {
        if session.base_addr != expected {
            return Err(ProbeFailure::BaseMismatch {
                expected,
                got: session.base_addr,
            });
        }
    }
    Ok(())
}

/// Run every check in order, stopping at the first failure.
pub async fn run(cfg: &ProbeConfig) -> Result<ProbeReport, ProbeFailure> {
    let auth = AuthClient::new(&cfg.auth_url);

    // 1. Wrong password first: a server that skips the credential check
    // would otherwise pass every later step.
    let wrong = credentials(&cfg.user, &format!("{}-wrong-login-probe", cfg.password));
    match auth.phase1(&wrong).await {
        Ok((_, account_id)) => return Err(ProbeFailure::WrongPasswordAccepted { account_id }),
        Err(Error::LoginRejected(reason)) if reason == BAD_PASSWORD_REASON => {
            tracing::info!(reason, "wrong password rejected");
        }
        Err(Error::LoginRejected(reason)) => {
            return Err(ProbeFailure::WrongPasswordOtherReason(reason))
        }
        Err(e) => return Err(ProbeFailure::WrongPasswordRequest(e)),
    }

    // 2. The real login, Phase 1 + 2.
    let session: AuthSession = auth
        .login(&credentials(&cfg.user, &cfg.password), &cfg.shard)
        .await
        .map_err(ProbeFailure::Login)?;
    tracing::info!(
        account_id = session.account_id,
        base = %session.base_addr,
        "SOAP Phase 1 + 2 succeeded"
    );
    // 3. The account id and the advertised endpoint.
    check_session(cfg, &session)?;

    // 4. Mercury handshake against what the server advertised.
    if cfg.handshake {
        GameSession::from_auth_session(&session, PROBE_REQUEST_ID)
            .await
            .map_err(|source| ProbeFailure::Handshake {
                base: session.base_addr,
                source,
            })?;
        tracing::info!(base = %session.base_addr, "Mercury handshake succeeded");
    }

    Ok(ProbeReport {
        account_id: session.account_id,
        base_addr: session.base_addr,
        handshake_done: cfg.handshake,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(expect_base: &str, expect_account_id: u32) -> ProbeConfig {
        ProbeConfig {
            auth_url: String::new(),
            user: "test".into(),
            password: "test".into(),
            shard: "Test".into(),
            expect_base: Some(expect_base.parse().unwrap()),
            expect_account_id: Some(expect_account_id),
            handshake: false,
        }
    }

    fn session(base: &str, account_id: u32) -> AuthSession {
        AuthSession {
            base_addr: base.parse().unwrap(),
            ticket: "A".repeat(20),
            session_key: [0; 32],
            account_id,
        }
    }

    #[test]
    fn check_session_passes_when_endpoint_and_account_match() {
        let r = check_session(&cfg("127.0.0.1:32832", 2), &session("127.0.0.1:32832", 2));
        assert!(r.is_ok(), "{r:?}");
    }

    #[test]
    fn check_session_rejects_a_different_advertised_ip() {
        // The image default BASE_EXTERNAL is 127.0.0.1; a container that
        // advertises its bridge address instead is unreachable to a client.
        let err =
            check_session(&cfg("127.0.0.1:32832", 2), &session("172.17.0.2:32832", 2)).unwrap_err();
        match err {
            ProbeFailure::BaseMismatch { expected, got } => {
                assert_eq!(expected, "127.0.0.1:32832".parse().unwrap());
                assert_eq!(got, "172.17.0.2:32832".parse().unwrap());
            }
            other => panic!("expected BaseMismatch, got {other}"),
        }
    }

    #[test]
    fn check_session_rejects_a_different_advertised_port() {
        let err =
            check_session(&cfg("127.0.0.1:32832", 2), &session("127.0.0.1:32833", 2)).unwrap_err();
        assert!(
            matches!(err, ProbeFailure::BaseMismatch { got, .. } if got.port() == 32833),
            "{err}"
        );
    }

    #[test]
    fn check_session_rejects_a_different_account_id() {
        // Developer mode without a database answers every login as account 1.
        let err =
            check_session(&cfg("127.0.0.1:32832", 2), &session("127.0.0.1:32832", 1)).unwrap_err();
        assert!(
            matches!(
                err,
                ProbeFailure::AccountIdMismatch {
                    expected: 2,
                    got: 1
                }
            ),
            "{err}"
        );
    }

    #[test]
    fn password_sha1_hex_matches_the_test_account_credential() {
        // The seeded `test` account's stored hash, and `Credentials::test_account()`.
        assert_eq!(
            password_sha1_hex("test"),
            "A94A8FE5CCB19BA61C4C0873D391E987982FBBD3"
        );
    }
}
