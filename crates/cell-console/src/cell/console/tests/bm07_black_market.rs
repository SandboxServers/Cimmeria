//! BM-07: the GM Black Market tools `.bm_seed`, `.bm_expire` and `.bm_list`
//! on the cell: the parsers, the message handed to the base, the refusals
//! (type 12) and the non-GM gate. The base half is tested in
//! `cimmeria-base-methods` (`methods/black_market/gm/tests.rs`).

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;
use tracing::Level;

use super::pt07_giveability::{say, world};
use super::setup;
use crate::cell::console::black_market::{
    parse_expire, parse_seed, DEFAULT_SEED_COUNT, EXPIRE_USAGE, SEED_USAGE,
};
use crate::cell::console::handle_console_command;
use crate::cell::messages::{BlackMarketCellToBase, BmGmActor, CellToBaseMsg};
use crate::test_support::LogCapture;

#[test]
fn seed_parse_defaults_to_the_uat_set_and_caps_at_60() {
    assert_eq!(parse_seed(&[]), Ok(DEFAULT_SEED_COUNT));
    assert_eq!(parse_seed(&["1"]), Ok(1));
    assert_eq!(parse_seed(&["60"]), Ok(60));
    for bad in ["0", "61", "-1", "x", "300"] {
        let r = parse_seed(&[bad]).unwrap_err();
        assert_eq!(
            (r.reason, r.line.as_str()),
            ("invalid_count", SEED_USAGE),
            "{bad}"
        );
    }
}

#[test]
fn expire_parse_needs_a_positive_id() {
    assert_eq!(parse_expire(&["12"]), Ok(12));
    assert_eq!(
        parse_expire(&["#12"]),
        Ok(12),
        "the id as .bm_list prints it"
    );
    for bad in [&[][..], &["x"], &["0"], &["-3"]] {
        let r = parse_expire(bad).unwrap_err();
        assert_eq!((r.reason, r.line.as_str()), ("no_auction_id", EXPIRE_USAGE));
    }
}

fn gm_world() -> (crate::cell::space_manager::SpaceManager, u32) {
    let (mut mgr, gm, _npc) = setup();
    let e = mgr.get_entity_mut(gm).unwrap();
    e.account_id = Some(7);
    e.player_id = Some(70);
    (mgr, gm)
}

async fn run(
    mgr: &mut crate::cell::space_manager::SpaceManager,
    gm: u32,
    line: &str,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, line, &tx, mgr, &ChainEngine::new()).await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

/// The Black Market GM messages among `msgs`, which must be all of them.
fn bm(msgs: Vec<CellToBaseMsg>) -> Vec<BlackMarketCellToBase> {
    msgs.into_iter()
        .map(|m| match m {
            CellToBaseMsg::BlackMarket(g) => g,
            other => panic!("expected only BlackMarket, got {other:?}"),
        })
        .collect()
}

/// Each command hands the base one message with the GM's ids from the
/// cell entity (never a client payload) and sends no line of its own: the
/// base answers.
#[tokio::test]
async fn each_command_forwards_one_message_with_the_gm_ids() {
    let (mut mgr, gm) = gm_world();
    let actor = BmGmActor {
        entity_id: gm,
        player_id: 70,
        account_id: Some(7),
    };
    for (line, want) in [
        (
            ".bm_seed",
            BlackMarketCellToBase::GmSeed {
                actor,
                count: DEFAULT_SEED_COUNT,
            },
        ),
        (
            ".bm_seed 25",
            BlackMarketCellToBase::GmSeed { actor, count: 25 },
        ),
        (
            ".bm_expire 42",
            BlackMarketCellToBase::GmExpire {
                actor,
                sequence_id: 42,
            },
        ),
        (".bm_list", BlackMarketCellToBase::GmList { actor }),
    ] {
        let msgs = run(&mut mgr, gm, line).await;
        assert!(
            !msgs.iter().any(|m| super::decode_feedback(m).is_some()),
            "{line}: no cell-side line {msgs:?}"
        );
        assert_eq!(bm(msgs), vec![want], "{line}");
    }
}

/// Type 12: a malformed command goes no further than the cell: a WARN
/// `bm.gm_rejected` with the reason and the GM's ids, the usage line, and
/// no message to the base.
#[tokio::test]
async fn refusals_log_the_reason_and_send_nothing_to_the_base() {
    let (mut mgr, gm) = gm_world();
    for (line, reason, usage) in [
        (".bm_seed 0", "invalid_count", SEED_USAGE),
        (".bm_seed lots", "invalid_count", SEED_USAGE),
        (".bm_expire", "no_auction_id", EXPIRE_USAGE),
        (".bm_expire -3", "no_auction_id", EXPIRE_USAGE),
    ] {
        let capture = LogCapture::install();
        let msgs = run(&mut mgr, gm, line).await;
        assert!(
            !msgs
                .iter()
                .any(|m| matches!(m, CellToBaseMsg::BlackMarket(_))),
            "{line}: {msgs:?}"
        );
        let lines: Vec<String> = msgs.iter().filter_map(super::decode_feedback).collect();
        assert_eq!(lines, vec![usage.to_string()], "{line}");
        let row = capture
            .find_event(Level::WARN, "GM Black Market command refused", reason)
            .unwrap_or_else(|| panic!("{line}: bm.gm_rejected reason={reason}"));
        assert!(row.has_field("event", "bm.gm_rejected"));
        assert!(row.has_field("account_id", "7") && row.has_field("player_id", "70"));
    }
}

/// A player (access level 0) typing any of the three gets the "GM command"
/// line and nothing reaches the base: no auction is listed or settled.
#[tokio::test]
async fn non_gm_black_market_commands_are_refused() {
    for line in [".bm_seed 60", ".bm_expire 1", ".bm_list"] {
        let (mut mgr, _npc) = world(0);
        let msgs = say(&mut mgr, line).await;
        assert!(
            !msgs
                .iter()
                .any(|m| matches!(m, CellToBaseMsg::BlackMarket(_))),
            "{line}: {msgs:?}"
        );
        let lines: Vec<String> = msgs.iter().filter_map(super::decode_feedback).collect();
        assert_eq!(lines.len(), 1, "{line}: {lines:?}");
        assert!(lines[0].contains("is a GM command"), "{line}: {lines:?}");
    }
}
