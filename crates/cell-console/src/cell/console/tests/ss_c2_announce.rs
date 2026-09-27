//! SS-C2: `.announce` end to end through the console parser.

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_wire::cell::chat::serialize_gm_broadcast;
use cimmeria_wire::cell::messages::ChatCellToBase;
use tokio::sync::mpsc;

use super::setup;
use crate::cell::console::handle_console_command;
use crate::cell::messages::CellToBaseMsg;

/// `.announce space <text>` sends the GM line to the GM's space (the GM is
/// the only player in the fixture) and nothing to the base; the typed
/// words are re-joined with single spaces.
#[tokio::test]
async fn announce_space_sends_the_gm_line_to_the_space() {
    let (mut mgr, gm, _npc) = setup();
    mgr.get_entity_mut(gm).unwrap().character_name = Some("Gm".to_string());
    let (tx, mut rx) = mpsc::channel(16);

    handle_console_command(
        gm,
        ".announce space Event  at the gate",
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let msgs: Vec<CellToBaseMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert_eq!(msgs.len(), 1, "one line, to the one player in the space");
    match &msgs[0] {
        CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } => {
            assert_eq!(*entity_id, gm);
            assert_eq!(
                *method_index,
                crate::mercury::method_idx::ON_PLAYER_COMMUNICATION
            );
            assert_eq!(*args, serialize_gm_broadcast("Gm", "Event at the gate"));
        }
        other => panic!("expected the GM line, got {other:?}"),
    }
}

/// `.announce <text>` with no scope word is global: one broadcast handed
/// to the base, tagged `console`.
#[tokio::test]
async fn announce_without_scope_is_global() {
    let (mut mgr, gm, _npc) = setup();
    mgr.get_entity_mut(gm).unwrap().character_name = Some("Gm".to_string());
    let (tx, mut rx) = mpsc::channel(16);

    handle_console_command(
        gm,
        ".announce Hello all",
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let msgs: Vec<CellToBaseMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert_eq!(msgs.len(), 1);
    match &msgs[0] {
        CellToBaseMsg::Chat(ChatCellToBase::GmBroadcast {
            entity_id,
            source,
            args,
            ..
        }) => {
            assert_eq!(*entity_id, gm);
            assert_eq!(*source, "console");
            assert_eq!(*args, serialize_gm_broadcast("Gm", "Hello all"));
        }
        other => panic!("expected a global GmBroadcast, got {other:?}"),
    }
}

/// `.help announce` shows the summary and the argument detail line, like
/// the other documented commands.
#[tokio::test]
async fn help_announce_shows_usage_and_argument_detail() {
    let (mut mgr, gm, _npc) = setup();
    let (tx, mut rx) = mpsc::channel(64);

    handle_console_command(gm, ".help announce", &tx, &mut mgr, &ChainEngine::new()).await;

    let lines: Vec<String> = std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|m| super::decode_feedback(&m))
        .collect();
    assert!(
        lines.iter().any(|l| l.starts_with(".announce: ")),
        "summary line: {lines:?}"
    );
    assert!(
        // Rendered bracketed because the spec's `min` is 0 (so a bare
        // `.announce` reaches its own usage line); the text says required.
        lines
            .iter()
            .any(|l| l.starts_with("    [text] (str): Required.")),
        "argument detail line: {lines:?}"
    );
}

/// Type 12: a bare `.announce` reaches the command's own refusal, not the
/// generic argc check: the usage line and `chat.gm_broadcast_rejected
/// reason=no_text` with the GM's ids. Fails if the spec's `min` goes back
/// to 1 (the argc check answers first and logs no chat event).
#[tokio::test]
async fn bare_announce_logs_no_text_and_shows_usage() {
    let (mut mgr, gm, _npc) = setup();
    {
        let e = mgr.get_entity_mut(gm).unwrap();
        e.account_id = Some(7);
        e.player_id = Some(70);
    }
    let (tx, mut rx) = mpsc::channel(16);
    let capture = crate::test_support::LogCapture::install();

    handle_console_command(gm, ".announce", &tx, &mut mgr, &ChainEngine::new()).await;

    let row = capture
        .find_event(tracing::Level::WARN, ".announce had no text", "no_text")
        .expect("a bare .announce must log chat.gm_broadcast_rejected reason=no_text");
    assert!(row.has_field("event", "chat.gm_broadcast_rejected"));
    assert!(row.has_field("account_id", "7"));
    assert!(row.has_field("player_id", "70"));
    let lines: Vec<String> = std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|m| super::decode_feedback(&m))
        .collect();
    assert_eq!(
        lines,
        vec![".announce: nothing to announce. Usage: .announce [space] <text>".to_string()],
        "only the command's own usage line"
    );
}
