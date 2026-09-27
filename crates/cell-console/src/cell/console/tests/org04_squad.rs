//! ORG-04: the `.squad_*` console commands reach the squad handlers
//! through the chat interceptor, GM-gated like every `.` command.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::super::chat::{handle_chat_message, CHAN_SAY};
use super::{decode_feedback, setup};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;

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

/// A GM's `.squad_join Sentinel` founds a squad the sentinel leads, and
/// `.squad_info` then lists it.
#[tokio::test]
async fn gm_squad_join_then_info() {
    let (mut mgr, gm) = fixture(2);

    let msgs = say(&mut mgr, gm, ".squad_join Sentinel").await;
    let sid = mgr.squads.squad_of(7001).expect("the GM joined a squad");
    assert_eq!(mgr.squads.squad_of(7002), Some(sid));
    assert_eq!(mgr.squads.squad(sid).unwrap().leader_player_id(), 7002);
    assert!(
        feedback_to(&msgs, gm).contains(&"You joined Sentinel's squad.".to_owned()),
        "{msgs:?}"
    );

    let msgs = say(&mut mgr, gm, ".squad_info").await;
    let lines = feedback_to(&msgs, gm);
    assert_eq!(
        lines[0],
        format!("Squad {sid}: 2 members, loot round robin.")
    );
    assert_eq!(lines.len(), 3, "{lines:?}");
}

/// `.squad_invite Sentinel` issues a real invite the sentinel can answer.
#[tokio::test]
async fn gm_squad_invite_issues_an_invite() {
    let (mut mgr, gm) = fixture(2);
    let msgs = say(&mut mgr, gm, ".squad_invite Sentinel").await;
    assert_eq!(
        mgr.squads.pending_for(7002, std::time::Instant::now()),
        1,
        "{msgs:?}"
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
