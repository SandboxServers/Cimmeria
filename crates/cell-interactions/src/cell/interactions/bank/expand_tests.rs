//! The cell half of the vault expansion (BV-05): every personal open asks
//! for a quote, the base's offer is recorded on the session and shown as
//! the Expand dialog, and the answer carries the one-shot offer and a
//! **fresh** vault verdict to the base. The purchase itself is pinned in
//! `cimmeria-base-session` `bank_expand/tests.rs`.

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_entity::cell_entity::{ExpansionOffer, VaultScope};
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};
use cimmeria_wire::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use cimmeria_wire::cell::client_methods::player::ON_DIALOG_DISPLAY;
use cimmeria_wire::cell::messages::BankCellToBase;
use cimmeria_wire::cell::vault::{VaultAccess, VAULT_EXPAND_DIALOG_ID};

use super::tests::{spawn_banker, two_space_manager, PLAYER};
use super::*;
use crate::cell::interactions::handle_interact;
use crate::cell::messages::CellToBaseMsg;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};

/// The player's `sgw_player.player_id` in `two_space_manager`.
const PLAYER_ID: i32 = 12;

fn bank_msgs(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<BankCellToBase> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::Bank(b) = msg {
            out.push(b);
        }
    }
    out
}

/// Every `(method_index, args)` sent to the player, in order.
fn methods(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(u16, Vec<u8>)> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: PLAYER,
            method_index,
            args,
        } = msg
        {
            out.push((method_index, args));
        }
    }
    out
}

fn dialog_display_args(speaker: u32) -> Vec<u8> {
    let mut args = Vec::new();
    args.extend_from_slice(&(speaker as i32).to_le_bytes());
    args.extend_from_slice(&VAULT_EXPAND_DIALOG_ID.to_le_bytes());
    args.extend_from_slice(&0i32.to_le_bytes());
    args.push(1);
    args.extend_from_slice(&0i32.to_le_bytes());
    args
}

fn offer(mgr: &SpaceManager) -> Option<ExpansionOffer> {
    mgr.get_entity(PLAYER)
        .and_then(|p| p.vault_session.as_ref())
        .and_then(|s| s.expansion_offer)
}

const OFFER_40: ExpansionOffer = ExpansionOffer {
    from_slots: 40,
    price: 100,
};

fn rows(capture: &LogCaptureGuard, name: &str) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.target == "bank" && c.has_field("event", name))
        .collect()
}

fn one(capture: &LogCaptureGuard, name: &str, level: Level, want: &[(&str, &str)]) -> Captured {
    let found = rows(capture, name);
    assert_eq!(found.len(), 1, "exactly one {name}: {:#?}", capture.all());
    let row = found.into_iter().next().unwrap();
    assert_eq!(row.level, level, "{name} level");
    for (k, v) in [("account_id", "6"), ("player_id", "12"), ("entity_id", "1")]
        .iter()
        .chain(want)
    {
        assert!(row.has_field(k, v), "{name}: {k}={v} missing: {row:#?}");
    }
    row
}

/// A Banker click asks for a quote spoken by the Banker; `.bank` asks for
/// one spoken by the GM's own entity. Fails if the quote request is removed
/// from the open path (no Expand dialog is ever offered).
#[tokio::test]
async fn every_personal_open_asks_the_base_for_a_quote() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let (tx, mut rx) = mpsc::channel(16);

    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    assert_eq!(
        bank_msgs(&mut rx),
        vec![BankCellToBase::ExpansionQuote {
            entity_id: PLAYER,
            account_id: Some(6),
            player_id: PLAYER_ID,
            speaker_id: banker,
        }]
    );

    assert!(open_vault_gm(PLAYER, &tx, &mut mgr).await);
    assert_eq!(
        bank_msgs(&mut rx),
        vec![BankCellToBase::ExpansionQuote {
            entity_id: PLAYER,
            account_id: Some(6),
            player_id: PLAYER_ID,
            speaker_id: PLAYER,
        }]
    );
}

/// An org Banker refusal opens no vault, so it asks for no quote.
#[tokio::test]
async fn a_refused_open_asks_for_no_quote() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Team);
    let (tx, mut rx) = mpsc::channel(16);

    handle_interact(PLAYER, banker, &tx, &mut mgr).await;

    assert!(bank_msgs(&mut rx).is_empty());
}

