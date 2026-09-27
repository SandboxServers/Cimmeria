//! `CellToBaseMsg::Org` routing (ORG-01, ORG-06): every nested variant reaches
//! `org_dispatch`. The unserved ones are logged no-ops that send nothing;
//! ORG-05's creation variants reach the creation handlers.

use super::super::*;
use super::empty_maps;
use crate::cell::messages::OrgCellToBase;
use crate::test_support::{LogCapture, TestTransport};
use cimmeria_entity::organization::{CashDir, OrgType};

async fn route(msg: OrgCellToBase) -> Arc<TestTransport> {
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let (connected, entity_to_addr) = empty_maps();
    handle_cell_message(
        CellToBaseMsg::Org(msg),
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;
    typed_transport
}

/// `TransferCash` (the Bank's stub) and a forwarded call reach the org arm
/// and log the actor the cell named. With no session behind entity 21 the
/// stub answers nobody, and the forward is dropped as a stale actor before
/// it is served (WARN `org.actor_mismatch`).
#[tokio::test]
async fn every_org_variant_reaches_the_org_arm() {
    let capture = LogCapture::install();
    let msgs = [
        (
            OrgCellToBase::TransferCash {
                player_id: 11,
                entity_id: 21,
                org_id: 5,
                dir: CashDir::Withdraw(100),
            },
            "org.transfer_cash_unimplemented",
            tracing::Level::DEBUG,
        ),
        (
            OrgCellToBase::ForwardCellCall {
                player_id: 11,
                entity_id: 21,
                method_index: 13,
                args: vec![5, 0, 0, 0, 0, 0, 0, 0],
            },
            "org.actor_mismatch",
            tracing::Level::WARN,
        ),
    ];
    for (msg, event, level) in msgs {
        let transport = route(msg).await;
        assert!(transport.is_empty(), "{event}: nobody to answer");
        let ev = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", event))
            .unwrap_or_else(|| panic!("{event} not logged"));
        assert_eq!(ev.target, "org");
        assert_eq!(ev.level, level);
        // The actor comes from the cell's session state and is logged.
        assert!(ev.has_field("player_id", "11") && ev.has_field("entity_id", "21"));
    }
}

/// A call no packet serves yet (CM 10, the Team / Command minimap ping)
/// and the Bank's CM 19 stub answer a live session with ORG-01's pair, so
/// the press is never silent.
#[tokio::test]
async fn unserved_calls_from_a_live_session_are_answered() {
    let msgs = [
        OrgCellToBase::ForwardCellCall {
            player_id: 11,
            entity_id: 21,
            method_index: 10,
            args: [&5i32.to_le_bytes()[..], &[0u8; 12]].concat(),
        },
        OrgCellToBase::TransferCash {
            player_id: 11,
            entity_id: 21,
            org_id: 5,
            dir: CashDir::Deposit(100),
        },
    ];
    for msg in msgs {
        let kind = msg.kind();
        let typed_transport = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed_transport.clone();
        let (connected, entity_to_addr) = empty_maps();
        let addr: std::net::SocketAddr = "127.0.0.1:54321".parse().unwrap();
        let mut s = crate::test_support::test_default_connected_client_state();
        s.active_player_id = Some(11);
        s.player_entity_id = Some(21);
        s.listed_online = true;
        connected.lock().unwrap().insert(addr, s);
        entity_to_addr.lock().unwrap().insert(21, addr);
        handle_cell_message(
            CellToBaseMsg::Org(msg),
            &transport,
            &connected,
            &entity_to_addr,
            &None,
            &None,
            &None,
            "127.0.0.1",
            7777,
        )
        .await;
        // onErrorCode, then the feedback line.
        assert_eq!(typed_transport.filter_to(addr).len(), 2, "{kind}");
    }
}

/// ORG-05: `RegistrarOpen`, `Create` and `GmCreate` reach the creation
/// handlers. With no session behind entity 21, each is refused for the
/// stale actor (WARN `org.actor_mismatch`) in one INFO outcome row of its
/// own event, and nothing is sent to a client.
#[tokio::test]
async fn creation_variants_reach_the_creation_handlers() {
    let capture = LogCapture::install();
    let msgs = [
        (
            OrgCellToBase::RegistrarOpen {
                player_id: 11,
                entity_id: 21,
                npc_entity_id: 31,
                org_type: OrgType::Team,
            },
            "org.registrar_open",
        ),
        (
            OrgCellToBase::Create {
                player_id: 11,
                entity_id: 21,
                org_type: OrgType::Command,
                name: "SG-1".into(),
            },
            "org.create",
        ),
        (
            OrgCellToBase::GmCreate {
                player_id: 11,
                entity_id: 21,
                org_type: OrgType::Team,
                name: "SG-1".into(),
            },
            "org.gm_action",
        ),
    ];
    for (msg, event) in msgs {
        let transport = route(msg).await;
        assert!(transport.is_empty(), "{event}: no client to answer");
        let row = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", event) && c.fields.contains_key("outcome"))
            .unwrap_or_else(|| panic!("{event} row not logged"));
        assert_eq!(
            (row.target.as_str(), row.level),
            ("org", tracing::Level::INFO)
        );
        assert!(
            row.has_field("reason", "actor_mismatch"),
            "{event}: {row:?}"
        );
    }
    assert!(
        !capture
            .all()
            .iter()
            .any(|c| c.has_field("event", "org.create_unimplemented")),
        "the ORG-01 no-op arm is gone"
    );
}

/// A forward outside 8..=17 is refused before its bytes are decoded: CM 18
/// never leaves the cell and CM 19 has its own variant. A forward inside the
/// range whose bytes do not decode is refused with the decoder's reason.
#[tokio::test]
async fn forward_outside_8_to_17_or_malformed_is_rejected() {
    let capture = LogCapture::install();
    for method_index in [7u16, 18, 19, 94] {
        let transport = route(OrgCellToBase::ForwardCellCall {
            player_id: 11,
            entity_id: 21,
            method_index,
            // Well-formed CM 18 / CM 19 bytes: only the range stops them.
            args: vec![1, 0, 0, 0, 1, 0, 0, 0],
        })
        .await;
        assert!(transport.is_empty());
        assert!(
            capture
                .all()
                .iter()
                .any(|c| c.has_field("event", "org.forward_rejected")
                    && c.has_field("reason", "method_out_of_range")
                    && c.has_field("method_index", &method_index.to_string())),
            "{method_index} not rejected on range"
        );
    }
    assert!(
        !capture
            .all()
            .iter()
            .any(|c| c.has_field("event", "org.forward_unimplemented")),
        "an out-of-range forward reached the decoder"
    );

    // CM 13 with a forged WSTRING length.
    route(OrgCellToBase::ForwardCellCall {
        player_id: 11,
        entity_id: 21,
        method_index: 13,
        args: vec![5, 0, 0, 0, 0xFF, 0xFF, 0, 0],
    })
    .await;
    assert!(capture
        .find_event(tracing::Level::WARN, "did not decode", "truncated")
        .is_some());
}

/// A forwarded CM 9 (ORG-06) whose actor is no longer a session in the
/// world (here, no session at all) is dropped with WARN
/// `org.actor_mismatch` before any database work: the cell's named actor
/// is re-checked against the base's own session map.
#[tokio::test]
async fn forwarded_leave_from_a_stale_actor_is_dropped() {
    let capture = LogCapture::install();
    let transport = route(OrgCellToBase::ForwardCellCall {
        player_id: 11,
        entity_id: 21,
        method_index: 9,
        args: 5i32.to_le_bytes().to_vec(),
    })
    .await;
    assert!(transport.is_empty());
    let ev = capture
        .find_event(tracing::Level::WARN, "no longer matches", "actor_mismatch")
        .expect("org.actor_mismatch WARN");
    assert!(ev.has_field("event", "org.actor_mismatch"), "{ev:?}");
    assert!(ev.has_field("org_id", "5"), "{ev:?}");
    assert!(
        !capture
            .all()
            .iter()
            .any(|c| c.has_field("event", "org.leave")),
        "a stale actor must not reach the leave handler"
    );
}

/// `GmDisband` reaches the ORG-06 handler, which re-reads the access level
/// from the base's own session: with no session the caller is level 0 and
/// the disband is refused (`not_gm`) with one `org.disband` row.
#[tokio::test]
async fn gm_disband_without_a_gm_session_is_refused() {
    let capture = LogCapture::install();
    route(OrgCellToBase::GmDisband {
        player_id: 11,
        entity_id: 21,
        org_id: 5,
    })
    .await;
    let row = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "org.disband"))
        .expect("org.disband row");
    assert!(row.has_field("outcome", "rejected"), "{row:?}");
    assert!(row.has_field("reason", "not_gm"), "{row:?}");
}

