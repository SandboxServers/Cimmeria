//! BV-04 console: `.bankdump [player]` hands a read-only vault listing to
//! the base with the GM's own ids, and every refusal decided on the cell
//! logs `gm_action` with its reason.
//!
//! The base half (the read, the lines, `result=ok`) is tested in
//! `cimmeria-base-session` `base::bank_dump::tests`.
//!
//! Filter prefix: `bv04_console_`.

use cimmeria_common::EntityId;
use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::chat::{handle_chat_message, CHAN_SAY};
use crate::cell::messages::{BankCellToBase, BankSubject, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

const GM_ACCOUNT: u32 = 31;
const GM_PLAYER: i32 = 41;

/// Say `text` as `speaker`; return every message sent and the feedback
/// lines among them.
async fn say(
    mgr: &mut SpaceManager,
    speaker: u32,
    text: &str,
) -> (Vec<CellToBaseMsg>, Vec<String>) {
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
    let (mut msgs, mut lines) = (Vec::new(), Vec::new());
    while let Ok(msg) = rx.try_recv() {
        if let Some(t) = decode_feedback(&msg) {
            lines.push(t);
        }
        msgs.push(msg);
    }
    (msgs, lines)
}

fn dumps(msgs: &[CellToBaseMsg]) -> Vec<&BankCellToBase> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::Bank(b) => Some(b),
            _ => None,
        })
        .collect()
}

fn gm_actions(capture: &LogCaptureGuard) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", "gm_action"))
        .collect()
}

fn with_identity(mgr: &mut SpaceManager, gm: u32) {
    let e = mgr.get_entity_mut(gm).unwrap();
    e.account_id = Some(GM_ACCOUNT);
    e.player_id = Some(GM_PLAYER);
}

/// A bare `.bankdump` asks for the GM's own vault by player id; a named one
/// passes the name through untouched (the base matches it exactly, online
/// or not). Both carry the GM's own ids for the base's `gm_action`, and
/// both run in the INFO `bank.console_dump` span. Fails if the registry
/// row or the dispatch arm is removed ("Unknown command", no message).
#[tokio::test]
async fn bv04_console_bankdump_forwards_the_subject_with_the_gm_ids() {
    let (mut mgr, gm, _npc) = setup();
    with_identity(&mut mgr, gm);

    let capture = LogCapture::install();
    let (own, _) = say(&mut mgr, gm, ".bankdump").await;
    let (named, _) = say(&mut mgr, gm, ".bankdump Kasuf").await;

    assert_eq!(
        dumps(&own),
        vec![&BankCellToBase::GmDump {
            entity_id: gm,
            account_id: Some(GM_ACCOUNT),
            player_id: Some(GM_PLAYER),
            subject: BankSubject::Player(GM_PLAYER),
        }],
        "{own:?}"
    );
    assert_eq!(
        dumps(&named),
        vec![&BankCellToBase::GmDump {
            entity_id: gm,
            account_id: Some(GM_ACCOUNT),
            player_id: Some(GM_PLAYER),
            subject: BankSubject::Name("Kasuf".to_string()),
        }],
        "{named:?}"
    );
    assert_eq!(
        capture
            .all()
            .iter()
            .filter(|c| c.target == "span:bank.console_dump" && c.level == tracing::Level::INFO)
            .count(),
        2,
        "each `.bankdump` runs in the INFO bank.console_dump span"
    );
    assert!(
        gm_actions(&capture).is_empty(),
        "a forwarded dump is logged by the base, not twice"
    );
}

/// `.bankdump a b` is refused by the argument count before anything is
/// sent.
#[tokio::test]
async fn bv04_console_bankdump_takes_at_most_one_name() {
    let (mut mgr, gm, _npc) = setup();
    with_identity(&mut mgr, gm);
    let (msgs, lines) = say(&mut mgr, gm, ".bankdump a b").await;
    assert!(dumps(&msgs).is_empty(), "{msgs:?}");
    assert!(
        lines.iter().any(|l| l.contains("too many arguments")),
        "{lines:?}"
    );
}

