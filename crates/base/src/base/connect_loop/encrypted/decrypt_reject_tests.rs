//! Guards for the decrypt-reject seam on an established session.
//!
//! Colo, 2026-09-26: a tester's client kept re-sending its plaintext
//! `baseAppLogin` every 300 ms after the server had registered the
//! encrypted channel. Each retry was logged as
//! `Decryption failed (bad HMAC?)` with `disconnect_reason = "decrypt_fail"`
//! and `ciphertext length 25`, although nothing was torn down. These tests
//! drive the same 41-byte datagram through `handle_encrypted_datagram` and
//! pin three things: the retry gets its own `reason`, a real decrypt failure
//! keeps `reason = "decrypt_fail"`, and neither drops the session or claims
//! a disconnect.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::test_transport::TestTransport;
use cimmeria_mercury::transport::Transport;
use tracing::Level;

use crate::base::ConnectedClientState;
use crate::test_support::{test_default_connected_client_state, LogCapture};

use super::decrypt_reject::{REASON_DECRYPT_FAIL, REASON_LOGIN_RETRY_ON_CHANNEL};
use super::handle_encrypted_datagram;

const ACCOUNT_ID: u32 = 6;

/// The client's `baseAppLogin` datagram, byte for byte as it reaches the
/// server: flags `0x41`, msg `0x00` with a u16 length of 25, the request
/// header (reply id + next-request offset), the 25-byte payload
/// (account id, ticket length, 20-character ticket), then the footers
/// (first-request offset, sequence).
fn plaintext_base_app_login(seq: u32) -> Vec<u8> {
    let mut raw = vec![0x41, 0x00];
    raw.extend_from_slice(&25u16.to_le_bytes());
    raw.extend_from_slice(&0x0000_0001u32.to_le_bytes()); // reply id
    raw.extend_from_slice(&0u16.to_le_bytes()); // next request offset
    raw.extend_from_slice(&ACCOUNT_ID.to_le_bytes());
    raw.push(20);
    raw.extend_from_slice(b"733421AB00112233CDEF");
    raw.extend_from_slice(&1u16.to_le_bytes()); // first request offset
    raw.extend_from_slice(&seq.to_le_bytes());
    raw
}

struct Rig {
    addr: SocketAddr,
    transport: Arc<dyn Transport>,
    connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
}

fn rig() -> Rig {
    let addr: SocketAddr = "127.0.0.1:22449".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.account_id = ACCOUNT_ID;
    let mut map = HashMap::new();
    map.insert(addr, state);
    Rig {
        addr,
        transport: Arc::new(TestTransport::new()),
        connected: Arc::new(Mutex::new(map)),
    }
}

async fn deliver(rig: &Rig, datagram: &[u8]) {
    let (enc, key, pending_acks) = {
        let map = rig.connected.lock().unwrap();
        let s = &map[&rig.addr];
        (s.enc.clone(), s.key, Arc::clone(&s.pending_acks))
    };
    handle_encrypted_datagram(
        &rig.transport,
        rig.addr,
        datagram,
        enc,
        key,
        ACCOUNT_ID,
        &pending_acks,
        &rig.connected,
        &None,
        &None,
        &Arc::new(Mutex::new(EntityManager::new())),
        &None,
        &Arc::new(Mutex::new(HashMap::new())),
    )
    .await
    .unwrap();
}

/// The colo datagram: 41 bytes, which the v1 decrypt path splits into a
/// 25-byte "ciphertext" and a 16-byte tag. This pins the size match the
/// diagnosis rests on, so a change to either side shows up here first.
#[test]
fn plaintext_login_is_the_41_byte_datagram_behind_the_colo_error() {
    let raw = plaintext_base_app_login(3);
    assert_eq!(raw.len(), 41);
    let (_, ticket) = crate::base::login::parse_baseapp_login(&raw).unwrap();
    assert_eq!(ticket.len(), 20);

    let enc = test_default_connected_client_state().enc;
    let err = enc.decrypt(&raw).unwrap_err().to_string();
    assert!(
        err.contains("ciphertext length 25"),
        "v1 decrypt must fail on the length check, as on the colo: {err}"
    );
}

#[tokio::test]
async fn login_retry_on_an_established_channel_is_named_and_keeps_the_session() {
    let rig = rig();
    let guard = LogCapture::install();

    deliver(&rig, &plaintext_base_app_login(3)).await;

    let row = guard
        .find_event(
            Level::WARN,
            "retrying baseAppLogin",
            REASON_LOGIN_RETRY_ON_CHANNEL,
        )
        .unwrap_or_else(|| panic!("no login-retry row; saw {:#?}", guard.all()));
    assert!(row.has_field("account_id", &ACCOUNT_ID.to_string()));
    assert!(
        guard
            .find_event(Level::WARN, "", REASON_DECRYPT_FAIL)
            .is_none(),
        "a plaintext login retry must not be reported as a decrypt failure"
    );
    assert!(
        guard
            .all()
            .iter()
            .all(|c| !c.fields.contains_key("disconnect_reason")),
        "nothing disconnects here, so no row may carry disconnect_reason"
    );
    assert!(
        rig.connected.lock().unwrap().contains_key(&rig.addr),
        "a dropped retry must leave the session registered"
    );
}

#[tokio::test]
async fn undecryptable_datagram_is_a_decrypt_fail_without_a_disconnect() {
    let rig = rig();
    let guard = LogCapture::install();

    // 48 bytes: block-aligned, so it reaches the HMAC check and fails there.
    deliver(&rig, &[0x5Au8; 48]).await;

    guard
        .find_event(Level::WARN, "Decryption failed", REASON_DECRYPT_FAIL)
        .unwrap_or_else(|| panic!("no decrypt_fail row; saw {:#?}", guard.all()));
    assert!(
        guard
            .find_event(Level::WARN, "", REASON_LOGIN_RETRY_ON_CHANNEL)
            .is_none(),
        "garbage must not be classed as a login retry"
    );
    assert!(
        guard
            .all()
            .iter()
            .all(|c| !c.fields.contains_key("disconnect_reason")),
        "nothing disconnects here, so no row may carry disconnect_reason"
    );
    assert!(
        rig.connected.lock().unwrap().contains_key(&rig.addr),
        "a decrypt failure must leave the session registered"
    );
}