/// ORG-07: `GmJoin` and `GmRank` reach their handlers. With no session
/// behind entity 21 there is no GM to re-read, so each is refused `not_gm`
/// in one INFO outcome row and nothing is sent.
#[tokio::test]
async fn gm_join_and_rank_without_a_gm_session_are_refused() {
    let capture = LogCapture::install();
    let msgs = [
        (
            OrgCellToBase::GmJoin {
                player_id: 11,
                entity_id: 21,
                org_id: 5,
                target_name: None,
            },
            "org.gm_join",
        ),
        (
            OrgCellToBase::GmRank {
                player_id: 11,
                entity_id: 21,
                target_name: "Bo".into(),
                rank: 3,
                org_id: Some(5),
            },
            "org.gm_rank",
        ),
    ];
    for (msg, event) in msgs {
        let transport = route(msg).await;
        assert!(transport.is_empty(), "{event}");
        let row = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", event))
            .unwrap_or_else(|| panic!("{event} not logged"));
        assert_eq!(row.level, tracing::Level::INFO);
        assert!(row.has_field("reason", "not_gm"), "{:?}", row.fields);
    }
}

/// A forwarded invite response (CM 8, ORG-07) whose actor is no longer a
/// session in the world is dropped with WARN `org.actor_mismatch` before
/// any invite is looked up.
#[tokio::test]
async fn forwarded_invite_response_from_a_stale_actor_is_dropped() {
    let capture = LogCapture::install();
    let request_id = cimmeria_entity::organization::BASE_INVITE_REQUEST_FLAG | 7;
    let transport = route(OrgCellToBase::ForwardCellCall {
        player_id: 11,
        entity_id: 21,
        method_index: 8,
        args: [&request_id.to_le_bytes()[..], &[1]].concat(),
    })
    .await;
    assert!(transport.is_empty());
    assert!(capture
        .find_event(
            tracing::Level::WARN,
            "no longer matches a session",
            "actor_mismatch"
        )
        .is_some());
    assert!(!capture
        .all()
        .iter()
        .any(|c| c.has_field("event", "org.invite_response")));
}

