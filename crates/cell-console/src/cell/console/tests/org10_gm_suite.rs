//! ORG-10: `.org_info [player]`, `.org_list` and
//! `.org_set_perms <orgId> <rank> <mask>` are registered, GM-gated and
//! forwarded to the base with the GM's own character; a malformed argument
//! never leaves the cell. Every console-side refusal of a GM organization
//! command writes one `org.gm_action` row (D-ORG13).

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::super::chat::{handle_chat_message, CHAN_SAY};
use super::super::dispatch::find_command;
use super::super::org::parse_mask;
use super::setup;
use crate::cell::messages::{CellToBaseMsg, OrgCellToBase};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};
use tracing::Level;

fn fixture(gm_access: u32) -> (SpaceManager, u32) {
    let (mut mgr, gm, _npc) = setup();
    let e = mgr.get_entity_mut(gm).unwrap();
    e.player_id = Some(7301);
    e.account_id = Some(8301);
    e.character_name = Some("Gm".to_owned());
    e.access_level = gm_access;
    (mgr, gm)
}

async fn say(mgr: &mut SpaceManager, speaker: u32, text: &str) -> Vec<OrgCellToBase> {
    let (tx, mut rx) = mpsc::channel(64);
    handle_chat_message(
        speaker,
        "Gm",
        0,
        CHAN_SAY,
        text,
        &tx,
        mgr,
        &ChainEngine::new(),
    )
    .await;
    drop(tx);
    let mut out = Vec::new();
    while let Some(msg) = rx.recv().await {
        if let CellToBaseMsg::Org(o) = msg {
            out.push(o);
        }
    }
    out
}

/// The `org.gm_action` rows in `capture`.
fn gm_actions(capture: &LogCaptureGuard) -> Vec<Captured> {
    capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "org.gm_action"))
        .collect()
}

/// Each new command is registered with the argument counts its handler
/// reads; a missing row would answer "Unknown command".
#[test]
fn org10_commands_are_registered() {
    for (name, min, max) in [
        ("org_info", 0, 1),
        ("org_list", 0, 0),
        ("org_set_perms", 3, 3),
    ] {
        let spec = find_command(name).unwrap_or_else(|| panic!(".{name} is not registered"));
        assert_eq!((spec.min, spec.max), (min, max), ".{name}");
    }
}

#[tokio::test]
async fn org_info_forwards_to_the_base() {
    let (mut mgr, gm) = fixture(2);
    assert_eq!(
        say(&mut mgr, gm, ".org_info").await,
        vec![OrgCellToBase::GmInfo {
            player_id: 7301,
            entity_id: gm,
            target_name: None,
        }]
    );
    assert_eq!(
        say(&mut mgr, gm, ".org_info Bo").await,
        vec![OrgCellToBase::GmInfo {
            player_id: 7301,
            entity_id: gm,
            target_name: Some("Bo".into()),
        }]
    );
}

#[tokio::test]
async fn org_list_forwards_to_the_base() {
    let (mut mgr, gm) = fixture(2);
    assert_eq!(
        say(&mut mgr, gm, ".org_list").await,
        vec![OrgCellToBase::GmList {
            player_id: 7301,
            entity_id: gm,
        }]
    );
}

/// Decimal and `0x` hex masks both reach the base unchanged: the clamp is
/// the base's, under the lock, where the organization's type is known.
#[tokio::test]
async fn org_set_perms_forwards_to_the_base() {
    let (mut mgr, gm) = fixture(2);
    for (line, mask) in [
        (".org_set_perms 42 6 1024", 1024),
        (".org_set_perms 42 6 0x400", 0x400),
        (".org_set_perms 42 6 0xFFFFFFFF", u32::MAX),
    ] {
        assert_eq!(
            say(&mut mgr, gm, line).await,
            vec![OrgCellToBase::GmSetPerms {
                player_id: 7301,
                entity_id: gm,
                org_id: 42,
                rank: 6,
                mask,
            }],
            "{line}"
        );
    }
}

#[test]
fn masks_parse_as_decimal_or_hex() {
    assert_eq!(parse_mask("0"), Some(0));
    assert_eq!(parse_mask("67108863"), Some(0x3FF_FFFF));
    assert_eq!(parse_mask("0x3ff_ffff"), None);
    assert_eq!(parse_mask("0X3FFFFFF"), Some(0x3FF_FFFF));
    assert_eq!(parse_mask("-1"), None);
    assert_eq!(parse_mask("mask"), None);
}

/// A malformed argument is refused on the cell with one `org.gm_action`
/// row and its reason; nothing reaches the base.
#[tokio::test]
async fn malformed_org_set_perms_stays_on_the_cell() {
    let (mut mgr, gm) = fixture(2);
    for (line, reason) in [
        (".org_set_perms forty 6 1", "org_id_invalid"),
        (".org_set_perms 42 six 1", "rank_invalid"),
        (".org_set_perms 42 6 lots", "mask_invalid"),
    ] {
        let capture = LogCapture::install();
        assert!(say(&mut mgr, gm, line).await.is_empty(), "{line}");
        let rows = gm_actions(&capture);
        assert_eq!(rows.len(), 1, "{line}: {rows:#?}");
        let row = &rows[0];
        assert_eq!((row.level, row.target.as_str()), (Level::INFO, "org"));
        for (k, v) in [
            ("action", "gm_org_set_perms"),
            ("outcome", "rejected"),
            ("reason", reason),
            ("player_id", "7301"),
            ("account_id", "8301"),
        ] {
            assert!(row.has_field(k, v), "{line}: {k}={v} {:?}", row.fields);
        }
    }
}

/// The ORG-06 / ORG-07 commands' console-side refusals now write their
/// `org.gm_action` audit row beside their own row (the D-ORG13 gap ORG-10
/// closed): before, a GM typo left no `org.gm_action` at all.
#[tokio::test]
async fn console_refusals_of_every_gm_org_command_write_one_gm_action_row() {
    let (mut mgr, gm) = fixture(2);
    for (line, event, action, reason) in [
        (
            ".org_disband forty",
            "org.disband",
            "disband",
            "org_id_invalid",
        ),
        (
            ".org_join forty",
            "org.gm_join",
            "gm_org_join",
            "org_id_invalid",
        ),
        (
            ".org_rank Bo six",
            "org.gm_rank",
            "gm_org_rank",
            "rank_invalid",
        ),
    ] {
        let capture = LogCapture::install();
        assert!(say(&mut mgr, gm, line).await.is_empty(), "{line}");
        let rows = gm_actions(&capture);
        assert_eq!(rows.len(), 1, "{line}: {rows:#?}");
        assert!(rows[0].has_field("action", action), "{line}");
        assert!(rows[0].has_field("reason", reason), "{line}");
        assert!(rows[0].has_field("player_id", "7301"), "{line}");
        assert_eq!(
            capture
                .all()
                .iter()
                .filter(|c| c.has_field("event", event))
                .count(),
            1,
            "{line}: its own {event} row is still written once"
        );
    }
}

#[tokio::test]
async fn org10_commands_from_a_non_gm_are_not_forwarded() {
    let (mut mgr, gm) = fixture(0);
    for line in [".org_info", ".org_list", ".org_set_perms 42 6 1"] {
        assert!(say(&mut mgr, gm, line).await.is_empty(), "{line}");
    }
}
