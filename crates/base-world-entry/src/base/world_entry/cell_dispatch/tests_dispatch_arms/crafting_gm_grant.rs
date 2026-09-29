//! `.craftkit` and `.learnblueprint` through the real dispatcher: each
//! grant reaches its own handler, which re-checks the caller's access
//! level. A caller below GameMaster (a forged or mis-routed message; the
//! cell's console gate would not send it) reads the refusal of the command
//! it ran, and nothing needs a database to get that far.

use super::super::*;
use super::one_session;
use crate::cell::messages::PluginMsg;
use crate::cell::messages::{GmCraftGrant, GmCraftGrantKind};
use crate::mercury::{build_player_entity_method_packet, method_idx};
use crate::test_support::{LogCapture, TestTransport};
use cimmeria_base_crafting::base::crafting::feedback::feedback_text_args;
use cimmeria_mercury::encryption::EncryptionVersion;

const ENTITY: u32 = 4273;
const PLAYER_ID: i32 = 4274;

fn feedback_packet(text: &str) -> Vec<u8> {
    build_player_entity_method_packet(
        &[0u8; 32],
        0,
        &[],
        ENTITY,
        method_idx::ON_PLAYER_COMMUNICATION,
        &feedback_text_args(text),
        EncryptionVersion::V1,
    )
}

/// Dispatch `grant` from a caller below GameMaster; what the caller got,
/// and the log.
async fn dispatch_from_non_gm(
    grant: GmCraftGrantKind,
) -> (Vec<Vec<u8>>, crate::test_support::LogCaptureGuard) {
    let capture = LogCapture::install();
    let typed = Arc::new(TestTransport::new());
    let transport: Arc<dyn Transport> = typed.clone();
    let (addr, connected, entity_to_addr) = one_session(ENTITY, false);
    assert_eq!(connected.lock().unwrap()[&addr].access_level, 0);
    route_cell_message(
        CellToBaseMsg::Plugin(PluginMsg::new(GmCraftGrant {
            entity_id: ENTITY,
            player_id: PLAYER_ID,
            gm_entity_id: ENTITY,
            grant,
        })),
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
        &super::crafting_plugins(),
    )
    .await;
    (typed.filter_to(addr), capture)
}

#[tokio::test]
async fn craftkit_reaches_its_handler_and_a_non_gm_is_refused() {
    let (sent, capture) = dispatch_from_non_gm(GmCraftGrantKind::Kit {
        blueprint_id: 25,
        count: 1,
    })
    .await;
    assert_eq!(
        sent,
        vec![feedback_packet(
            "craftkit: refused, GameMaster access is required."
        )]
    );
    let e = capture
        .find_event(tracing::Level::WARN, "below GameMaster", "not_gm")
        .expect("the refusal is logged");
    assert!(e.has_field("event", "gm_craftkit"), "{e:#?}");
}

#[tokio::test]
async fn learnblueprint_reaches_its_handler_and_a_non_gm_is_refused() {
    let (sent, capture) =
        dispatch_from_non_gm(GmCraftGrantKind::LearnBlueprint { blueprint_id: 25 }).await;
    assert_eq!(
        sent,
        vec![feedback_packet(
            "learnblueprint: refused, GameMaster access is required."
        )]
    );
    let e = capture
        .find_event(tracing::Level::WARN, "below GameMaster", "not_gm")
        .expect("the refusal is logged");
    assert!(e.has_field("event", "gm_learnblueprint"), "{e:#?}");
}