/// ORG-10: `GmInfo`, `GmList`, `GmSetPerms` and `GmReload` each reach their
/// base handler, which refuses a caller with no GM session (D-ORG13) in its
/// one `org.gm_action` row, naming the command's `action`.
#[tokio::test]
async fn org10_gm_variants_reach_their_handlers() {
    let msgs = [
        (
            OrgCellToBase::GmInfo {
                player_id: 11,
                entity_id: 21,
                target_name: Some("Bo".into()),
            },
            "gm_org_info",
        ),
        (
            OrgCellToBase::GmList {
                player_id: 11,
                entity_id: 21,
            },
            "gm_org_list",
        ),
        (
            OrgCellToBase::GmSetPerms {
                player_id: 11,
                entity_id: 21,
                org_id: 5,
                rank: 2,
                mask: 1024,
            },
            "gm_org_set_perms",
        ),
        (
            OrgCellToBase::GmReload {
                player_id: 11,
                entity_id: 21,
            },
            "gm_reload_organizations",
        ),
    ];
    for (msg, action) in msgs {
        let capture = LogCapture::install();
        let transport = route(msg).await;
        assert!(transport.is_empty(), "{action}");
        let rows: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.has_field("event", "org.gm_action"))
            .collect();
        assert_eq!(rows.len(), 1, "{action}: {rows:#?}");
        assert_eq!(rows[0].level, tracing::Level::INFO);
        assert!(rows[0].has_field("action", action), "{:?}", rows[0].fields);
        assert!(
            rows[0].has_field("reason", "not_gm"),
            "{:?}",
            rows[0].fields
        );
        assert!(rows[0].has_field("player_id", "11"), "{:?}", rows[0].fields);
    }
}

