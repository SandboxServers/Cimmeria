//! SS-C3: `.mute` and `.unmute` through the console parser. The cell only
//! parses, bounds and forwards; the base resolves the name and holds the
//! table (`cimmeria-base-session` `mutes/`).

use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_wire::cell::messages::ChatCellToBase;
use tokio::sync::mpsc;

use super::setup;
use crate::cell::console::handle_console_command;
use crate::cell::messages::CellToBaseMsg;

fn ids(mgr: &mut crate::cell::space_manager::SpaceManager, gm: u32) {
    let e = mgr.get_entity_mut(gm).unwrap();
    e.account_id = Some(7);
    e.player_id = Some(70);
}

/// `.mute <name> <minutes> <reason...>` hands the base one `Mute` carrying
/// the GM's ids from the cell's session state, never from the typed line.
#[tokio::test]
async fn mute_forwards_to_the_base_with_the_gm_ids() {
    let (mut mgr, gm, _npc) = setup();
    ids(&mut mgr, gm);
    let (tx, mut rx) = mpsc::channel(16);

    handle_console_command(
        gm,
        ".mute Loudmouth 15 trade spam",
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let msgs: Vec<CellToBaseMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert!(matches!(
        &msgs[0],
        CellToBaseMsg::Chat(ChatCellToBase::Mute {
            entity_id,
            player_id: Some(70),
            account_id: Some(7),
            target_name,
            minutes: 15,
            reason,
        }) if *entity_id == gm && target_name == "Loudmouth" && reason == "trade spam"
    ));
}

/// `.unmute <name>` hands the base one `Unmute`.
#[tokio::test]
async fn unmute_forwards_to_the_base() {
    let (mut mgr, gm, _npc) = setup();
    ids(&mut mgr, gm);
    let (tx, mut rx) = mpsc::channel(16);

    handle_console_command(gm, ".unmute Loudmouth", &tx, &mut mgr, &ChainEngine::new()).await;

    let msgs: Vec<CellToBaseMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert_eq!(msgs.len(), 1, "{msgs:?}");
    assert!(matches!(
        &msgs[0],
        CellToBaseMsg::Chat(ChatCellToBase::Unmute {
            entity_id,
            account_id: Some(7),
            target_name,
            ..
        }) if *entity_id == gm && target_name == "Loudmouth"
    ));
}

/// Type 12: a bad `.mute` reaches the command's own refusal, not the
/// generic argc check: `chat.gm_mute_refused` with its `reason` and the
/// GM's ids, the GM's line, and nothing handed to the base.
#[tokio::test]
async fn bad_mute_logs_reason_and_forwards_nothing() {
    for (line, reason, gm_line) in [
        (
            ".mute",
            "usage",
            ".mute: Usage: .mute <name> <minutes> [reason]",
        ),
        (
            ".mute Loudmouth forever",
            "bad_duration",
            ".mute: minutes must be a whole number from 1 to 10080.",
        ),
    ] {
        let (mut mgr, gm, _npc) = setup();
        ids(&mut mgr, gm);
        let (tx, mut rx) = mpsc::channel(16);
        let capture = crate::test_support::LogCapture::install();

        handle_console_command(gm, line, &tx, &mut mgr, &ChainEngine::new()).await;

        let row = capture
            .find_event(tracing::Level::WARN, "GM .mute refused on the cell", reason)
            .unwrap_or_else(|| panic!("{line}: chat.gm_mute_refused reason={reason}"));
        assert!(row.has_field("event", "chat.gm_mute_refused"));
        assert!(row.has_field("account_id", "7"));
        assert!(row.has_field("player_id", "70"));
        let msgs: Vec<CellToBaseMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert!(
            !msgs
                .iter()
                .any(|m| matches!(m, CellToBaseMsg::Chat(ChatCellToBase::Mute { .. }))),
            "{line}: nothing forwarded"
        );
        let lines: Vec<String> = msgs.iter().filter_map(super::decode_feedback).collect();
        assert_eq!(lines, vec![gm_line.to_string()], "{line}");
    }
}

/// Type 12: a bare `.unmute` gets its own usage line and
/// `chat.gm_unmute_refused reason=usage`.
#[tokio::test]
async fn bare_unmute_logs_usage() {
    let (mut mgr, gm, _npc) = setup();
    ids(&mut mgr, gm);
    let (tx, mut rx) = mpsc::channel(16);
    let capture = crate::test_support::LogCapture::install();

    handle_console_command(gm, ".unmute", &tx, &mut mgr, &ChainEngine::new()).await;

    let row = capture
        .find_event(
            tracing::Level::WARN,
            "GM .unmute refused on the cell",
            "usage",
        )
        .expect("chat.gm_unmute_refused reason=usage");
    assert!(row.has_field("event", "chat.gm_unmute_refused"));
    let lines: Vec<String> = std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|m| super::decode_feedback(&m))
        .collect();
    assert_eq!(lines, vec![".unmute: Usage: .unmute <name>".to_string()]);
}
