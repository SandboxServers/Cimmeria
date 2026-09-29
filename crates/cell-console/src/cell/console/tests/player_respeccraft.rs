//! `.respeccraft`, the one `.`-console line any player may type: it
//! forwards one `RespecCraftOpen` for the speaker, GM or not, and is never
//! broadcast as chat.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::super::chat::{handle_chat_message, CHAN_SAY};
use super::setup;
use crate::cell::messages::{CellToBaseMsg, RespecCraftOpen};
use crate::cell::space_manager::SpaceManager;

const PLAYER_ID: i32 = 4311;

/// The console fixture's speaker as a loaded character at `access_level`.
fn speaker(access_level: u32) -> (SpaceManager, u32) {
    let (mut mgr, speaker, _npc) = setup();
    let e = mgr.get_entity_mut(speaker).unwrap();
    e.access_level = access_level;
    e.player_id = Some(PLAYER_ID);
    (mgr, speaker)
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

fn opens(msgs: &[CellToBaseMsg]) -> Vec<&RespecCraftOpen> {
    msgs.iter()
        .filter_map(|m| m.plugin_payload::<RespecCraftOpen>())
        .collect()
}

/// The guard: a player (access level 0) can open a respec for themselves,
/// and the line is consumed, not broadcast.
#[tokio::test]
async fn a_players_respeccraft_forwards_one_open_and_is_not_chat() {
    let (mut mgr, player) = speaker(0);
    let msgs = say(&mut mgr, player, ".respeccraft").await;
    assert_eq!(
        opens(&msgs),
        vec![&RespecCraftOpen {
            entity_id: player,
            player_id: PLAYER_ID,
        }]
    );
    assert_eq!(msgs.len(), 1, "nothing else, no chat broadcast: {msgs:?}");
}

#[tokio::test]
async fn a_gms_respeccraft_opens_their_own() {
    let (mut mgr, gm) = speaker(2);
    let msgs = say(&mut mgr, gm, ".respeccraft").await;
    assert_eq!(
        opens(&msgs),
        vec![&RespecCraftOpen {
            entity_id: gm,
            player_id: PLAYER_ID,
        }]
    );
}

/// Arguments are refused with a line and open nothing.
#[tokio::test]
async fn respeccraft_with_arguments_is_refused_with_a_line() {
    let (mut mgr, player) = speaker(0);
    let msgs = say(&mut mgr, player, ".respeccraft now").await;
    assert!(opens(&msgs).is_empty(), "{msgs:?}");
    assert!(
        matches!(
            msgs.as_slice(),
            [CellToBaseMsg::EntityMethodCall { entity_id, method_index: 28, .. }] if *entity_id == player
        ),
        "{msgs:?}"
    );
}

/// A speaker with no character id gets a line and forwards nothing.
#[tokio::test]
async fn respeccraft_without_a_player_id_is_refused_with_a_line() {
    let (mut mgr, player) = speaker(0);
    mgr.get_entity_mut(player).unwrap().player_id = None;
    let msgs = say(&mut mgr, player, ".respeccraft").await;
    assert!(opens(&msgs).is_empty(), "{msgs:?}");
    assert!(
        matches!(
            msgs.as_slice(),
            [CellToBaseMsg::EntityMethodCall {
                method_index: 28,
                ..
            }]
        ),
        "{msgs:?}"
    );
}

/// Other `.`-lines from a player are still ordinary chat.
#[tokio::test]
async fn a_players_other_dot_lines_are_untouched() {
    let (mut mgr, player) = speaker(0);
    let msgs = say(&mut mgr, player, ".respeccraftx").await;
    assert!(opens(&msgs).is_empty(), "{msgs:?}");
}