/// While the Expand dialog is quarantined (#943), the base's offer is
/// recorded on the session but nothing is sent: no `onDialogDisplay` for an
/// id the client has no entry for, and no price line pointing at a button
/// that is not there. DEBUG `expand_offer_suppressed reason=dialog_quarantined`,
/// once. Fails if the suppression is removed (the dialog would go out).
#[tokio::test]
async fn a_quarantined_dialog_is_recorded_but_not_shown() {
    // Pins the quarantined behaviour: serving the dialog stops this
    // compiling, so the test is updated with the flag.
    const {
        assert!(!cimmeria_wire::cell::vault::VAULT_EXPAND_DIALOG_SERVED);
    }
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let (tx, mut rx) = mpsc::channel(16);
    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    let _ = methods(&mut rx);
    let capture = LogCapture::install();

    offer_vault_expansion(PLAYER, PLAYER_ID, banker, 40, 100, &tx, &mut mgr).await;

    assert_eq!(offer(&mgr), Some(OFFER_40));
    assert!(
        methods(&mut rx).is_empty(),
        "nothing is sent while quarantined"
    );
    assert!(!mgr
        .get_entity(PLAYER)
        .unwrap()
        .offered_dialogs()
        .contains(&VAULT_EXPAND_DIALOG_ID));
    one(
        &capture,
        "expand_offer_suppressed",
        Level::DEBUG,
        &[
            ("reason", "dialog_quarantined"),
            ("bank_slots", "40"),
            ("price", "100"),
        ],
    );
    assert!(rows(&capture, "expand_offered").is_empty());
}

/// Once served, showing an offer sends the Expand dialog by the Banker
/// (recorded as offered, so its answer passes #479), then a chat line with
/// the price; DEBUG `expand_offered`.
#[tokio::test]
async fn showing_an_offer_sends_the_expand_dialog_and_the_price() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let (tx, mut rx) = mpsc::channel(16);
    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    let _ = methods(&mut rx);
    let capture = LogCapture::install();

    show_expand_offer(PLAYER, banker, OFFER_40, &tx, &mut mgr).await;

    let line = "Your vault has 40 slots. 10 more cost 100 naquadah: press Expand vault in the \
                Banker's dialog to buy them.";
    assert_eq!(
        methods(&mut rx),
        vec![
            (ON_DIALOG_DISPLAY, dialog_display_args(banker)),
            (
                ON_PLAYER_COMMUNICATION,
                serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, line)
            ),
        ]
    );
    assert!(mgr
        .get_entity(PLAYER)
        .unwrap()
        .offered_dialogs()
        .contains(&VAULT_EXPAND_DIALOG_ID));
    one(
        &capture,
        "expand_offered",
        Level::DEBUG,
        &[
            ("banker_id", &banker.to_string()),
            ("gm_override", "false"),
            ("bank_slots", "40"),
            ("price", "100"),
        ],
    );
}

/// GM `.bankexpand` in a GM `.bank` session: `Expand` with no offer (the
/// base quotes the step), an open Banker-less verdict, and
/// `trigger=gm_console`. With no session the verdict goes closed and the
/// base refuses it. Fails if the console path sends the session's offer or
/// a stale verdict.
#[tokio::test]
async fn gm_bankexpand_sends_a_fresh_verdict_and_no_offer() {
    use cimmeria_wire::cell::messages::ExpandTrigger;
    let mut mgr = two_space_manager();
    let (tx, mut rx) = mpsc::channel(16);

    gm_expand_vault(PLAYER, &tx, &mut mgr).await;
    let msgs = bank_msgs(&mut rx);
    let [BankCellToBase::Expand {
        offer,
        vault,
        trigger,
        ..
    }] = msgs.as_slice()
    else {
        panic!("one Expand: {msgs:?}");
    };
    assert_eq!((*offer, *trigger), (None, ExpandTrigger::GmConsole));
    assert_eq!(vault.personal_vault_refusal(), Some("no_vault_session"));

    assert!(open_vault_gm(PLAYER, &tx, &mut mgr).await);
    let _ = bank_msgs(&mut rx);
    let capture = LogCapture::install();
    gm_expand_vault(PLAYER, &tx, &mut mgr).await;
    let msgs = bank_msgs(&mut rx);
    let [BankCellToBase::Expand { offer, vault, .. }] = msgs.as_slice() else {
        panic!("one Expand: {msgs:?}");
    };
    assert_eq!(*offer, None);
    assert!(
        vault.opens_personal_vault() && vault.gm_override(),
        "{vault:?}"
    );
    assert!(capture
        .all()
        .iter()
        .any(|c| c.target == "span:bank.console_expand" && c.level == Level::INFO));
}

