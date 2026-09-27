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
        lines.iter().any(|l| l.starts_with("    text (str): ")),
        "argument detail line: {lines:?}"
    );
}
