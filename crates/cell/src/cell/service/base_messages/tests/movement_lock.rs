//! `EntityMove` against a content movement hold (cellblock-boot-lock D-BL4).
//! The gate (`client_move_lock`) refuses a move beyond the slack; this file
//! pins what the handler sends back: a snap to the anchor and the hold line.

use super::*;
use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::space_manager::MovementHold;

/// Build a 7777 entity at `spawn_pos` in `Castle_CellBlock` with a hold on it.
fn held_manager(spawn_pos: [f32; 3], message: Option<&str>) -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle_CellBlock" Instanced="true" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(r#"<?xml version="1.0"?><Spaces></Spaces>"#)
        .unwrap();
    mgr.create_entity(7777, "Castle_CellBlock", spawn_pos, [0.0; 3])
        .unwrap();
    let space_id = mgr.get_entity_space_id(7777).unwrap();
    mgr.get_entity_mut(7777)
        .unwrap()
        .extensions
        .insert(MovementHold {
            anchor: spawn_pos,
            space_id,
            chain_id: 1023,
            client_flag: false,
            message: message.map(str::to_owned),
            last_snap: None,
            last_feedback: None,
        });
    mgr
}

/// Send one `EntityMove` to `position` and return what the base was sent.
async fn send_move(
    mgr: &mut SpaceManager,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    tx: &mpsc::Sender<CellToBaseMsg>,
    position: [f32; 3],
) -> Vec<CellToBaseMsg> {
    let engine = ChainEngine::new();
    handle_base_message(
        BaseToCellMsg::EntityMove {
            entity_id: 7777,
            claimed_space_id: 0,
            position,
            direction: [0, 0, 0],
            velocity: [0.0; 3],
        },
        tx,
        mgr,
        &engine,
        &[],
    )
    .await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

/// True when some `ON_PLAYER_COMMUNICATION` call to entity 7777 carries `text`
/// UTF-16LE-encoded.
fn says(msgs: &[CellToBaseMsg], text: &str) -> bool {
    let needle: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    msgs.iter().any(|m| match m {
        CellToBaseMsg::EntityMethodCall {
            entity_id: 7777,
            method_index: ON_PLAYER_COMMUNICATION,
            args,
        } => args.windows(needle.len()).any(|w| w == needle.as_slice()),
        _ => false,
    })
}

fn teleports(msgs: &[CellToBaseMsg]) -> usize {
    msgs.iter()
        .filter(|m| matches!(m, CellToBaseMsg::TeleportPlayer { .. }))
        .count()
}

/// A held player walking 5 m snaps to the anchor and is told why.
#[tokio::test]
async fn a_held_move_snaps_to_the_anchor_and_says_why() {
    let spawn_pos = [10.0_f32, 5.0, 20.0];
    let mut mgr = held_manager(spawn_pos, Some("held"));
    let (tx, mut rx) = mpsc::channel(8);

    let msgs = send_move(&mut mgr, &mut rx, &tx, [15.0, 5.0, 20.0]).await;

    let snaps: Vec<_> = msgs
        .iter()
        .filter_map(|m| match m {
            CellToBaseMsg::TeleportPlayer {
                position, prev_pos, ..
            } => Some((*position, *prev_pos)),
            _ => None,
        })
        .collect();
    assert_eq!(snaps, vec![(spawn_pos, spawn_pos)], "{msgs:#?}");
    assert!(says(&msgs, "held"), "{msgs:#?}");
    assert_eq!(mgr.get_entity(7777).unwrap().position.x, spawn_pos[0]);
}

/// The gate throttles the snap and the message, so a second held move straight
/// after the first sends nothing.
#[tokio::test]
async fn an_immediate_second_held_move_sends_nothing() {
    let mut mgr = held_manager([10.0, 5.0, 20.0], Some("held"));
    let (tx, mut rx) = mpsc::channel(8);

    let first = send_move(&mut mgr, &mut rx, &tx, [15.0, 5.0, 20.0]).await;
    assert_eq!(teleports(&first), 1, "{first:#?}");

    let second = send_move(&mut mgr, &mut rx, &tx, [16.0, 5.0, 20.0]).await;
    assert_eq!(teleports(&second), 0, "{second:#?}");
    assert!(!says(&second, "held"), "{second:#?}");
}

/// A move of 0.5 m is inside the 1 m slack: it is written, not snapped.
#[tokio::test]
async fn a_move_inside_the_slack_is_written() {
    let mut mgr = held_manager([10.0, 5.0, 20.0], Some("held"));
    let (tx, mut rx) = mpsc::channel(8);

    let msgs = send_move(&mut mgr, &mut rx, &tx, [10.5, 5.0, 20.0]).await;

    assert_eq!(teleports(&msgs), 0, "{msgs:#?}");
    assert_eq!(mgr.get_entity(7777).unwrap().position.x, 10.5);
}
