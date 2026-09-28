//! A player's `.respeccraft` through the real dispatcher: the
//! `RespecCraftOpen` arm reaches the respec handler, which answers with a
//! line even when it cannot open a respec. `db_pool` is `None`, so the
//! answer is the "unavailable" line.

use super::super::*;
use super::one_session;
use crate::base::crafting::feedback::feedback_text_args;
use crate::cell::messages::RespecCraftOpen;
use crate::mercury::{build_player_entity_method_packet, method_idx};
use crate::test_support::{LogCapture, TestTransport};
use cimmeria_mercury::encryption::EncryptionVersion;

const ENTITY: u32 = 4312;
const PLAYER_ID: i32 = 4313;

#[tokio::test]
async fn respeccraft_open_is_routed_and_answered() {
    let capture = LogCapture::install();
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let (addr, connected, entity_to_addr) = one_session(ENTITY, false);
    let open = CellToBaseMsg::RespecCraftOpen(RespecCraftOpen {
        entity_id: ENTITY,
        player_id: PLAYER_ID,
    });

    handle_cell_message(
        open,
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

    assert_eq!(
        typed.filter_to(addr),
        vec![build_player_entity_method_packet(
            &[0u8; 32],
            0,
            &[],
            ENTITY,
            method_idx::ON_PLAYER_COMMUNICATION,
            &feedback_text_args("Crafting respec is unavailable right now. Nothing was changed."),
            EncryptionVersion::V1,
        )]
    );
    let event = capture
        .find_event(tracing::Level::INFO, "rejected", "unavailable")
        .expect("the refusal is logged");
    assert!(event.has_field("verb", "respeccraft"), "{event:#?}");
    assert!(
        event.has_field("player_id", &PLAYER_ID.to_string()),
        "{event:#?}"
    );
}
