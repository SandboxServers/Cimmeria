//! BV-05 console: `.bankexpand` hands a purchase to the base through the
//! Banker dialog's path (`trigger=gm_console`), and a non-GM's
//! `.bankexpand` is refused with a visible line instead of being said
//! aloud. The purchase itself is pinned in `cimmeria-base-session`
//! `bank_expand::gm_tests`.
//!
//! Filter prefix: `bv05_console_`.

use cimmeria_common::EntityId;
use cimmeria_content_engine::chain::ChainEngine;
use cimmeria_wire::cell::messages::ExpandTrigger;
use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::chat::{handle_chat_message, CHAN_SAY};
use crate::cell::messages::{BankCellToBase, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;

/// Say `text` as `speaker`; the bank messages, the recipients of every
/// entity method, and the feedback lines.
async fn say(
    mgr: &mut SpaceManager,
    speaker: u32,
    text: &str,
) -> (Vec<BankCellToBase>, Vec<u32>, Vec<String>) {
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
    let (mut bank, mut to, mut lines) = (Vec::new(), Vec::new(), Vec::new());
    while let Ok(msg) = rx.try_recv() {
        if let Some(t) = decode_feedback(&msg) {
            lines.push(t);
        }
        match msg {
            CellToBaseMsg::Bank(b) => bank.push(b),
            CellToBaseMsg::EntityMethodCall { entity_id, .. } => to.push(entity_id),
            _ => {}
        }
    }
    (bank, to, lines)
}

/// A GM with a `.bank` session: `.bankexpand` sends one `Expand` with no
/// offer, an open Banker-less verdict and `trigger=gm_console`. Fails if
/// the registry row or the dispatch arm is removed (the console answers
/// "Unknown command" and sends nothing).
#[tokio::test]
async fn bv05_console_bankexpand_sends_a_gm_purchase() {
    let (mut mgr, gm, _npc) = setup();
    mgr.get_entity_mut(gm).unwrap().player_id = Some(77);
    say(&mut mgr, gm, ".bank").await;

    let (bank, _, _) = say(&mut mgr, gm, ".bankexpand").await;

    let [BankCellToBase::Expand {
        player_id,
        offer,
        vault,
        trigger,
        ..
    }] = bank.as_slice()
    else {
        panic!("one Expand: {bank:?}");
    };
    assert_eq!(*player_id, 77);
    assert_eq!((*offer, *trigger), (None, ExpandTrigger::GmConsole));
    assert!(
        vault.opens_personal_vault() && vault.gm_override(),
        "{vault:?}"
    );
}

/// A non-GM's `.bankexpand` reaches neither the base nor a witness, and
/// tells the player why: WARN `expand_rejected reason=not_gm`. Fails if
/// the chat interceptor's refusal is removed (the generic GM-command line
/// replaces the bank one, with no `bank` row).
#[tokio::test]
async fn bv05_console_bankexpand_is_refused_for_a_non_gm() {
    let (mut mgr, player, _npc) = setup();
    mgr.create_entity(2, "Agnos", [11.0, 0.0, 11.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(2);
    {
        let p = mgr.get_entity_mut(player).unwrap();
        p.access_level = 0;
        p.player_id = Some(78);
        p.witnesses.insert(EntityId(2));
    }
    let capture = crate::test_support::LogCapture::install();

    let (bank, to, lines) = say(&mut mgr, player, ".bankexpand").await;

    assert!(bank.is_empty(), "nothing reaches the base: {bank:?}");
    assert!(
        to.iter().all(|e| *e == player),
        "nothing to the witness: {to:?}"
    );
    assert_eq!(
        lines,
        vec![".bankexpand needs GM access. Nothing was charged.".to_string()]
    );
    let rejected: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", "expand_rejected"))
        .collect();
    assert_eq!(rejected.len(), 1, "{rejected:#?}");
    assert_eq!(rejected[0].level, tracing::Level::WARN);
    assert!(rejected[0].has_field("reason", "not_gm"));
    assert!(rejected[0].has_field("trigger", "gm_console"));
    assert!(rejected[0].has_field("player_id", "78"));
}
