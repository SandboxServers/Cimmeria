//! CR-05 `.allcraft` (D-CR17): a GM's line forwards one `GmAllCraft` for the
//! targeted player; a non-GM's `.allcraft` is ordinary chat and forwards
//! nothing, so "craft anywhere" cannot be switched on by a player.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::super::chat::{handle_chat_message, CHAN_SAY};
use super::setup;
use crate::cell::messages::{CellToBaseMsg, GmAllCraft};
use crate::cell::space_manager::SpaceManager;

const TARGET: u32 = 2;
const TARGET_PLAYER_ID: i32 = 4301;

/// The console fixture plus a second player, targeted by the caller.
fn with_player_target(caller_access_level: u32) -> (SpaceManager, u32) {
    let (mut mgr, gm, _npc) = setup();
    mgr.create_entity(TARGET, "Agnos", [11.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    mgr.connect_entity(TARGET);
    mgr.get_entity_mut(TARGET).unwrap().player_id = Some(TARGET_PLAYER_ID);
    let caller = mgr.get_entity_mut(gm).unwrap();
    caller.access_level = caller_access_level;
    caller.current_target_id = Some(TARGET as i32);
    (mgr, gm)
}

async fn say(mgr: &mut SpaceManager, speaker: u32, text: &str) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    handle_chat_message(
        speaker,
        "Speaker",
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

fn grants(msgs: &[CellToBaseMsg]) -> Vec<&GmAllCraft> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::GmAllCraft(g) => Some(g),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn gm_allcraft_forwards_one_grant_for_the_target() {
    let (mut mgr, gm) = with_player_target(2);
    let msgs = say(&mut mgr, gm, ".allcraft").await;
    assert_eq!(
        grants(&msgs),
        vec![&GmAllCraft {
            entity_id: TARGET,
            player_id: TARGET_PLAYER_ID,
            gm_entity_id: gm,
        }]
    );
}

/// The guard: a player's `.allcraft` is chat, not a grant.
#[tokio::test]
async fn non_gm_allcraft_forwards_nothing() {
    let (mut mgr, player) = with_player_target(0);
    let msgs = say(&mut mgr, player, ".allcraft").await;
    assert!(grants(&msgs).is_empty(), "{msgs:?}");
}
