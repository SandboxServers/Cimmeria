//! Packet P38 regression suite: `.net_timer <id> <type> [totalTime]
//! [secondaryId]` — the hand-rolled `onTimerUpdate` (client method 12)
//! argument block.
//!
//! Filter prefix: `legacy_p38_`.
//!
//! `onTimerUpdate` is `(INT32 Id, UINT8 Type, INT32 SourceID, INT32
//! SecondaryId, FLOAT TotalTime, FLOAT BigWorldTimeComplete)` — 21 bytes
//! (`entities/defs/interfaces/SGWBeing.def`). Issue #719: this buffer used to
//! omit `SecondaryId`, so a live client read `TotalTime` as `SecondaryId` and
//! ran off the end of the message. It is the one `onTimerUpdate` site built
//! by hand rather than through `serialize_timer_update`, so nothing else pins
//! its layout.
//!
//! The trailing `BigWorldTimeComplete` float is the absolute expiry on the
//! server's game clock, asserted as a window below.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::setup;
use crate::cell::console::exec;
use crate::cell::messages::CellToBaseMsg;

const ON_TIMER_UPDATE: u16 = 12;

/// Every `onTimerUpdate` payload sent directly to `entity_id`.
fn drain_timer_updates(rx: &mut mpsc::Receiver<CellToBaseMsg>, entity_id: u32) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id: eid,
            method_index: ON_TIMER_UPDATE,
            args,
        } = msg
        {
            if eid == entity_id {
                out.push(args);
            }
        }
    }
    out
}

#[tokio::test]
async fn legacy_p38_net_timer_emits_byte_exact_21_byte_block_with_secondary_id() {
    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);

    exec(
        "net_timer",
        gm,
        &["7", "2", "12.5", "99"],
        None,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let payloads = drain_timer_updates(&mut rx, gm);
    assert_eq!(payloads.len(), 1, "exactly one onTimerUpdate to the caller");
    let args = &payloads[0];
    assert_eq!(
        args.len(),
        21,
        "onTimerUpdate is 21 bytes; 17 means SecondaryId was dropped (#719)"
    );

    let mut expected = Vec::new();
    expected.extend_from_slice(&7i32.to_le_bytes()); // Id
    expected.push(2); // Type
    expected.extend_from_slice(&(gm as i32).to_le_bytes()); // SourceID = caller
    expected.extend_from_slice(&99i32.to_le_bytes()); // SecondaryId
    expected.extend_from_slice(&12.5f32.to_le_bytes()); // TotalTime
    assert_eq!(
        &args[..17],
        expected.as_slice(),
        "Id, Type, SourceID, SecondaryId, TotalTime must sit at 0, 4, 5, 9, 13"
    );
}

#[tokio::test]
async fn legacy_p38_net_timer_defaults_secondary_id_to_zero() {
    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);

    exec("net_timer", gm, &["7", "2"], None, &tx, &mut mgr, &engine).await;

    let payloads = drain_timer_updates(&mut rx, gm);
    assert_eq!(payloads.len(), 1, "exactly one onTimerUpdate to the caller");
    let args = &payloads[0];
    assert_eq!(args.len(), 21);
    assert_eq!(
        &args[9..13],
        &0i32.to_le_bytes(),
        "omitted secondaryId defaults to 0, and still occupies its four bytes"
    );
    assert_eq!(
        &args[13..17],
        &1.0f32.to_le_bytes(),
        "omitted totalTime defaults to 1.0"
    );
}

/// `BigWorldTimeComplete` is `totalTime` after the game clock at the
/// command, as `Net.py:93` sent it (`Atrea.getGameTime() + totalTime`).
#[tokio::test]
async fn legacy_p38_net_timer_expiry_is_absolute_on_the_game_clock() {
    use crate::mercury::game_clock::{game_time_secs, init};

    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);
    // Past the epoch, so a relative expiry cannot pass the window.
    init();
    while game_time_secs() < 0.01 {
        std::thread::sleep(std::time::Duration::from_millis(2));
    }

    let before = game_time_secs();
    exec(
        "net_timer",
        gm,
        &["7", "2", "12.5"],
        None,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;
    let after = game_time_secs();

    let payloads = drain_timer_updates(&mut rx, gm);
    assert_eq!(payloads.len(), 1);
    let expiry = f32::from_le_bytes(payloads[0][17..21].try_into().unwrap());
    assert!(
        (before + 12.5..=after + 12.5).contains(&expiry),
        "BigWorldTimeComplete {expiry} is not game time [{before}, {after}] + 12.5"
    );
}

#[tokio::test]
async fn legacy_p38_net_timer_type_is_signed_int8() {
    let (mut mgr, gm, _npc) = setup();
    let engine = ChainEngine::new();
    let (tx, mut rx) = mpsc::channel(64);

    // `Type` is INT8 in `SGWBeing.def`: -1 is a legal value and encodes as
    // 0xFF; 200 does not fit and must be rejected, not truncated.
    exec("net_timer", gm, &["7", "-1"], None, &tx, &mut mgr, &engine).await;
    exec("net_timer", gm, &["7", "200"], None, &tx, &mut mgr, &engine).await;

    let payloads = drain_timer_updates(&mut rx, gm);
    assert_eq!(payloads.len(), 1, "only the in-range type is sent");
    assert_eq!(payloads[0][4], 0xFF, "Type -1 encodes as one signed byte");
}
