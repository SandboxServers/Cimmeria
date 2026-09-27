//! The organization base-method arm (0xCF-0xD2): routing and the answers
//! the base gives itself. A squad rank change (no handler; ORG-E1
//! follow-up one) gets `onErrorCode` then a feedback line (TESTING.md type 8,
//! byte-checked after decrypting what `TestTransport` captured); Team and
//! Command calls reach the ORG-07 handlers, whose behaviour is tested
//! against a live database in `base-session` (`organization::handlers`);
//! here, with no database, each is refused with one line and one outcome
//! row. A malformed payload is logged and answered with nothing.

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

/// `organizationRankChange` with a squad id: the first packet is
/// `onErrorCode` (121, extended sub-slot `121 - 61 = 0x3C`) with SystemID 0,
/// InstanceID = the org id, ErrorCodeID 0; the second is the feedback line.
/// `/squadpromote` is not known to use 0xD2, so it has no handler yet.
#[tokio::test]
async fn squad_rank_change_is_answered_with_error_code_then_feedback() {
    let capture = LogCapture::install();
    let squad = 0x4000_0009i32;
    let payload = [&squad.to_le_bytes()[..], &WS_BO, &[2]].concat();
    let transport = call(0xD2, &payload).await;

    let sent = transport.filter_to(ADDR.parse().unwrap());
    assert_eq!(sent.len(), 2, "onErrorCode + feedback line");
    let id = squad.to_le_bytes();
    #[rustfmt::skip]
    let want: [u8; 15] = [
        0xBD, 12, 0,              // extended marker, word length 4 + 1 + 7
        0x42, 0x42, 0, 0,         // entity id
        0x3C,                     // sub-slot: 121 - 61
        0,                        // SystemID ERRORCODE_SYSTEM_Ability
        id[0], id[1], id[2], id[3], // InstanceID = org id
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
    assert!(ev.has_field("method", "organizationRankChange"));
    // The actor is the session's, not anything in the payload.
    assert!(ev.has_field("entity_id", "16962"), "{:?}", ev.fields);
}

/// Each of the four ids reaches the organization arm, not the unhandled
/// WARN catch-all it used to fall into (audit A-03). The Team and Command
/// forms reach the ORG-07 handlers: each ends in its outcome row and one
/// feedback line (here the invitee is not online, or there is no database).
#[tokio::test]
async fn all_four_ids_reach_the_org_arm() {
    let capture = LogCapture::install();
    let cases: [(u8, Vec<u8>, &str); 4] = [
        (0xCF, [&[9u8, 0, 0, 0][..], &WS_BO].concat(), "org.invite"),
        (0xD0, [&[1u8][..], &WS_BO].concat(), "org.invite"),
        (0xD1, [&[9u8, 0, 0, 0][..], &WS_BO].concat(), "org.kick"),
        (
            0xD2,
            [&[9u8, 0, 0, 0][..], &WS_BO, &[6]].concat(),
            "org.rank_change",
        ),
    ];
    for (msg_id, payload, event) in cases {
        let before = capture
            .all()
            .iter()
            .filter(|c| c.has_field("event", event))
            .count();
        let transport = call(msg_id, &payload).await;
        assert_eq!(transport.len(), 1, "{msg_id:#04x}: one feedback line");
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.has_field("event", event))
            .collect();
        assert_eq!(rows.len(), before + 1, "{msg_id:#04x}: one {event} row");
        let row = rows.last().unwrap();
        assert!(row.has_field("outcome", "rejected"), "{:?}", row.fields);
        assert!(row.has_field("player_id", "77"), "{:?}", row.fields);
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
/// forwarded; the last Team/Command id goes to the base's kick handler
/// (with no database here, refused `no_db` with one line).
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

    let capture = LogCapture::install();
    let payload = [&(SQUAD_ORG_ID_MIN - 1).to_le_bytes()[..], &WS_BO].concat();
    let (transport, to_cell) = call_with_cell(0xD1, &payload).await;
    assert!(to_cell.is_empty());
    assert_eq!(transport.len(), 1, "the kick handler's line");
    assert!(capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "org.kick") && c.has_field("reason", "no_db")));
}

