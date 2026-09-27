//! `CellToBaseMsg::Org` routing (ORG-01): every nested variant reaches
//! `org_dispatch` and is a logged no-op that sends nothing.

use super::super::*;
use super::empty_maps;
use crate::cell::messages::OrgCellToBase;
use crate::test_support::{LogCapture, TestTransport};
use cimmeria_entity::organization::OrgType;

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
                amount: -100,
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
