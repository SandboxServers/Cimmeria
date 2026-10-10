//! Wire guards for the `onCharacterCreateFailed` codes (Class Start v6 CS-08
//! F1): each refusal is driven through the real handler and the packet it
//! sends is decrypted and compared byte for byte. The refusals before the
//! database run with no database attached; the taken name is a live-DB test.
//!
//! Bug shape: the handler sent 2 for every malformed payload, rejected name
//! and bad skin tint, and 3 with no database. The client renders the code
//! through `error_texts`, where 2 and 3 are `CONDITION_FEEDBACK_*` rows, so a
//! rejected name read "CONDITION_FEEDBACK_PositionCheckNotBelow". The codes
//! are asserted as literals, not through `fail_code`, so reverting a
//! constant fails here too.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::encryption::MercuryEncryption;

use super::live_db_tests::{build_create_character_payload, cleanup, insert_account};
use super::seed_parity_live_db_tests::{create, default_choices};
use super::*;
use crate::test_support::{require_db_or_skip, test_default_connected_client_state, TestTransport};

/// `test_default_connected_client_state` builds its cipher from this key.
const SESSION_KEY: [u8; 32] = [0u8; 32];
const ACCOUNT_EID: u32 = 0xAAAA_0001;
/// Sentinel after `profile_live_db_tests`' `0x7000_1E04`.
const NAME_TAKEN_ACCOUNT: i32 = 0x7000_1E05;

/// Run the handler on `payload` with no database and return the one
/// message it sent: `[0x83][u16 len = 8][u32 account eid][i32 code]`.
async fn refusal_message(payload: &[u8]) -> Vec<u8> {
    refusal_message_with(payload, 7, &None).await
}

async fn refusal_message_with(
    payload: &[u8],
    account_id: u32,
    db_pool: &Option<Arc<PgPool>>,
) -> Vec<u8> {
    let transport = Arc::new(TestTransport::new());
    let dyn_transport: Arc<dyn Transport> = transport.clone();
    let addr: SocketAddr = "127.0.0.1:55830".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.account_entity_id = ACCOUNT_EID;
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));

    handle_create_character(
        &dyn_transport,
        addr,
        SESSION_KEY,
        account_id,
        payload,
        &connected,
        db_pool,
    )
    .await
    .expect("the handler answers the client");

    let sent = transport.drain();
    assert_eq!(sent.len(), 1, "exactly one packet: the refusal");
    let enc = MercuryEncryption::from_session_key(SESSION_KEY);
    let plain = enc.decrypt(&sent[0].1).expect("decrypt the refusal");
    // Flags byte in front, the 4-byte sequence footer behind (no ACKs).
    plain[1..plain.len() - 4].to_vec()
}

fn expected(code: i32) -> Vec<u8> {
    let mut out = vec![0x83, 0x08, 0x00];
    out.extend_from_slice(&ACCOUNT_EID.to_le_bytes());
    out.extend_from_slice(&code.to_le_bytes());
    out
}

/// A valid payload for char_def 1 (the handler checks the visual choices
/// only once it has a database).
fn payload(name: &str, extra: &str, char_def_id: i32, skin: i32) -> Vec<u8> {
    build_create_character_payload(name, extra, char_def_id, &[], skin)
}

#[tokio::test]
async fn empty_payload_is_not_enough_information() {
    assert_eq!(refusal_message(&[]).await, expected(10000));
}

#[tokio::test]
async fn payload_cut_before_char_def_is_not_enough_information() {
    let mut p = Vec::new();
    crate::mercury::write_wstring(&mut p, "Good Name");
    crate::mercury::write_wstring(&mut p, "");
    assert_eq!(refusal_message(&p).await, expected(10000));
}

#[tokio::test]
async fn payload_cut_inside_visual_choices_is_not_enough_information() {
    let mut p = Vec::new();
    crate::mercury::write_wstring(&mut p, "Good Name");
    crate::mercury::write_wstring(&mut p, "");
    p.extend_from_slice(&1i32.to_le_bytes());
    // Two choices announced, none sent.
    p.extend_from_slice(&2u32.to_le_bytes());
    assert_eq!(refusal_message(&p).await, expected(10000));
}

#[tokio::test]
async fn payload_cut_before_skin_tint_is_not_enough_information() {
    let mut p = payload("Good Name", "", 1, 0);
    p.truncate(p.len() - 4);
    assert_eq!(refusal_message(&p).await, expected(10000));
}

/// The UAT case: a name the format rules reject.
#[tokio::test]
async fn rejected_name_is_invalid_character_name() {
    assert_eq!(
        refusal_message(&payload("ab", "", 1, 0)).await,
        expected(20001)
    );
}

#[tokio::test]
async fn rejected_extra_name_is_invalid_character_name() {
    assert_eq!(
        refusal_message(&payload("Good Name", "<b>", 1, 0)).await,
        expected(20001)
    );
}

#[tokio::test]
async fn skin_tint_out_of_range_is_invalid_skin_color() {
    assert_eq!(
        refusal_message(&payload("Good Name", "", 1, 16)).await,
        expected(10002)
    );
}

#[tokio::test]
async fn unknown_char_def_is_invalid_character_type() {
    assert_eq!(
        refusal_message(&payload("Good Name", "", 999, 0)).await,
        expected(10001)
    );
}

/// A well-formed request with no database to write to.
#[tokio::test]
async fn no_database_is_unspecified_error() {
    assert_eq!(
        refusal_message(&payload("Good Name", "", 1, 0)).await,
        expected(10003)
    );
}

/// A name another character already has: python's `isCharacterNameAllowed`
/// refused it with the same code as a malformed name (the handler used to
/// send 1, `CONDITION_FEEDBACK_PositionCheckNotAbove`).
#[tokio::test]
async fn taken_name_is_invalid_character_name_live_db() {
    let pool = require_db_or_skip!();
    cleanup(&pool, NAME_TAKEN_ACCOUNT).await;
    insert_account(&pool, NAME_TAKEN_ACCOUNT, 0).await;
    create(&pool, NAME_TAKEN_ACCOUNT, 0, 1, "Taken Name", false).await;

    let choices = default_choices(&pool, 1).await;
    let payload = build_create_character_payload("Taken Name", "", 1, &choices, 0);
    let message = refusal_message_with(
        &payload,
        NAME_TAKEN_ACCOUNT as u32,
        &Some(Arc::new(pool.clone())),
    )
    .await;
    cleanup(&pool, NAME_TAKEN_ACCOUNT).await;
    assert_eq!(message, expected(20001));
}
