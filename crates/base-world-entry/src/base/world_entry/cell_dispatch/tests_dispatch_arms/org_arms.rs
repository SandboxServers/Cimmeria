//! `CellToBaseMsg::Org` routing (ORG-01): every nested variant reaches
//! `org_dispatch` and is a logged no-op that sends nothing.

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

#[tokio::test]
async fn every_org_variant_reaches_the_org_arm() {
    let capture = LogCapture::install();
    let msgs = [
        (
            OrgCellToBase::Create {
                player_id: 11,
                entity_id: 21,
                org_type: OrgType::Command,
                name: "SG-1".into(),
            },
            "org.create_unimplemented",
        ),
        (
            OrgCellToBase::TransferCash {
                player_id: 11,
                entity_id: 21,
                org_id: 5,
                dir: CashDir::Withdraw(100),
            },
            "org.transfer_cash_unimplemented",
        ),
        (
            OrgCellToBase::ForwardCellCall {
                player_id: 11,
                entity_id: 21,
                method_index: 13,
                args: vec![5, 0, 0, 0, 0, 0, 0, 0],
            },
            "org.forward_unimplemented",
        ),
    ];
    for (msg, event) in msgs {
        let transport = route(msg).await;
        assert!(transport.is_empty(), "{event}: a no-op sends nothing");
        let ev = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", event))
            .unwrap_or_else(|| panic!("{event} not logged"));
        assert_eq!(ev.target, "org");
        assert_eq!(ev.level, tracing::Level::DEBUG);
        // The actor comes from the cell's session state and is logged.
        assert!(ev.has_field("player_id", "11") && ev.has_field("entity_id", "21"));
    }
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
