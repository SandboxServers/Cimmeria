//! The `Crafting` arm: every request is logged at `crafting` and answered
//! with a visible line on the player's own client (D-CR14).
//!
//! Revert-verifier: removing the arm's call, or the `reject` send inside
//! `handle_craft_request`, leaves the player's address with no packet and
//! fails the count below.

use super::super::*;
use super::one_session;
use crate::base::crafting::feedback::feedback_text_args;
use crate::cell::messages::{CraftRequest, CraftVerb};
use crate::mercury::{build_player_entity_method_packet, method_idx};
use crate::test_support::{LogCapture, TestTransport};

#[tokio::test]
async fn crafting_request_is_logged_and_answered_with_feedback() {
    let capture = LogCapture::install();
    let typed_transport = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed_transport.clone();
    let entity_id = 4250;
    let (addr, connected, entity_to_addr) = one_session(entity_id, false);

    handle_cell_message(
        CellToBaseMsg::Crafting(CraftRequest {
            entity_id,
            player_id: 4251,
            verb: CraftVerb::Alloy {
                blueprint_id: 42,
                current_tier_item_id: 9001,
                lower_tier_items: vec![11, 12],
            },
            allowed: 0,
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

    let sent = typed_transport.filter_to(addr);
    assert_eq!(sent.len(), 1, "one feedback line to the requesting player");
    assert_eq!(typed_transport.len(), 1, "nothing to any other address");
    let expected = build_player_entity_method_packet(
        &[0u8; 32],
        0,
        &[],
        entity_id,
        method_idx::ON_PLAYER_COMMUNICATION,
        &feedback_text_args("Alloying is not available yet."),
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    // `feedback_text_args` is speaker SYSTEM on CHAN_FEEDBACK, so this pins
    // the channel as well as the text.
    assert_eq!(sent[0], expected, "the CHAN_FEEDBACK line, byte for byte");

    let event = capture
        .find_message(tracing::Level::INFO, "crafting request")
        .expect("the request is logged");
    assert_eq!(event.target, "crafting");
    assert!(event.has_field("event", "request"), "{event:#?}");
    assert!(event.has_field("method", "alloying"), "{event:#?}");
    assert!(event.has_field("player_id", "4251"), "{event:#?}");
    assert!(
        event
            .fields
            .get("args")
            .is_some_and(|a| a.contains("lower_tier_items: [11, 12]")),
        "the log carries every argument: {event:#?}"
    );
    assert!(
        capture
            .find_event(tracing::Level::INFO, "rejected", "not_available_yet")
            .is_some(),
        "the rejection is logged with its reason"
    );
}
