//! Wire-format and fan-out cardinality tests for the two gate sequences.
//!
//! TESTING.md type 2 (wire format): the `onSequence` argument bytes for
//! `Stargate_MakeGate` (6100) and `Stargate_CrossGate` (6113) are pinned
//! byte-for-byte against the layout in `SGWPlayer.py:2112` / `:2124` —
//! `(seqId, entityId, entityId, 1, gameTime, [], KISMET_VIEW_EventInvoker,
//! 0)`, with `ImpactTime` sent as 0.0 (we don't forward the game clock;
//! the client uses it only as an animation offset and the ring path has
//! always sent 0.0).
//!
//! The fan-out cardinality half is here rather than in the base crate
//! because it is about how many `WitnessEntityMethod` messages the CELL
//! emits; the byte-level routing of those messages is pinned in
//! `base/world_entry/cell_dispatch/tests_dispatch_arms/stargate_fanout.rs`.

use tokio::sync::mpsc;

use super::super::sequences::{
    send_gate_sequence, EVENT_STARGATE_CROSS_GATE, EVENT_STARGATE_MAKE_GATE,
};
use super::{
    grant_all_addresses, make_manager_with_stargates, CASTLE_EVENT_SET, SEQ_CROSS_GATE,
    SEQ_MAKE_GATE,
};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_SEQUENCE;

/// The 26 bytes `onSequence` carries for sequence `seq_id` fired by
/// entity `eid`, spelled out literally so a change to the builder has to
/// be made twice.
fn expected_args(seq_id: i32, eid: u32) -> Vec<u8> {
    let s = seq_id.to_le_bytes();
    let e = (eid as i32).to_le_bytes();
    vec![
        s[0], s[1], s[2], s[3], // KismetEventSetSeqID
        e[0], e[1], e[2], e[3], // SourceID   = the dialer
        e[0], e[1], e[2], e[3], // TargetID   = the dialer
        1,    // PrimaryTarget
        0, 0, 0, 0, // ImpactTime = 0.0f32
        0, 0, 0, 0, // NameValuePairs count = 0
        3, // ViewType = KISMET_VIEW_EventInvoker
        0, 0, 0, 0, // InstanceId
    ]
}