/// ORG-08: CM 13-17 from a live session reach the text and rank-editor
/// handlers, not the "not available yet" arm. With no database each ends
/// in its own `rejected` row (`no_db`) and one feedback line.
#[tokio::test]
async fn texts_and_rank_editor_reach_their_handlers() {
    let wstr = |s: &str| {
        let units: Vec<u16> = s.encode_utf16().collect();
        let mut b = (units.len() as u32).to_le_bytes().to_vec();
        units
            .iter()
            .for_each(|u| b.extend_from_slice(&u.to_le_bytes()));
        b
    };
    let org = 5i32.to_le_bytes().to_vec();
    let cases: [(u16, Vec<u8>, &str); 5] = [
        (13, [org.clone(), wstr("Hi")].concat(), "org.set_text"),
        (14, [org.clone(), wstr("Hi")].concat(), "org.set_text"),
        (
            15,
            [org.clone(), wstr("Bo"), wstr("Hi")].concat(),
            "org.set_text",
        ),
        (
            16,
            [
                org.clone(),
                2i32.to_le_bytes().to_vec(),
                1i32.to_le_bytes().to_vec(),
            ]
            .concat(),
            "org.set_rank_permissions",
        ),
        (
            17,
            [org.clone(), 2i32.to_le_bytes().to_vec(), wstr("Grunt")].concat(),
            "org.set_rank_name",
        ),
    ];
    for (method_index, args, event) in cases {
        let capture = LogCapture::install();
        let typed_transport = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = typed_transport.clone();
        let (connected, entity_to_addr) = empty_maps();
        let addr: std::net::SocketAddr = "127.0.0.1:54321".parse().unwrap();
        let mut s = crate::test_support::test_default_connected_client_state();
        s.active_player_id = Some(11);
        s.player_entity_id = Some(21);
        s.listed_online = true;
        connected.lock().unwrap().insert(addr, s);
        entity_to_addr.lock().unwrap().insert(21, addr);
        handle_cell_message(
            CellToBaseMsg::Org(OrgCellToBase::ForwardCellCall {
                player_id: 11,
                entity_id: 21,
                method_index,
                args,
            }),
            &transport,
            &connected,
            &entity_to_addr,
            &None,
            &None,
            &None,
            "127.0.0.1",
            7777,
        )
        .await;
        let row = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", event) && c.fields.contains_key("outcome"))
            .unwrap_or_else(|| panic!("CM {method_index}: no {event} row"));
        assert!(
            row.has_field("reason", "no_db"),
            "CM {method_index}: {row:?}"
        );
        assert!(
            !capture
                .all()
                .iter()
                .any(|c| c.has_field("event", "org.forward_unimplemented")),
            "CM {method_index} reached the not-available arm"
        );
        // One feedback line, no onErrorCode.
        assert_eq!(
            typed_transport.filter_to(addr).len(),
            1,
            "CM {method_index}"
        );
    }
}