/// A non-GM's `.bankexpand`: WARN `expand_rejected reason=not_gm
/// trigger=gm_console`, one line, and nothing reaches the base.
#[tokio::test]
async fn a_non_gm_bankexpand_is_refused() {
    let mgr = two_space_manager();
    let (tx, mut rx) = mpsc::channel(16);
    let capture = LogCapture::install();

    refuse_non_gm_expand(PLAYER, &tx, &mgr).await;

    one(
        &capture,
        "expand_rejected",
        Level::WARN,
        &[("reason", "not_gm"), ("trigger", "gm_console")],
    );
    assert_eq!(
        methods(&mut rx),
        vec![(
            ON_PLAYER_COMMUNICATION,
            serialize_on_player_communication(
                "SYSTEM",
                0,
                CHAN_FEEDBACK,
                ".bankexpand needs GM access. Nothing was charged."
            )
        )]
    );
}

/// An offer that arrives after the session changed shows nothing: no
/// session, another speaker (a re-pin to another Banker), or an entity
/// that is now another character. DEBUG `expand_offer_dropped` names why.
#[tokio::test]
async fn a_stale_offer_shows_no_dialog() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let (tx, mut rx) = mpsc::channel(16);

    for (reason, player_id, speaker, open) in [
        ("no_vault_session", PLAYER_ID, banker, false),
        ("speaker_changed", PLAYER_ID, banker + 1, true),
        ("entity_is_another_player", PLAYER_ID + 1, banker, true),
    ] {
        mgr.get_entity_mut(PLAYER).unwrap().vault_session = None;
        if open {
            handle_interact(PLAYER, banker, &tx, &mut mgr).await;
        }
        let _ = methods(&mut rx);
        let capture = LogCapture::install();

        offer_vault_expansion(PLAYER, player_id, speaker, 40, 100, &tx, &mut mgr).await;

        assert!(methods(&mut rx).is_empty(), "{reason}: nothing shown");
        assert_eq!(offer(&mgr), None, "{reason}: nothing recorded");
        let rows = rows(&capture, "expand_offer_dropped");
        assert_eq!(rows.len(), 1, "{reason}: {rows:#?}");
        assert!(rows[0].has_field("reason", reason), "{:#?}", rows[0]);
        assert_eq!(rows[0].level, Level::DEBUG);
    }
}

/// The answer next to the Banker: `Expand` with the offered size and an
/// open personal verdict, and the offer is used up, so a second answer
/// carries none (the base refuses it as `no_offer`). Fails if the take is
/// removed (the second answer would carry 40 again).
#[tokio::test]
async fn the_answer_carries_the_offer_once_with_an_open_verdict() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let (tx, mut rx) = mpsc::channel(16);
    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    offer_vault_expansion(PLAYER, PLAYER_ID, banker, 40, 100, &tx, &mut mgr).await;
    let _ = bank_msgs(&mut rx);
    let capture = LogCapture::install();

    answer_vault_expansion(PLAYER, 8, &tx, &mut mgr).await;
    answer_vault_expansion(PLAYER, 8, &tx, &mut mgr).await;

    let open = VaultAccess::Open {
        org_id: None,
        scope: VaultScope::Personal,
        banker_id: Some(banker),
        distance: Some(2.0),
    };
    let expand = |offer| BankCellToBase::Expand {
        entity_id: PLAYER,
        account_id: Some(6),
        player_id: PLAYER_ID,
        offer,
        vault: open,
        trigger: cimmeria_wire::cell::messages::ExpandTrigger::Dialog,
    };
    assert_eq!(
        bank_msgs(&mut rx),
        vec![expand(Some(OFFER_40)), expand(None)]
    );
    assert!(capture
        .all()
        .iter()
        .any(|c| c.target == "span:bank.expand" && c.level == Level::INFO));
}

/// The dialog is not an authority check: walking away between the offer
/// and the press sends a **closed** verdict with the distance, taken at the
/// press. Fails if the verdict is taken when the offer is recorded instead.
#[tokio::test]
async fn walking_away_before_pressing_sends_a_fresh_closed_verdict() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let (tx, mut rx) = mpsc::channel(16);
    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    offer_vault_expansion(PLAYER, PLAYER_ID, banker, 40, 100, &tx, &mut mgr).await;
    let _ = bank_msgs(&mut rx);
    mgr.update_entity_position(PLAYER, [12.0, 0.0, 0.0], [0; 3], [0.0; 3]);

    answer_vault_expansion(PLAYER, 8, &tx, &mut mgr).await;

    let msgs = bank_msgs(&mut rx);
    let [BankCellToBase::Expand { vault, offer, .. }] = msgs.as_slice() else {
        panic!("one Expand: {msgs:?}");
    };
    assert_eq!(*offer, Some(OFFER_40));
    assert_eq!(vault.personal_vault_refusal(), Some("banker_out_of_range"));
    assert_eq!(vault.distance(), Some(10.0));
}

