//! `Action::MovementLock` (cellblock-boot-lock D-BL3): the hold's gates, the
//! ref-counted `BSF_MOVEMENT_LOCK` flag, the release line, and the end-to-end
//! lock on a client position update.
//!
//! Revert proof: drop the `contains` early return in `movement_lock::run` and
//! `a_second_hold_does_not_double_count_the_flag` fails on the flag bit.

use cimmeria_content_engine::movement_lock::MovementLockOp;

use super::*;
use crate::cell::client_methods::communicator::ON_PLAYER_COMMUNICATION;
use crate::cell::ring_transport::wire_helpers::{update_state_flag, BSF_MOVEMENT_LOCK};
use crate::cell::space_manager::{ClientMoveOutcome, MovementHold};
use crate::test_support::LogCapture;

const PLAYER: u32 = 31;
const PLAYER_ID: i32 = 4343;
const PROP: u32 = 32;
const TAG: &str = "BootCtl";
const CHAIN: i64 = 7;

/// The Agnos startup world: the player at the origin and the anchor prop at
/// (1, 0, 1), tagged [`TAG`].
fn world() -> SpaceManager {
    let mut mgr = make_space_mgr();
    mgr.create_entity(PLAYER, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(PLAYER).unwrap();
    p.is_player = true;
    p.player_id = Some(PLAYER_ID);
    mgr.connect_entity(PLAYER);
    mgr.create_entity(PROP, "Agnos", [1.0, 0.0, 1.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(PROP).unwrap().tag = Some(TAG.into());
    mgr
}

fn hold(at: [f32; 3], client_flag: bool) -> MovementLockOp {
    MovementLockOp::Hold {
        anchor_tag: TAG.to_string(),
        at,
        max_distance: 3.0,
        client_flag,
        message: Some("held".to_string()),
    }
}

fn release(message: Option<&str>) -> MovementLockOp {
    MovementLockOp::Release {
        message: message.map(str::to_string),
    }
}

/// Fire one `movement_lock` action on [`CHAIN`] for the player.
async fn fire(mgr: &mut SpaceManager, op: MovementLockOp) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(64);
    execute_actions(
        ResolvedActions {
            action_delays: Vec::new(),
            params: std::collections::HashMap::new(),
            actions: vec![(CHAIN, Action::MovementLock(op))],
        },
        PLAYER,
        PLAYER_ID,
        &tx,
        mgr,
        &ChainEngine::new(),
    )
    .await;
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn says(msgs: &[CellToBaseMsg], text: &str) -> bool {
    let needle: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
    msgs.iter().any(|m| match m {
        CellToBaseMsg::EntityMethodCall {
            entity_id: PLAYER,
            method_index: ON_PLAYER_COMMUNICATION,
            args,
        } => args.windows(needle.len()).any(|w| w == needle.as_slice()),
        _ => false,
    })
}

fn flag_set(mgr: &SpaceManager) -> bool {
    mgr.get_entity(PLAYER).unwrap().state_field & BSF_MOVEMENT_LOCK != 0
}

#[tokio::test]
async fn hold_pins_the_player_and_says_why() {
    let mut mgr = world();
    let msgs = fire(&mut mgr, hold([0.0; 3], false)).await;
    let held = mgr
        .get_entity(PLAYER)
        .unwrap()
        .extensions
        .get::<MovementHold>()
        .expect("the player is held");
    assert_eq!(held.anchor, [0.0; 3]);
    assert_eq!(held.chain_id, CHAIN);
    assert!(says(&msgs, "held"), "{msgs:#?}");
}

#[tokio::test]
async fn hold_is_skipped_far_from_the_anchor() {
    let capture = LogCapture::install();
    let mut mgr = world();
    let msgs = fire(&mut mgr, hold([10.0, 0.0, 0.0], false)).await;
    assert!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .extensions
            .get::<MovementHold>()
            .is_none(),
        "no hold far from the anchor"
    );
    assert!(!says(&msgs, "held"), "{msgs:#?}");
    assert!(
        capture
            .all()
            .iter()
            .any(|c| c.has_field("reason", "hold_skipped_far_from_anchor")),
        "the skip is logged"
    );
}

#[tokio::test]
async fn hold_is_skipped_without_the_prop() {
    let capture = LogCapture::install();
    let mut mgr = world();
    mgr.get_entity_mut(PROP).unwrap().tag = None;
    let msgs = fire(&mut mgr, hold([0.0; 3], false)).await;
    assert!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .extensions
            .get::<MovementHold>()
            .is_none(),
        "no hold without the anchor prop"
    );
    assert!(!says(&msgs, "held"), "{msgs:#?}");
    assert!(
        capture
            .all()
            .iter()
            .any(|c| c.has_field("reason", "hold_skipped_no_anchor")),
        "the skip is logged"
    );
}

#[tokio::test]
async fn a_second_hold_does_not_double_count_the_flag() {
    let mut mgr = world();
    fire(&mut mgr, hold([0.0; 3], true)).await;
    fire(&mut mgr, hold([0.0; 3], true)).await;
    fire(&mut mgr, release(None)).await;
    assert_eq!(
        mgr.get_entity(PLAYER).unwrap().state_field & BSF_MOVEMENT_LOCK,
        0,
        "one release clears the one flag the holds set"
    );
}

#[tokio::test]
async fn release_leaves_another_holders_flag() {
    let mut mgr = world();
    fire(&mut mgr, hold([0.0; 3], true)).await;
    let (tx, _rx) = mpsc::channel(64);
    update_state_flag(PLAYER, BSF_MOVEMENT_LOCK, true, &tx, &mut mgr).await;
    fire(&mut mgr, release(None)).await;
    assert!(flag_set(&mgr), "the second holder still holds the flag");
}

#[tokio::test]
async fn release_removes_the_hold_and_says_the_line() {
    let mut mgr = world();
    fire(&mut mgr, hold([0.0; 3], false)).await;
    let msgs = fire(&mut mgr, release(Some("free"))).await;
    assert!(
        mgr.get_entity(PLAYER)
            .unwrap()
            .extensions
            .get::<MovementHold>()
            .is_none(),
        "the release removes the hold"
    );
    assert!(says(&msgs, "free"), "{msgs:#?}");
}

#[tokio::test]
async fn release_without_a_hold_still_says_the_line() {
    let mut mgr = world();
    let msgs = fire(&mut mgr, release(Some("free"))).await;
    assert!(says(&msgs, "free"), "{msgs:#?}");
}

#[tokio::test]
async fn a_held_player_is_locked_end_to_end() {
    let mut mgr = world();
    fire(&mut mgr, hold([0.0; 3], false)).await;
    let outcome = mgr.apply_client_position_update(PLAYER, [5.0, 0.0, 0.0], [0; 3], [0.0; 3]);
    assert!(
        matches!(outcome, ClientMoveOutcome::Locked { .. }),
        "{outcome:?}"
    );

    fire(&mut mgr, release(None)).await;
    let outcome = mgr.apply_client_position_update(PLAYER, [5.0, 0.0, 0.0], [0; 3], [0.0; 3]);
    assert!(
        !matches!(outcome, ClientMoveOutcome::Locked { .. }),
        "{outcome:?}"
    );
}
