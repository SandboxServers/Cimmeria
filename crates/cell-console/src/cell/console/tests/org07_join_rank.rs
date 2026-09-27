//! ORG-07: `.org_join <orgId> [player]` and `.org_rank <player> <rank>
//! [orgId]` are forwarded to the base as `OrgCellToBase::GmJoin` /
//! `GmRank` with the GM's own character, GM-gated like every `.` command;
//! a malformed argument never leaves the cell.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::super::chat::{handle_chat_message, CHAN_SAY};
use super::setup;
use crate::cell::messages::{CellToBaseMsg, OrgCellToBase};
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;
use tracing::Level;

fn fixture(gm_access: u32) -> (SpaceManager, u32) {
    let (mut mgr, gm, _npc) = setup();
    let e = mgr.get_entity_mut(gm).unwrap();
    e.player_id = Some(7201);
    e.account_id = Some(8201);
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

#[tokio::test]
async fn org_join_forwards_to_the_base() {
    let (mut mgr, gm) = fixture(2);
    assert_eq!(
        say(&mut mgr, gm, ".org_join 42").await,
        vec![OrgCellToBase::GmJoin {
            player_id: 7201,
            entity_id: gm,
            org_id: 42,
            target_name: None,
        }]
    );
    assert_eq!(
        say(&mut mgr, gm, ".org_join 42 Bo").await,
        vec![OrgCellToBase::GmJoin {
            player_id: 7201,
            entity_id: gm,
            org_id: 42,
            target_name: Some("Bo".into()),
        }]
    );
}

#[tokio::test]
async fn org_rank_forwards_to_the_base() {
    let (mut mgr, gm) = fixture(2);
    assert_eq!(
        say(&mut mgr, gm, ".org_rank Bo 6").await,
        vec![OrgCellToBase::GmRank {
            player_id: 7201,
            entity_id: gm,
            target_name: "Bo".into(),
            rank: 6,
            org_id: None,
        }]
    );
    assert_eq!(
        say(&mut mgr, gm, ".org_rank Bo 6 42").await,
        vec![OrgCellToBase::GmRank {
            player_id: 7201,
            entity_id: gm,
            target_name: "Bo".into(),
            rank: 6,
            org_id: Some(42),
        }]
    );
}

/// A malformed id or rank is refused on the cell with its row; nothing
/// reaches the base.
#[tokio::test]
async fn malformed_org_join_and_rank_stay_on_the_cell() {
    let (mut mgr, gm) = fixture(2);
    for (line, event, reason) in [
        (".org_join forty", "org.gm_join", "org_id_invalid"),
        (".org_rank Bo six", "org.gm_rank", "rank_invalid"),
        (".org_rank Bo 6 x", "org.gm_rank", "org_id_invalid"),
    ] {
        let capture = LogCapture::install();
        assert!(say(&mut mgr, gm, line).await.is_empty(), "{line}");
        let row = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", event))
            .unwrap_or_else(|| panic!("{line}: no {event} row"));
        assert_eq!((row.level, row.target.as_str()), (Level::INFO, "org"));
        assert!(row.has_field("reason", reason), "{line}: {:?}", row.fields);
    }
}

#[tokio::test]
async fn org_join_and_rank_from_a_non_gm_are_not_forwarded() {
    let (mut mgr, gm) = fixture(0);
    assert!(say(&mut mgr, gm, ".org_join 42").await.is_empty());
    assert!(say(&mut mgr, gm, ".org_rank Bo 6").await.is_empty());
}
