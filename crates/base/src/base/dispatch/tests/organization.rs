//! The organization base-method arm (0xCF-0xD2, ORG-01): each well-formed
//! call is answered with `onErrorCode` then a feedback line (TESTING.md
//! type 8, byte-checked after decrypting what `TestTransport` captured), and
//! a malformed payload is logged and answered with nothing.

use super::super::*;
use crate::cell::messages::OrgBaseToCell;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use cimmeria_mercury::encryption::MercuryEncryption;
use tracing::Level;

const ADDR: &str = "127.0.0.1:54400";
const ENTITY_ID: u32 = 0x0000_4242;

/// Decrypt one captured packet (all-zero test key) and strip the flags byte
/// and the 4-byte seq footer, leaving the Mercury body.
fn body(packet: &[u8]) -> Vec<u8> {
    let pt = MercuryEncryption::from_session_key([0u8; 32])
        .decrypt(packet)
        .expect("decrypt");
    pt[1..pt.len() - 4].to_vec()
}

async fn call(msg_id: u8, payload: &[u8]) -> Arc<TestTransport> {
    let addr: SocketAddr = ADDR.parse().unwrap();
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(ENTITY_ID);
    state.active_player_id = Some(77);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY_ID, addr)])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    dispatch_sgw_player_base_method(
        msg_id,
        payload,
        &None,
        addr,
        &transport,
        [0u8; 32],
        &connected,
        &entity_manager,
        &None,
        &entity_to_addr,
        &None,
    )
    .await
    .expect("org base method dispatch never errors");
    typed
}

/// `organizationKick(INT32 9, WSTRING "Bo")`: the first packet is
/// `onErrorCode` (121, extended sub-slot `121 - 61 = 0x3C`) with SystemID 0,
/// InstanceID = the org id, ErrorCodeID 0; the second is the feedback line.
#[tokio::test]
async fn kick_is_answered_with_error_code_then_feedback() {
    let capture = LogCapture::install();
    let payload = [9, 0, 0, 0, 2, 0, 0, 0, 0x42, 0, 0x6F, 0];
    let transport = call(0xD1, &payload).await;

    let sent = transport.filter_to(ADDR.parse().unwrap());
    assert_eq!(sent.len(), 2, "onErrorCode + feedback line");
    #[rustfmt::skip]
    let want: [u8; 15] = [
        0xBD, 12, 0,              // extended marker, word length 4 + 1 + 7
        0x42, 0x42, 0, 0,         // entity id
        0x3C,                     // sub-slot: 121 - 61
        0,                        // SystemID ERRORCODE_SYSTEM_Ability
        9, 0, 0, 0,               // InstanceID = org id
        0, 0,                     // ErrorCodeID CONDITION_FEEDBACK_InvalidEntity
    ];
    assert_eq!(body(&sent[0]), want);
    // The feedback line is `onPlayerCommunication` (28, direct `0x9C`).
    assert_eq!(body(&sent[1])[0], 28 | 0x80);

    let ev = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "org.base_method_unimplemented"))
        .expect("decoded call logged");
    assert_eq!(ev.target, "org");
    assert_eq!(ev.level, Level::DEBUG);
    assert!(ev.has_field("method", "organizationKick"));
    assert!(ev.has_field("instance_id", "9"));
    // The actor is the session's, not anything in the payload.
    assert!(ev.has_field("entity_id", "16962"), "{:?}", ev.fields);
}

/// Each of the four ids reaches the organization arm, not the unhandled
/// WARN catch-all it used to fall into (audit A-03).
#[tokio::test]
async fn all_four_ids_reach_the_org_arm() {
    let capture = LogCapture::install();
    let ws_bo = [2u8, 0, 0, 0, 0x42, 0, 0x6F, 0];
    let cases: [(u8, Vec<u8>, &str); 4] = [
        (
            0xCF,
            [&[9u8, 0, 0, 0][..], &ws_bo].concat(),
            "organizationInvite",
        ),
        (
            0xD0,
            [&[1u8][..], &ws_bo].concat(),
            "organizationInviteByType",
        ),
        (
            0xD1,
            [&[9u8, 0, 0, 0][..], &ws_bo].concat(),
            "organizationKick",
        ),
        (
            0xD2,
            [&[9u8, 0, 0, 0][..], &ws_bo, &[6]].concat(),
            "organizationRankChange",
        ),
    ];
    for (msg_id, payload, method) in cases {
        let transport = call(msg_id, &payload).await;
        assert_eq!(transport.len(), 2, "{method}: answered");
        assert!(
            capture.all().iter().any(|c| c.has_field("method", method)),
            "{method} not decoded"
        );
    }
    assert!(
        capture
            .find_message(Level::WARN, "Unhandled SGWPlayer base method")
            .is_none(),
        "an org id fell through to the catch-all"
    );
}

/// A forged name length is rejected with a reason and gets no answer.
#[tokio::test]
async fn malformed_payload_is_logged_and_not_answered() {
    let capture = LogCapture::install();
    let payload = [9, 0, 0, 0, 0xFF, 0xFF, 0, 0, 0x42, 0];
    let transport = call(0xCF, &payload).await;
    assert!(transport.is_empty());
    assert!(capture
        .find_event(Level::WARN, "did not decode", "truncated")
        .is_some());
}

/// Like [`call`], with a live cell channel; returns what reached the client
/// and the cell.
async fn call_with_cell(msg_id: u8, payload: &[u8]) -> (Arc<TestTransport>, Vec<BaseToCellMsg>) {
    let addr: SocketAddr = ADDR.parse().unwrap();
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(ENTITY_ID);
    state.active_player_id = Some(77);
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY_ID, addr)])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let (cell_tx, mut cell_rx) = mpsc::channel(8);
    dispatch_sgw_player_base_method(
        msg_id,
        payload,
        &None,
        addr,
        &transport,
        [0u8; 32],
        &connected,
        &entity_manager,
        &Some(cell_tx),
        &entity_to_addr,
        &None,
    )
    .await
    .expect("org base method dispatch never errors");
    let mut to_cell = Vec::new();
    while let Ok(msg) = cell_rx.try_recv() {
        to_cell.push(msg);
    }
    (typed, to_cell)
}

