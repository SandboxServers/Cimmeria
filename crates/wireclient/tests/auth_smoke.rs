//! Auth smoke — drive wireclient's SOAP client against an in-process
//! `AuthService`.
//!
//! Pairs symmetrically with the server-side
//! `cimmeria_services::auth::login_smoke`: that test asserts the server
//! emits the expected XML and cookie; this one asserts the wireclient
//! consumes them correctly and produces a load-bearing [`AuthSession`].
//! Together they pin both sides of the SOAP handshake.

use std::net::TcpListener as StdTcpListener;

use cimmeria_common::ServerConfig;
use cimmeria_services::auth::{AuthService, ShardInfo};
use cimmeria_wireclient::auth::{AuthClient, Credentials};
use cimmeria_wireclient::Error;

const SHARD: &str = "WireclientSmoke";
const SHARD_HOST: &str = "127.0.0.1";
const SHARD_PORT: u16 = 32832;

fn ephemeral_port() -> u16 {
    let listener = StdTcpListener::bind("127.0.0.1:0").expect("bind ephemeral TCP listener");
    let port = listener.local_addr().expect("local_addr").port();
    drop(listener);
    port
}

async fn start_auth() -> (AuthService, u16) {
    const MAX_ATTEMPTS: usize = 5;
    let base_config = ServerConfig {
        auth_host: "127.0.0.1".to_string(),
        developer_mode: true,
        ..ServerConfig::default()
    };
    for _ in 0..MAX_ATTEMPTS {
        let port = ephemeral_port();
        let mut config = base_config.clone();
        config.logon_port = port;
        let mut auth = AuthService::new(&config);
        auth.register_shard(ShardInfo {
            name: SHARD.into(),
            host: SHARD_HOST.into(),
            port: SHARD_PORT,
            protected: false,
        });
        if auth.start().await.is_ok() {
            return (auth, port);
        }
    }
    panic!("could not bind AuthService on an ephemeral port after 5 attempts");
}

#[tokio::test]
async fn wireclient_drives_phase1_phase2_against_inprocess_auth() {
    let (mut auth, port) = start_auth().await;
    let base_url = format!("http://127.0.0.1:{port}");
    let client = AuthClient::new(&base_url);

    let session = client
        .login(&Credentials::test_account(), SHARD)
        .await
        .expect("login should succeed against developer-mode auth");

    // ── Phase 2 attributes are load-bearing for the Mercury handshake ──
    // SessionKey decoded into 32 bytes.
    // (Just asserting the array length is non-trivial — a regression in
    // the hex-decode path would surface as a length mismatch in
    // `auth::phase2`, not here.)
    assert_eq!(session.session_key.len(), 32);

    // Ticket is exactly 20 ASCII chars — the server-side
    // `parse_baseapp_login` rejects anything else.
    assert_eq!(session.ticket.len(), 20);
    assert!(
        session.ticket.is_ascii(),
        "ticket must be ASCII (got {:?})",
        session.ticket
    );

    // Shard endpoint matches what we registered.
    assert_eq!(session.base_addr.ip().to_string(), SHARD_HOST);
    assert_eq!(session.base_addr.port(), SHARD_PORT);

    // account_id is parsed from Phase 1's `<AccountInfo AccountId="…">`
    // and threaded onto the session — `build_baseapp_login` reads it
    // here, not from a caller-supplied parameter.
    // Developer-mode auth assigns account_id=1.
    assert_eq!(
        session.account_id, 1,
        "developer_mode auth must return account_id=1 (got {})",
        session.account_id
    );

    // The encryption context derived from the session key must use the
    // same key for AES and HMAC (zero-IV) per C++ `EncryptionFilter::setKey`.
    // Smoke-check by encrypting a known 16-byte plaintext and asserting
    // the output is exactly 16 (ciphertext) + 16 (PKCS7) + 16 (HMAC) = 48
    // bytes — any regression in the from_session_key adapter would
    // surface as a different output length.
    let enc = session.encryption();
    let ct = enc.encrypt(&[0u8; 16]).expect("encrypt");
    assert_eq!(
        ct.len(),
        48,
        "16B plaintext + PKCS7 + HMAC must be exactly 48B"
    );

    auth.stop().await;
}

#[tokio::test]
async fn wireclient_phase1_returns_sid_cookie() {
    let (mut auth, port) = start_auth().await;
    let base_url = format!("http://127.0.0.1:{port}");
    let client = AuthClient::new(&base_url);

    let (sid, account_id) = client
        .phase1(&Credentials::test_account())
        .await
        .expect("Phase 1 should succeed");

    assert_eq!(sid.len(), 40, "SID must be 40 chars (C++ format)");
    assert!(
        sid.chars().all(|c| c.is_ascii_alphanumeric()),
        "SID must be ASCII alphanumeric (got {sid:?})"
    );
    assert_eq!(account_id, 1, "developer_mode account_id is 1");

    auth.stop().await;
}

#[tokio::test]
async fn wireclient_phase2_replay_with_same_sid_errors() {
    let (mut auth, port) = start_auth().await;
    let base_url = format!("http://127.0.0.1:{port}");
    let client = AuthClient::new(&base_url);

    let (sid, account_id) = client.phase1(&Credentials::test_account()).await.unwrap();
    let _ok = client.phase2(&sid, account_id, SHARD).await.unwrap();
    // Sessions are single-use. A second Phase 2 with the same SID must
    // not produce a fresh AuthSession — the server returns
    // `ServerSelectionError`, which our parser surfaces as the
    // typed `Error::NoServerLocation` variant. Assert the variant
    // directly so a regression that surfaces a different (still-
    // string-matching) error doesn't pass.
    let err = client.phase2(&sid, account_id, SHARD).await.unwrap_err();
    assert!(
        matches!(err, Error::NoServerLocation),
        "reused SID must fail with Error::NoServerLocation, got: {err:?}"
    );

    auth.stop().await;
}
