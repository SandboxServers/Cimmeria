//! The router: decoding, the ORG-01 answer for everything not yet served,
//! and the id-range boundaries between the squad handlers and the rest.

use cimmeria_entity::organization::{BASE_INVITE_REQUEST_FLAG, SQUAD_ORG_ID_MIN};

use super::*;
use crate::test_support::{make_space_manager_with_player, LogCapture};

const NOT_AVAILABLE: &str = "Organizations are not available yet.";

/// CM 13 used to read only the org id; the MOTD `WSTRING` was dropped
/// (audit A-02). The dispatcher decodes it (two UTF-16 units reach the
/// DEBUG log) and answers: `onErrorCode` with the org id, then the
/// feedback line.
#[tokio::test]
async fn motd_is_decoded_and_answered() {
    let capture = LogCapture::install();
    let mut mgr = make_space_manager_with_player(1);
    let (tx, mut rx) = channel();
    let args = [7, 0, 0, 0, 2, 0, 0, 0, 0x48, 0, 0x69, 0];
    assert!(dispatch(1, MOTD, &args, &tx, &mut mgr).await);
    let ev = capture
        .find_message(Level::DEBUG, "UNIMPLEMENTED: organizationMOTD")
        .expect("decoded MOTD log");
    assert_eq!(ev.target, "org");
    assert!(ev.has_field("text_units", "2"), "{:?}", ev.fields);

    let sent = drain(&mut rx);
    assert_eq!(to(&sent, 1), rejection(7, NOT_AVAILABLE));
}

/// Every method 8-19 answers. CM 8 and 18 carry no org id (instance 0);
/// here the caller is not an initialised player, so the squad handlers
/// answer CM 8, 9 (id 5 is a Team/Command id, so ORG-01's answer) and 18
/// with a refusal pair too.
#[tokio::test]
async fn every_org_method_is_answered() {
    let mut mgr = make_space_manager_with_player(1);
    let (tx, mut rx) = channel();
    let cases: [(u16, Vec<u8>, i32); 12] = [
        (8, vec![1, 0, 0, 0, 1], 0),
        (9, vec![5, 0, 0, 0], 5),
        (10, [&[5u8, 0, 0, 0][..], &[0; 12]].concat(), 5),
        (11, vec![5, 0, 0, 0, 1], 5),
        (12, vec![5, 0, 0, 0, 1], 5),
        (13, vec![5, 0, 0, 0, 0, 0, 0, 0], 5),
        (14, vec![5, 0, 0, 0, 0, 0, 0, 0], 5),
        (15, vec![5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0], 5),
        (16, vec![5, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0], 5),
        (17, vec![5, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 0x41, 0], 5),
        (18, vec![1, 0, 0, 0], 0),
        (19, vec![5, 0, 0, 0, 1, 0, 0, 0], 5),
    ];
    for (idx, args, instance) in cases {
        assert!(dispatch(1, idx, &args, &tx, &mut mgr).await);
        let sent = drain(&mut rx);
        assert_eq!(sent.len(), 2, "CM {idx}");
        assert_eq!(sent[0].1, 121, "CM {idx}");
        assert_eq!(&sent[0].2[1..5], &instance.to_le_bytes(), "CM {idx}");
    }
}

/// A WSTRING whose declared length runs past the payload is rejected
/// with a reason, not read past or allocated, and not answered.
#[tokio::test]
async fn forged_wstring_length_is_rejected() {
    let capture = LogCapture::install();
    let mut mgr = make_space_manager_with_player(1);
    let (tx, mut rx) = channel();
    let args = [7, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0x48, 0];
    assert!(dispatch(1, NOTE, &args, &tx, &mut mgr).await);
    assert!(capture
        .find_event(Level::WARN, "did not decode", "truncated")
        .is_some());
    assert!(drain(&mut rx).is_empty());
}

#[tokio::test]
async fn indices_outside_8_to_19_are_not_handled() {
    let mut mgr = make_space_manager_with_player(1);
    let (tx, _rx) = channel();
    assert!(!dispatch(1, 7, &[], &tx, &mut mgr).await);
    assert!(!dispatch(1, 20, &[], &tx, &mut mgr).await);
}

/// CM 9 routes on the org id: the last Team/Command id gets ORG-01's
/// answer, the first squad id reaches the squad handler (which refuses a
/// squad the caller is not in with its own line).
#[tokio::test]
async fn leave_routes_on_the_squad_id_boundary() {
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();
    let below = SQUAD_ORG_ID_MIN - 1;
    dispatch(11, LEAVE, &below.to_le_bytes(), &tx, &mut mgr).await;
    assert_eq!(to(&drain(&mut rx), 11), rejection(below, NOT_AVAILABLE));
    dispatch(11, LEAVE, &SQUAD_ORG_ID_MIN.to_le_bytes(), &tx, &mut mgr).await;
    assert_eq!(
        to(&drain(&mut rx), 11),
        rejection(SQUAD_ORG_ID_MIN, "You are not in that squad.")
    );
}

/// CM 8 routes on the request id: a base-issued id (bit 29) gets ORG-01's
/// answer; a cell id reaches the squad handler.
#[tokio::test]
async fn invite_response_routes_on_the_base_flag() {
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();
    let base_id = BASE_INVITE_REQUEST_FLAG | 1;
    let args = [&base_id.to_le_bytes()[..], &[1]].concat();
    dispatch(11, INVITE_RESPONSE, &args, &tx, &mut mgr).await;
    assert_eq!(to(&drain(&mut rx), 11), rejection(0, NOT_AVAILABLE));
    let args = [&1i32.to_le_bytes()[..], &[1]].concat();
    dispatch(11, INVITE_RESPONSE, &args, &tx, &mut mgr).await;
    assert_eq!(
        to(&drain(&mut rx), 11),
        rejection(0, "That invitation is no longer valid.")
    );
}