/// Only the authored button buys. A close (`-1`) and any other button id
/// send nothing to the base, keep the offer, and log DEBUG
/// `expand_dismissed` with `reason` `closed` or `unexpected_button`. Fails
/// if the button check is removed (the answer would buy).
#[tokio::test]
async fn only_the_expand_button_buys() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let (tx, mut rx) = mpsc::channel(16);
    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    offer_vault_expansion(PLAYER, PLAYER_ID, banker, 40, 100, &tx, &mut mgr).await;
    let _ = bank_msgs(&mut rx);

    for (button_id, reason) in [
        (-1, "closed"),
        (0, "unexpected_button"),
        (9, "unexpected_button"),
    ] {
        let capture = LogCapture::install();
        answer_vault_expansion(PLAYER, button_id, &tx, &mut mgr).await;

        assert!(bank_msgs(&mut rx).is_empty(), "button {button_id}");
        assert_eq!(offer(&mgr), Some(OFFER_40), "button {button_id}");
        one(
            &capture,
            "expand_dismissed",
            Level::DEBUG,
            &[("reason", reason), ("button_id", &button_id.to_string())],
        );
    }
}

/// A closed base channel: WARN `expand_rejected reason=base_channel_closed`
/// with the offered size, because nobody else can log it.
#[tokio::test]
async fn a_closed_base_channel_logs_the_lost_purchase() {
    let mut mgr = two_space_manager();
    let banker = spawn_banker(&mut mgr, "Agnos", [2.0, 0.0, 0.0], VaultScope::Personal);
    let (tx, rx) = mpsc::channel(16);
    handle_interact(PLAYER, banker, &tx, &mut mgr).await;
    offer_vault_expansion(PLAYER, PLAYER_ID, banker, 40, 100, &tx, &mut mgr).await;
    drop(rx);
    let capture = LogCapture::install();

    answer_vault_expansion(PLAYER, 8, &tx, &mut mgr).await;

    one(
        &capture,
        "expand_rejected",
        Level::WARN,
        &[
            ("reason", "base_channel_closed"),
            ("bank_slots", "40"),
            ("price", "100"),
        ],
    );
}

/// An entity with no character id: WARN `expand_rejected
/// reason=player_missing` and a line; nothing reaches the base.
#[tokio::test]
async fn an_entity_without_a_character_is_refused_at_the_cell() {
    let mut mgr = two_space_manager();
    mgr.get_entity_mut(PLAYER).unwrap().player_id = None;
    let (tx, mut rx) = mpsc::channel(16);
    let capture = LogCapture::install();

    answer_vault_expansion(PLAYER, 8, &tx, &mut mgr).await;

    let rows = rows(&capture, "expand_rejected");
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert!(rows[0].has_field("reason", "player_missing"));
    assert!(rows[0].has_field("account_id", "6"));
    assert!(!rows[0].fields.contains_key("player_id"));
    assert_eq!(
        methods(&mut rx),
        vec![(
            ON_PLAYER_COMMUNICATION,
            serialize_on_player_communication(
                "SYSTEM",
                0,
                CHAN_FEEDBACK,
                "Your vault could not be expanded. Nothing was charged."
            )
        )]
    );
}

/// A quote from an entity with no character is skipped (DEBUG
/// `expand_quote_skipped`), and a closed base channel logs WARN
/// `expand_quote_send_failed`.
#[tokio::test]
async fn quote_request_negative_paths_log() {
    let mut mgr = two_space_manager();
    let (tx, rx) = mpsc::channel(16);
    drop(rx);
    let capture = LogCapture::install();
    assert!(open_vault_gm(PLAYER, &tx, &mut mgr).await);
    one(
        &capture,
        "expand_quote_send_failed",
        Level::WARN,
        &[("reason", "base_channel_closed")],
    );
    drop(capture);

    mgr.get_entity_mut(PLAYER).unwrap().player_id = None;
    let (tx, mut rx) = mpsc::channel(16);
    let capture = LogCapture::install();
    assert!(open_vault_gm(PLAYER, &tx, &mut mgr).await);
    assert!(bank_msgs(&mut rx).is_empty());
    let rows = rows(&capture, "expand_quote_skipped");
    assert_eq!(rows.len(), 1, "{rows:#?}");
    assert!(rows[0].has_field("reason", "no_player_id"));
}