/// A bare `.bankdump` from an entity with no character id: nothing goes to
/// the base, the GM is told to name a player, and `gm_action` logs
/// `result=refused reason=caller_not_player`.
#[tokio::test]
async fn bv04_console_bankdump_without_a_character_is_refused() {
    let (mut mgr, gm, _npc) = setup();
    mgr.get_entity_mut(gm).unwrap().account_id = Some(GM_ACCOUNT);

    let capture = LogCapture::install();
    let (msgs, lines) = say(&mut mgr, gm, ".bankdump").await;

    assert!(dumps(&msgs).is_empty(), "{msgs:?}");
    assert!(
        lines.iter().any(|l| l.contains("name a player")),
        "{lines:?}"
    );
    let events = gm_actions(&capture);
    assert_eq!(events.len(), 1, "{events:#?}");
    let e = &events[0];
    assert_eq!(e.level, tracing::Level::INFO);
    assert!(e.has_field("action", "bankdump"));
    assert!(e.has_field("result", "refused"));
    assert!(e.has_field("reason", "caller_not_player"));
    assert!(e.has_field("account_id", &GM_ACCOUNT.to_string()));
    assert!(e.has_field("entity_id", &gm.to_string()));
}

/// The base channel is gone: WARN `gm_action reason=base_channel_closed`
/// instead of a silent drop.
#[tokio::test]
async fn bv04_console_bankdump_logs_a_closed_base_channel() {
    let (mut mgr, gm, _npc) = setup();
    with_identity(&mut mgr, gm);
    let (tx, rx) = mpsc::channel::<CellToBaseMsg>(4);
    drop(rx);

    let capture = LogCapture::install();
    crate::cell::console::dispatch::exec(
        "bankdump",
        gm,
        &[],
        None,
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;

    let events = gm_actions(&capture);
    assert_eq!(events.len(), 1, "{events:#?}");
    let e = &events[0];
    assert_eq!(e.level, tracing::Level::WARN);
    assert!(e.has_field("reason", "base_channel_closed"));
    assert!(e.has_field("player_id", &GM_PLAYER.to_string()));
    assert!(e.has_field("target_player_id", &GM_PLAYER.to_string()));
    assert!(e.fields.contains_key("error"));
}

/// A non-GM's `.bankdump` is refused like every GM command ("is a GM
/// command", never broadcast) and also logs `gm_action result=refused
/// reason=not_gm` on the `bank` target. Fails if the chat hook is removed
/// (no `bank` row) or the generic refusal is (the line reaches the
/// witness).
#[tokio::test]
async fn bv04_console_bankdump_is_refused_for_a_non_gm() {
    let (mut mgr, player, _npc) = setup();
    with_identity(&mut mgr, player);
    mgr.create_entity(2, "Agnos", [11.0, 0.0, 11.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(2);
    {
        let p = mgr.get_entity_mut(player).unwrap();
        p.access_level = 0;
        p.witnesses.insert(EntityId(2));
    }

    let capture = LogCapture::install();
    let (msgs, lines) = say(&mut mgr, player, ".bankdump Kasuf").await;

    assert!(dumps(&msgs).is_empty(), "{msgs:?}");
    assert!(
        msgs.iter()
            .all(|m| !matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: 2, .. })),
        "nothing reaches the witness: {msgs:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("is a GM command")),
        "{lines:?}"
    );
    let events = gm_actions(&capture);
    assert_eq!(events.len(), 1, "{events:#?}");
    let e = &events[0];
    assert_eq!(e.level, tracing::Level::INFO);
    assert!(e.has_field("result", "refused"));
    assert!(e.has_field("reason", "not_gm"));
    assert!(e.has_field("account_id", &GM_ACCOUNT.to_string()));
    assert!(e.has_field("player_id", &GM_PLAYER.to_string()));
    assert!(e.has_field("entity_id", &player.to_string()));
}