/// Types 1 and 2 go to the base's invite handler and never reach the cell
/// (CAT-M-02: the cell would found a squad). With nobody online by that
/// name, the handler refuses with one line.
#[tokio::test]
async fn team_and_command_invite_by_type_are_not_forwarded() {
    for org_type in [1u8, 2] {
        let capture = LogCapture::install();
        let payload = [&[org_type][..], &WS_BO].concat();
        let (transport, to_cell) = call_with_cell(0xD0, &payload).await;
        assert!(to_cell.is_empty());
        assert_eq!(transport.len(), 1);
        assert!(capture.all().iter().any(
            |c| c.has_field("event", "org.invite") && c.has_field("reason", "target_not_found")
        ));
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

/// The unreachable-cell refusal counts on `squad_actions_total` like every
/// cell-side outcome row, under the cell's action labels (`invite`, `kick`),
/// so the metric does not undercount squad refusals when the cell is down.
#[tokio::test]
async fn squad_forward_failure_counts_on_squad_actions_total() {
    use cimmeria_observability::testing::{counter_total, install};
    install();
    let labels = |action| {
        [
            ("action", action),
            ("outcome", "rejected"),
            ("reason", "cell_unreachable"),
        ]
    };
    let invite_before = counter_total("squad_actions_total", &labels("invite"));
    let kick_before = counter_total("squad_actions_total", &labels("kick"));

    let invite = [&[0u8][..], &WS_BO].concat();
    call(0xD0, &invite).await;
    // `organizationKick` with an org id in the squad range (D-ORG05).
    let kick = [&0x4000_0000i32.to_le_bytes()[..], &WS_BO].concat();
    call(0xD1, &kick).await;

    // At least one, not exactly one: under `cargo test` the other tests in
    // this module that forward with no cell share the process-wide table
    // (nextest runs each test alone, where it is exactly one).
    assert!(counter_total("squad_actions_total", &labels("invite")) > invite_before);
    assert!(counter_total("squad_actions_total", &labels("kick")) > kick_before);
}

/// `/squadinvite Bo` with Bo online: the base forwards it with the session's
/// ids when Bo's cached Ignore list does not hold the inviter, and refuses
/// it itself when it does (ORG-07, carried from ORG-03): one `squad.invite`
/// outcome row (`reason = ignored`, on `squad`, since the cell never sees
/// it), a counted refusal, one feedback line, and nothing to the cell.
#[tokio::test]
async fn squad_invite_to_a_player_who_ignores_the_inviter_is_refused() {
    use cimmeria_base_session::base::contact_list::ignore::IgnoreCache;
    use cimmeria_base_session::base::organization::handlers::answer::IGNORED_TEXT;

    for ignoring in [false, true] {
        let capture = LogCapture::install();
        let addr: SocketAddr = ADDR.parse().unwrap();
        let bo_addr: SocketAddr = "127.0.0.1:54401".parse().unwrap();
        let typed = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed.clone();
        let mut me = test_default_connected_client_state();
        me.player_entity_id = Some(ENTITY_ID);
        me.active_player_id = Some(77);
        me.player_name = Some("Al".into());
        me.listed_online = true;
        let mut bo = test_default_connected_client_state();
        bo.player_entity_id = Some(0x4343);
        bo.active_player_id = Some(78);
        bo.player_name = Some("Bo".into());
        bo.listed_online = true;
        if ignoring {
            bo.ignore = IgnoreCache::with_player_ids(
                ["Al".to_string()].into_iter().collect(),
                [77].into_iter().collect(),
            );
        }
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, me), (bo_addr, bo)])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
            (ENTITY_ID, addr),
            (0x4343, bo_addr),
        ])));
        let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
        let (cell_tx, mut cell_rx) = mpsc::channel(8);
        let payload = [&[0u8][..], &WS_BO].concat();
        dispatch_sgw_player_base_method(
            0xD0,
            &payload,
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
        .unwrap();
        let forwarded = cell_rx.try_recv().is_ok();
        assert_eq!(forwarded, !ignoring, "ignoring = {ignoring}");
        if !ignoring {
            assert!(typed.is_empty(), "the cell answers a forwarded invite");
            continue;
        }
        assert!(typed.filter_to(bo_addr).is_empty(), "Bo is never asked");
        let sent = typed.filter_to(addr);
        assert_eq!(sent.len(), 1, "one line");
        let text: Vec<u8> = IGNORED_TEXT
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        assert!(
            body(&sent[0]).windows(text.len()).any(|w| w == text),
            "the Ignore line"
        );
        let row = capture
            .all()
            .into_iter()
            .find(|c| c.target == "squad" && c.has_field("event", "squad.invite"))
            .expect("squad outcome row");
        assert_eq!(row.level, Level::INFO);
        assert!(row.has_field("reason", "ignored") && row.has_field("target_player_id", "78"));
    }
}
