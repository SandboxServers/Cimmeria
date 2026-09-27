//! The router: decoding, the ORG-01 answer for everything not yet served,
//! and the id-range boundaries between the squad handlers and the rest.

use cimmeria_entity::organization::{BASE_INVITE_REQUEST_FLAG, SQUAD_ORG_ID_MIN};

use super::*;
use crate::test_support::{make_space_manager_with_player, LogCapture};

const NOT_AVAILABLE: &str = "Organizations are not available yet.";

/// CM 13 used to read only the org id; the MOTD `WSTRING` was dropped
/// (audit A-02). The dispatcher decodes it. Squads have no MOTD, so a
/// squad-range id is answered here (two UTF-16 units reach the DEBUG log,
/// with `route = rejected`): `onErrorCode` with the org id, then the
/// feedback line. A Team or Command id is forwarded to the base (ORG-07)
/// with the raw arguments.
#[tokio::test]
async fn motd_is_decoded_and_routed() {
    let capture = LogCapture::install();
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();
    let text = [2u8, 0, 0, 0, 0x48, 0, 0x69, 0];
    let squad_args = [&SQUAD_ORG_ID_MIN.to_le_bytes()[..], &text].concat();
    assert!(dispatch(11, MOTD, &squad_args, &tx, &mut mgr).await);
    let ev = capture
        .find_message(Level::DEBUG, "UNIMPLEMENTED: organizationMOTD")
        .expect("decoded MOTD log");
    assert_eq!(ev.target, "org");
    assert!(ev.has_field("text_units", "2"), "{:?}", ev.fields);
    assert!(ev.has_field("route", "rejected"), "{:?}", ev.fields);
    assert_eq!(
        to(&drain(&mut rx), 11),
        rejection(SQUAD_ORG_ID_MIN, NOT_AVAILABLE)
    );

    let team_args = [&7i32.to_le_bytes()[..], &text].concat();
    assert!(dispatch(11, MOTD, &team_args, &tx, &mut mgr).await);
    match rx.try_recv() {
        Ok(CellToBaseMsg::Org(crate::cell::messages::OrgCellToBase::ForwardCellCall {
            player_id,
            entity_id,
            method_index,
            args,
        })) => {
            assert_eq!((player_id, entity_id, method_index), (1, 11, MOTD));
            assert_eq!(args, team_args);
        }
        other => panic!("expected the MOTD forwarded to the base, got {other:?}"),
    }
}

/// Every method 8-19 answers. CM 8 and 18 carry no org id (instance 0);
/// here the caller is not an initialised player, so the squad handlers
/// answer CM 8 and 18 with a refusal pair too, and CM 9 (id 5, a
/// Team/Command id) cannot be forwarded without a character and gets
/// ORG-01's answer.
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