/// The one message that reached the cell, which must be an `Org`.
fn only_org(to_cell: Vec<BaseToCellMsg>) -> OrgBaseToCell {
    assert_eq!(to_cell.len(), 1, "exactly one message to the cell");
    match to_cell.into_iter().next() {
        Some(BaseToCellMsg::Org(m)) => m,
        _ => panic!("not an Org message"),
    }
}

const WS_BO: [u8; 8] = [2, 0, 0, 0, 0x42, 0, 0x6F, 0];

/// CAT-M-02: `organizationInviteByType` with a type above Command (2) names
/// no organization. Refused with one INFO outcome row (`org_type_invalid`,
/// with the session's identity), answered, and nothing reaches the cell.
#[tokio::test]
async fn invite_by_type_rejects_type_above_command() {
    for org_type in [3u8, 255] {
        let capture = LogCapture::install();
        let payload = [&[org_type][..], &WS_BO].concat();
        let (transport, to_cell) = call_with_cell(0xD0, &payload).await;
        assert!(to_cell.is_empty(), "type {org_type} reached the cell");
        let sent = transport.filter_to(ADDR.parse().unwrap());
        assert_eq!(sent.len(), 2, "type {org_type}: error code + line");
        assert_eq!(body(&sent[0])[8..13], [0, org_type, 0, 0, 0]);
        let row = capture
            .find_event(
                Level::INFO,
                "names no organization type",
                "org_type_invalid",
            )
            .expect("outcome row");
        assert_eq!(row.target, "org");
        assert!(row.has_field("event", "org.invite_by_type"));
        assert!(row.has_field("outcome", "rejected"));
        assert!(row.has_field("player_id", "77"), "{:?}", row.fields);
        assert!(row.fields.contains_key("account_id"), "{:?}", row.fields);
    }
}

/// `/squadinvite Bo` is `organizationInviteByType(0, "Bo")` (ORG-E1 Q2).
/// The base forwards it with the session's ids and answers nothing itself:
/// the cell resolves the name and answers.
#[tokio::test]
async fn squad_invite_by_type_forwards_to_the_cell() {
    let payload = [&[0u8][..], &WS_BO].concat();
    let (transport, to_cell) = call_with_cell(0xD0, &payload).await;
    assert!(transport.is_empty());
    assert_eq!(
        only_org(to_cell),
        OrgBaseToCell::SquadInvite {
            player_id: 77,
            entity_id: ENTITY_ID,
            target_name: "Bo".into(),
        }
    );
}

/// `organizationKick` routes on the id (D-ORG05): the first squad id is
/// forwarded; the last Team/Command id keeps ORG-01's answer until ORG-07.
#[tokio::test]
async fn kick_routes_on_the_squad_id_boundary() {
    use cimmeria_entity::organization::SQUAD_ORG_ID_MIN;
    let payload = [&SQUAD_ORG_ID_MIN.to_le_bytes()[..], &WS_BO].concat();
    let (transport, to_cell) = call_with_cell(0xD1, &payload).await;
    assert!(transport.is_empty());
    assert_eq!(
        only_org(to_cell),
        OrgBaseToCell::SquadKick {
            player_id: 77,
            entity_id: ENTITY_ID,
            org_id: SQUAD_ORG_ID_MIN,
            target_name: "Bo".into(),
        }
    );

    let payload = [&(SQUAD_ORG_ID_MIN - 1).to_le_bytes()[..], &WS_BO].concat();
    let (transport, to_cell) = call_with_cell(0xD1, &payload).await;
    assert!(to_cell.is_empty());
    assert_eq!(transport.len(), 2, "ORG-01 answer");
}

/// Types 1 and 2 keep ORG-01's answer until ORG-07, and never reach the
/// cell (CAT-M-02: they never create implicitly).
#[tokio::test]
async fn team_and_command_invite_by_type_are_not_forwarded() {
    for org_type in [1u8, 2] {
        let payload = [&[org_type][..], &WS_BO].concat();
        let (transport, to_cell) = call_with_cell(0xD0, &payload).await;
        assert!(to_cell.is_empty());
        assert_eq!(transport.len(), 2);
    }
}

/// Negative seam: with no cell channel the squad invite cannot be
/// forwarded. WARN `org.squad_forward_failed` (`cell_unreachable`), the
/// player still gets ORG-01's answer, and the squad action gets its one
/// outcome row on the `squad` target, since the cell never saw it.
#[tokio::test]
async fn squad_forward_failure_warns_and_logs_the_outcome() {
    let capture = LogCapture::install();
    let payload = [&[0u8][..], &WS_BO].concat();
    let transport = call(0xD0, &payload).await;
    assert_eq!(transport.len(), 2, "answered");
    let warn = capture
        .find_event(Level::WARN, "could not reach the cell", "cell_unreachable")
        .expect("WARN org.squad_forward_failed");
    assert!(warn.has_field("event", "org.squad_forward_failed"));
    let row = capture
        .all()
        .into_iter()
        .find(|c| c.target == "squad" && c.has_field("event", "squad.invite"))
        .expect("squad outcome row");
    assert_eq!(row.level, Level::INFO);
    assert!(row.has_field("outcome", "rejected") && row.has_field("reason", "cell_unreachable"));
    assert!(row.has_field("player_id", "77"));
}
