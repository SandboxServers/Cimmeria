//! ORG-06: `.org_disband <orgId>` is forwarded to the base as
//! `OrgCellToBase::GmDisband` with the GM's own character, GM-gated like
//! every `.` command; a malformed id never leaves the cell.

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
    e.player_id = Some(7101);
    e.account_id = Some(8101);
    e.character_name = Some("Gm".to_owned());
    e.access_level = gm_access;
    (mgr, gm)
}

async fn say(mgr: &mut SpaceManager, speaker: u32, text: &str) -> Vec<CellToBaseMsg> {
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
        out.push(msg);
    }
    out
}

fn disbands(msgs: &[CellToBaseMsg]) -> Vec<&OrgCellToBase> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::Org(o @ OrgCellToBase::GmDisband { .. }) => Some(o),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn org_disband_forwards_to_the_base() {
    let (mut mgr, gm) = fixture(2);
    let msgs = say(&mut mgr, gm, ".org_disband 42").await;
    assert_eq!(
        disbands(&msgs),
        vec![&OrgCellToBase::GmDisband {
            player_id: 7101,
            entity_id: gm,
            org_id: 42,
        }]
    );
}

#[tokio::test]
async fn org_disband_with_a_bad_id_stays_on_the_cell() {
    let (mut mgr, gm) = fixture(2);
    let capture = LogCapture::install();
    let msgs = say(&mut mgr, gm, ".org_disband forty").await;
    assert!(disbands(&msgs).is_empty());
    let row = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "org.disband"))
        .expect("org.disband row");
    assert_eq!((row.level, row.target.as_str()), (Level::INFO, "org"));
    assert!(
        row.has_field("reason", "org_id_invalid"),
        "{:?}",
        row.fields
    );
}

#[tokio::test]
async fn org_disband_from_a_non_gm_is_not_forwarded() {
    let (mut mgr, gm) = fixture(0);
    let msgs = say(&mut mgr, gm, ".org_disband 42").await;
    assert!(disbands(&msgs).is_empty());
}