/// CM 9 routes on the org id: the last Team/Command id is forwarded to the
/// base (ORG-06) with the caller's own character and the raw arguments, the
/// first squad id reaches the squad handler (which refuses a squad the
/// caller is not in with its own line).
#[tokio::test]
async fn leave_routes_on_the_squad_id_boundary() {
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();
    let below = SQUAD_ORG_ID_MIN - 1;
    dispatch(11, LEAVE, &below.to_le_bytes(), &tx, &mut mgr).await;
    match rx.try_recv() {
        Ok(CellToBaseMsg::Org(crate::cell::messages::OrgCellToBase::ForwardCellCall {
            player_id,
            entity_id,
            method_index,
            args,
        })) => {
            assert_eq!((player_id, entity_id, method_index), (1, 11, LEAVE));
            assert_eq!(args, below.to_le_bytes().to_vec());
        }
        other => panic!("expected the leave forwarded to the base, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "nothing else is sent");
    dispatch(11, LEAVE, &SQUAD_ORG_ID_MIN.to_le_bytes(), &tx, &mut mgr).await;
    assert_eq!(
        to(&drain(&mut rx), 11),
        rejection(SQUAD_ORG_ID_MIN, "You are not in that squad.")
    );
}

/// CM 8 routes on the request id: a base-issued id (bit 29) is forwarded
/// to the base (ORG-07) with the caller's character; a cell id reaches the
/// squad handler.
#[tokio::test]
async fn invite_response_routes_on_the_base_flag() {
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();
    let base_id = BASE_INVITE_REQUEST_FLAG | 1;
    let args = [&base_id.to_le_bytes()[..], &[1]].concat();
    dispatch(11, INVITE_RESPONSE, &args, &tx, &mut mgr).await;
    match rx.try_recv() {
        Ok(CellToBaseMsg::Org(crate::cell::messages::OrgCellToBase::ForwardCellCall {
            player_id,
            entity_id,
            method_index,
            args: fwd,
        })) => {
            assert_eq!(
                (player_id, entity_id, method_index),
                (1, 11, INVITE_RESPONSE)
            );
            assert_eq!(fwd, args);
        }
        other => panic!("expected the response forwarded to the base, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "nothing else is sent");
    let args = [&1i32.to_le_bytes()[..], &[1]].concat();
    dispatch(11, INVITE_RESPONSE, &args, &tx, &mut mgr).await;
    assert_eq!(
        to(&drain(&mut rx), 11),
        rejection(0, "That invitation is no longer valid.")
    );
}

/// CAT-M-16: no strike-team request is ever issued, so every
/// `strikeTeamResponse` (CM 11) is unsolicited: one INFO
/// `org.strike_team_response` row (`reason = unsolicited`), the refusal
/// pair, and nothing reaches the base, whatever the id routes to.
#[tokio::test]
async fn strike_team_response_rejected_unsolicited() {
    assert_unsolicited(STRIKE_TEAM_RESPONSE, "org.strike_team_response").await;
}

/// CAT-M-17: likewise `pvpOrganizationLeaveResponse` (CM 12): no PvP-leave
/// request is ever issued.
#[tokio::test]
async fn pvp_leave_response_rejected_unsolicited() {
    assert_unsolicited(PVP_LEAVE_RESPONSE, "org.pvp_leave_response").await;
}

async fn assert_unsolicited(method_index: u16, event: &str) {
    for org_id in [5, SQUAD_ORG_ID_MIN, 0] {
        let capture = LogCapture::install();
        let mut mgr = world(&["Alice"]);
        let (tx, mut rx) = channel();
        let args = [&org_id.to_le_bytes()[..], &[1]].concat();
        assert!(dispatch(11, method_index, &args, &tx, &mut mgr).await);
        // `drain` panics on anything but a client call, so a forward to the
        // base fails here.
        assert_eq!(
            to(&drain(&mut rx), 11),
            rejection(org_id, "There is no request to answer."),
            "CM {method_index}, id {org_id}"
        );
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.has_field("event", event))
            .collect();
        assert_eq!(rows.len(), 1, "one outcome row: {rows:#?}");
        assert_eq!(rows[0].level, Level::INFO);
        assert!(rows[0].has_field("outcome", "rejected"));
        assert!(rows[0].has_field("reason", "unsolicited"));
        assert!(rows[0].has_field("player_id", "1"), "{:?}", rows[0].fields);
    }
}

/// CM 19 with a Team or Command id is forwarded as `TransferCash` (the
/// Bank campaign's route, ORG-API) with the caller's own character; a squad
/// id is answered on the cell.
#[tokio::test]
async fn transfer_cash_routes_to_the_base_as_transfer_cash() {
    use cimmeria_entity::organization::CashDir;
    let mut mgr = world(&["Alice"]);
    let (tx, mut rx) = channel();
    let args = [&5i32.to_le_bytes()[..], &(-250i32).to_le_bytes()].concat();
    dispatch(11, TRANSFER_CASH, &args, &tx, &mut mgr).await;
    match rx.try_recv() {
        Ok(CellToBaseMsg::Org(crate::cell::messages::OrgCellToBase::TransferCash {
            player_id,
            entity_id,
            org_id,
            dir,
        })) => {
            assert_eq!((player_id, entity_id, org_id), (1, 11, 5));
            assert_eq!(Some(dir), CashDir::from_wire(-250));
        }
        other => panic!("expected TransferCash to the base, got {other:?}"),
    }
    let args = [&SQUAD_ORG_ID_MIN.to_le_bytes()[..], &100i32.to_le_bytes()].concat();
    dispatch(11, TRANSFER_CASH, &args, &tx, &mut mgr).await;
    assert_eq!(
        to(&drain(&mut rx), 11),
        rejection(SQUAD_ORG_ID_MIN, NOT_AVAILABLE)
    );
}
