//! `CellToBaseMsg::Chat(GmBroadcast)` routing (SS-C2): the arm fans the GM
//! line out to the listed session and logs the delivery.

use super::super::*;
use super::one_session;
use crate::cell::messages::ChatCellToBase;
use crate::test_support::{LogCapture, TestTransport};

#[tokio::test]
async fn gm_broadcast_arm_fans_out_and_logs_delivery() {
    let capture = LogCapture::install();
    let (addr, connected, entity_to_addr) = one_session(4242, false);
    {
        let mut clients = connected.lock().unwrap();
        let c = clients.get_mut(&addr).unwrap();
        c.player_entity_id = Some(4242);
        c.active_player_id = Some(77);
        c.player_name = Some("Alice".into());
        c.listed_online = true;
    }
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();

    handle_cell_message(
        CellToBaseMsg::Chat(ChatCellToBase::GmBroadcast {
            entity_id: 9,
            player_id: Some(5),
            account_id: Some(6),
            source: "native",
            args: cimmeria_wire::cell::chat::serialize_gm_broadcast("Gm", "hi"),
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

    assert_eq!(typed_transport.filter_to(addr).len(), 1);
    let row = capture
        .find_message(tracing::Level::INFO, "fanned out to every online player")
        .expect("the arm logs chat.gm_broadcast_delivered");
    assert!(row.has_field("event", "chat.gm_broadcast_delivered"));
    assert!(row.has_field("delivered", "1"));
    assert!(row.has_field("player_id", "5"));
    assert!(row.has_field("account_id", "6"));
}