fn one_player(mgr: &mut SpaceManager, eid: u32) {
    mgr.create_entity(eid, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    grant_all_addresses(mgr, eid);
    if let Some(e) = mgr.get_entity_mut(eid) {
        e.is_player = true;
        e.player_id = Some(eid as i32);
    }
    mgr.connect_entity(eid);
}

#[tokio::test]
async fn make_gate_sequence_has_the_expected_wire_bytes() {
    let mut mgr = make_manager_with_stargates();
    one_player(&mut mgr, 7);

    let (tx, mut rx) = mpsc::channel(16);
    let seq = send_gate_sequence(
        7,
        Some(CASTLE_EVENT_SET),
        EVENT_STARGATE_MAKE_GATE,
        &tx,
        &mgr,
    )
    .await;
    assert_eq!(
        seq,
        Some(SEQ_MAKE_GATE),
        "event 6100 on set 10011 resolves to sequence 10145 \
         (event_sets_sequences + sequences) — never a hardcoded id"
    );

    let msg = rx.try_recv().expect("one frame for the dialer");
    match msg {
        CellToBaseMsg::WitnessEntityMethod {
            witness_id,
            entity_id,
            method_index,
            args,
            entity_is_player,
        } => {
            assert_eq!(witness_id, 7);
            assert_eq!(entity_id, 7);
            assert_eq!(method_index, ON_SEQUENCE, "onSequence is client method 1");
            assert!(entity_is_player, "the dialer is a player — idbase 61");
            assert_eq!(args, expected_args(SEQ_MAKE_GATE, 7));
        }
        other => panic!("expected WitnessEntityMethod, got {other:?}"),
    }
    assert!(
        rx.try_recv().is_err(),
        "exactly one frame with no witnesses"
    );
}

#[tokio::test]
async fn cross_gate_sequence_has_the_expected_wire_bytes() {
    let mut mgr = make_manager_with_stargates();
    one_player(&mut mgr, 7);

    let (tx, mut rx) = mpsc::channel(16);
    let seq = send_gate_sequence(
        7,
        Some(CASTLE_EVENT_SET),
        EVENT_STARGATE_CROSS_GATE,
        &tx,
        &mgr,
    )
    .await;
    assert_eq!(seq, Some(SEQ_CROSS_GATE), "event 6113 → sequence 10158");

    let msg = rx.try_recv().expect("one frame for the crosser");
    match msg {
        CellToBaseMsg::WitnessEntityMethod { args, .. } => {
            assert_eq!(args, expected_args(SEQ_CROSS_GATE, 7));
        }
        other => panic!("expected WitnessEntityMethod, got {other:?}"),
    }
}

/// Fan-out cardinality: the dialer plus every witness, each exactly once.
/// A regression that dropped the witness loop leaves one message; one
/// that forgot to dedupe the dialer leaves four.
#[tokio::test]
async fn gate_sequence_reaches_every_witness_and_the_dialer_exactly_once() {
    let mut mgr = make_manager_with_stargates();
    for eid in [7u32, 8, 9] {
        one_player(&mut mgr, eid);
    }
    // `get_witnesses_of(7)` scans the space's players for those whose
    // own `witnesses` set contains 7 — set that up directly rather than
    // running an AoI pass, so the test pins the fan-out, not the AoI.
    for observer in [8u32, 9] {
        mgr.get_entity_mut(observer)
            .unwrap()
            .witnesses
            .insert(cimmeria_common::EntityId(7));
    }

    let witnesses = mgr.get_witnesses_of(7);
    assert_eq!(
        witnesses.len(),
        2,
        "fixture precondition: players 8 and 9 must witness 7, got {witnesses:?}"
    );

    let (tx, mut rx) = mpsc::channel(64);
    send_gate_sequence(
        7,
        Some(CASTLE_EVENT_SET),
        EVENT_STARGATE_MAKE_GATE,
        &tx,
        &mgr,
    )
    .await;

    let mut recipients = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args,
                ..
            } => {
                assert_eq!(entity_id, 7, "the observee is always the dialer");
                assert_eq!(method_index, ON_SEQUENCE);
                assert_eq!(
                    args,
                    expected_args(SEQ_MAKE_GATE, 7),
                    "every witness gets the identical frame"
                );
                recipients.push(witness_id);
            }
            other => panic!("unexpected message {other:?}"),
        }
    }
    recipients.sort_unstable();
    assert_eq!(
        recipients,
        vec![7, 8, 9],
        "the dialer and both witnesses, one frame each — no amplification, \
         no missing observer"
    );
}

/// A gate whose `stargates.event_set_id` is NULL, or whose event set has
/// no sequence for the event, must emit nothing rather than a frame with
/// a bogus sequence id.
#[tokio::test]
async fn missing_event_set_or_sequence_emits_nothing() {
    let capture = crate::test_support::LogCapture::install();
    let mut mgr = make_manager_with_stargates();
    one_player(&mut mgr, 7);
    let (tx, mut rx) = mpsc::channel(16);

    assert_eq!(
        send_gate_sequence(7, None, EVENT_STARGATE_MAKE_GATE, &tx, &mgr).await,
        None
    );
    assert!(rx.try_recv().is_err(), "NULL event set → no frame");

    // Event set present but the (set, event) pair is not in the map.
    // 6103 is `Stargate_DestroyGate` — deliberately never wired (D-CA10),
    // so it doubles as the unmapped-event probe.
    assert_eq!(
        send_gate_sequence(7, Some(CASTLE_EVENT_SET), 6103, &tx, &mgr).await,
        None
    );
    assert!(rx.try_recv().is_err(), "unmapped event → no frame");

    // Negative-logging convention: both misses are silent on the wire, so
    // the log is the only signal that a gate opened with no animation.
    // Distinct `reason` values keep "this world's gate has no event set"
    // apart from "this event set is missing that sequence" — different
    // seed fixes.
    assert!(
        capture
            .find_event(
                tracing::Level::WARN,
                "origin gate has no event_set_id",
                "gate_event_set_missing",
            )
            .is_some(),
        "a NULL stargates.event_set_id must WARN with \
         reason=gate_event_set_missing. Captured events: {:#?}",
        capture.all()
    );
    assert!(
        capture
            .find_event(
                tracing::Level::WARN,
                "not in event_sets_sequences map",
                "gate_sequence_unmapped",
            )
            .is_some(),
        "an event set with no sequence for the event must WARN with \
         reason=gate_sequence_unmapped. Captured events: {:#?}",
        capture.all()
    );
}
