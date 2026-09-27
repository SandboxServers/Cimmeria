//! BV-02 console: `.bank` opens the GM's personal vault anywhere, and a
//! non-GM's `.bank` is refused with a visible line instead of being said
//! aloud.
//!
//! Filter prefix: `bv02_console_`.

use cimmeria_common::EntityId;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::chat::{handle_chat_message, CHAN_SAY};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

const ON_VAULT_OPEN: u16 = cimmeria_wire::cell::client_methods::player::ON_VAULT_OPEN;

/// Say `text` as `speaker` and collect every `(recipient, method)` plus the
/// feedback lines.
async fn say(mgr: &mut SpaceManager, speaker: u32, text: &str) -> (Vec<(u32, u16)>, Vec<String>) {
    let (tx, mut rx) = mpsc::channel::<CellToBaseMsg>(64);
    handle_chat_message(
        speaker,
        "Tester",
        0,
        CHAN_SAY,
        text,
        &tx,
        mgr,
        &ChainEngine::new(),
    )
    .await;
    let (mut calls, mut lines) = (Vec::new(), Vec::new());
    while let Ok(msg) = rx.try_recv() {
        if let Some(t) = decode_feedback(&msg) {
            lines.push(t);
        }
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            ..
        } = msg
        {
            calls.push((entity_id, method_index));
        }
    }
    (calls, lines)
}

/// A GM's `.bank` sends `onVaultOpen` for the GM, opens a Banker-less
/// session, and confirms on the feedback channel. Fails if the registry row
/// or the dispatch arm is removed (the console answers "Unknown command").
#[tokio::test]
async fn bv02_console_bank_opens_the_gm_vault_with_a_bankerless_session() {
    let (mut mgr, gm, _npc) = setup();

    let capture = crate::test_support::LogCapture::install();
    let (calls, lines) = say(&mut mgr, gm, ".bank").await;
    assert!(
        capture
            .all()
            .iter()
            .any(|c| c.target == "span:bank.console_open" && c.level == tracing::Level::INFO),
        "`.bank` runs in the INFO bank.console_open span"
    );

    assert_eq!(
        calls.iter().filter(|c| **c == (gm, ON_VAULT_OPEN)).count(),
        1,
        "exactly one onVaultOpen to the GM: {calls:?}"
    );
    let session = mgr
        .get_entity(gm)
        .unwrap()
        .vault_session
        .clone()
        .expect("`.bank` opens a session");
    assert_eq!(session.banker_id, None);
    assert!(
        lines.iter().any(|l| l.contains("personal vault opened")),
        "confirmation line: {lines:?}"
    );
}

/// A non-GM's `.bank` opens nothing, reaches no witness, and tells the
/// player why. Fails if the chat interceptor's `.bank` refusal is removed
/// (the line is then broadcast as chat to the witness, with no feedback).
#[tokio::test]
async fn bv02_console_bank_is_refused_for_a_non_gm_with_feedback() {
    let (mut mgr, player, _npc) = setup();
    mgr.create_entity(2, "Agnos", [11.0, 0.0, 11.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(2);
    {
        let p = mgr.get_entity_mut(player).unwrap();
        p.access_level = 0;
        p.witnesses.insert(EntityId(2));
    }

    let capture = crate::test_support::LogCapture::install();
    let (calls, lines) = say(&mut mgr, player, ".bank").await;

    // D-BV19: WARN `vault_open_rejected reason=not_gm`, in the INFO
    // `bank.console_open` span.
    let rejected: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", "vault_open_rejected"))
        .collect();
    assert_eq!(rejected.len(), 1, "{rejected:#?}");
    assert_eq!(rejected[0].level, tracing::Level::WARN);
    assert!(rejected[0].has_field("reason", "not_gm"));
    assert!(rejected[0].has_field("entity_id", &player.to_string()));
    assert!(capture
        .all()
        .iter()
        .any(|c| c.target == "span:bank.console_open" && c.level == tracing::Level::INFO));

    assert!(
        calls
            .iter()
            .all(|(to, m)| *to == player && *m != ON_VAULT_OPEN),
        "nothing to the witness, no onVaultOpen: {calls:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("needs GM access")),
        "refusal line: {lines:?}"
    );
    assert!(mgr.get_entity(player).unwrap().vault_session.is_none());

    // Other `.`-text from a non-GM is still ordinary chat.
    let (calls, _) = say(&mut mgr, player, ".banker hello").await;
    assert!(
        calls.iter().any(|(to, _)| *to == 2),
        ".banker is chat: {calls:?}"
    );
}
