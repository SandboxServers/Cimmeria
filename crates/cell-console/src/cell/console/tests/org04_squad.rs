//! ORG-04: the `.squad_*` console commands reach the squad handlers
//! through the chat interceptor, GM-gated like every `.` command, and each
//! writes one `org.gm_action` row (TESTING.md type 12).

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::super::chat::{handle_chat_message, CHAN_SAY};
use super::{decode_feedback, exec, setup};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::{Captured, LogCapture, LogCaptureGuard};
use tracing::Level;

const SENTINEL: u32 = 2;

/// The console fixture's GM (entity 1) and a sentinel player (entity 2),
/// both initialised characters. `gm_access` is the caller's access level.
fn fixture(gm_access: u32) -> (SpaceManager, u32) {
    let (mut mgr, gm, _npc) = setup();
    mgr.create_entity(SENTINEL, "Agnos", [11.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(SENTINEL);
    for (eid, pid, name) in [(gm, 7001, "Gm"), (SENTINEL, 7002, "Sentinel")] {
        let e = mgr.get_entity_mut(eid).unwrap();
        e.player_id = Some(pid);
        e.account_id = Some(pid as u32 + 1000);
        e.character_name = Some(name.to_owned());
    }
    mgr.get_entity_mut(gm).unwrap().access_level = gm_access;
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

fn feedback_to(msgs: &[CellToBaseMsg], eid: u32) -> Vec<String> {
    msgs.iter()
        .filter(
            |m| matches!(m, CellToBaseMsg::EntityMethodCall { entity_id, .. } if *entity_id == eid),
        )
        .filter_map(decode_feedback)
        .collect()
}

/// The single `org.gm_action` row, which must be INFO on `org`.
fn gm_row(capture: &LogCaptureGuard) -> Captured {
    let rows: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("event", "org.gm_action"))
        .collect();
    assert_eq!(rows.len(), 1, "{rows:#?}");
    let row = rows.into_iter().next().unwrap();
    assert_eq!((row.level, row.target.as_str()), (Level::INFO, "org"));
    row
}

fn assert_fields(row: &Captured, want: &[(&str, &str)]) {
    for (k, v) in want {
        assert!(row.has_field(k, v), "{k}={v}: {:?}", row.fields);
    }
}

/// A GM's `.squad_join Sentinel` founds a squad the sentinel leads, and
/// `.squad_info` then lists it; each writes its GM row.
#[tokio::test]
async fn gm_squad_join_then_info() {
    let (mut mgr, gm) = fixture(2);
    let capture = LogCapture::install();

    let msgs = say(&mut mgr, gm, ".squad_join Sentinel").await;
    let sid = mgr.squads.squad_of(7001).expect("the GM joined a squad");
    assert_eq!(mgr.squads.squad_of(7002), Some(sid));
    assert_eq!(mgr.squads.squad(sid).unwrap().leader_player_id(), 7002);
    assert!(
        feedback_to(&msgs, gm).contains(&"You joined Sentinel's squad.".to_owned()),
        "{msgs:?}"
    );
    assert_fields(
        &gm_row(&capture),
        &[
            ("action", "gm_squad_join"),
            ("outcome", "ok"),
            ("player_id", "7001"),
            ("account_id", "8001"),
            ("entity_id", "1"),
            ("target_player_id", "7002"),
            ("target_account_id", "8002"),
            ("squad_id", &sid.to_string()),
        ],
    );
    drop(capture);

    let capture = LogCapture::install();
    let msgs = say(&mut mgr, gm, ".squad_info Sentinel").await;
    assert_eq!(
        feedback_to(&msgs, gm),
        vec![
            format!("Squad {sid}: 2 members, loot round robin."),
            "  Sentinel (leader, level 1, player 7002): entity 2".to_owned(),
            "  Gm (member, level 1, player 7001): entity 1".to_owned(),
        ]
    );
    assert_fields(
        &gm_row(&capture),
        &[
            ("action", "gm_squad_info"),
            ("outcome", "ok"),
            ("target_player_id", "7002"),
            ("squad_id", &sid.to_string()),
        ],
    );
}

/// `.squad_info` with no name shows the GM's own state.
#[tokio::test]
async fn gm_squad_info_without_a_squad() {
    let (mut mgr, gm) = fixture(2);
    let msgs = say(&mut mgr, gm, ".squad_info").await;
    assert_eq!(
        feedback_to(&msgs, gm),
        vec!["Gm is not in a squad.".to_owned()]
    );
}

/// A name that is not online is refused by the console before any squad
/// code runs, with a line and a `target_not_found` row.
#[tokio::test]
async fn gm_squad_join_unknown_name_is_refused() {
    let (mut mgr, gm) = fixture(2);
    let capture = LogCapture::install();
    let msgs = say(&mut mgr, gm, ".squad_join Nobody").await;
    assert_eq!(mgr.squads.squad_count(), 0);
    assert_eq!(
        feedback_to(&msgs, gm),
        vec!["No player named Nobody is online.".to_owned()]
    );
    assert_fields(
        &gm_row(&capture),
        &[("outcome", "rejected"), ("reason", "target_not_found")],
    );
}

/// The console's own GM check: a non-GM reaching the dispatcher directly
/// (past the chat gate) changes nothing and is logged `not_gm`.
#[tokio::test]
async fn squad_commands_check_gm_themselves() {
    let (mut mgr, player) = fixture(0);
    let capture = LogCapture::install();
    let (tx, mut rx) = mpsc::channel(16);
    exec(
        "squad_join",
        player,
        &["Sentinel"],
        None,
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;
    assert_eq!(mgr.squads.squad_count(), 0);
    assert!(rx.try_recv().is_ok(), "the caller gets a line");
    assert_fields(
        &gm_row(&capture),
        &[("outcome", "rejected"), ("reason", "not_gm")],
    );
}

/// `.squad_invite Sentinel` issues a real invite the sentinel can answer.
#[tokio::test]
async fn gm_squad_invite_issues_an_invite() {
    let (mut mgr, gm) = fixture(2);
    let capture = LogCapture::install();
    let msgs = say(&mut mgr, gm, ".squad_invite Sentinel").await;
    assert_eq!(
        mgr.squads.pending_for(7002, std::time::Instant::now()),
        1,
        "{msgs:?}"
    );
    assert_fields(
        &gm_row(&capture),
        &[
            ("action", "gm_squad_invite"),
            ("outcome", "ok"),
            ("target_player_id", "7002"),
        ],
    );
}

/// The guard: a player's `.squad_join` is refused as a GM command and
/// joins nothing.
#[tokio::test]
async fn non_gm_squad_join_is_refused() {
    let (mut mgr, player) = fixture(0);
    let msgs = say(&mut mgr, player, ".squad_join Sentinel").await;
    assert_eq!(mgr.squads.squad_count(), 0);
    assert!(
        feedback_to(&msgs, player)
            .iter()
            .any(|l| l.contains("GM command")),
        "{msgs:?}"
    );
}
