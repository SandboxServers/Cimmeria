//! DA-08: a ring trip's Kismet sequence reaches every witness, not only the
//! traveller.
//!
//! The rings are level actors in each client's copy of the map. A client
//! that is never told the sequence id watches the passenger fade out while
//! the pad stands still. The 2009 server sent the sequence to the traveller
//! only; Cimmeria fans it out the way gate travel already does.

use tokio::sync::mpsc;

use cimmeria_common::EntityId;
use cimmeria_content_engine::chain::ChainEngine;

use super::support::{spawn_player, three_ring_mgr, FakeClock};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::ring_transport::{handle_region_trigger, handle_select_destination};
use crate::mercury::method_idx::ON_SEQUENCE;

/// (owner sends of onSequence to the traveller, witness sends as (witness,
/// subject, sequence id)).
fn sequence_sends(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> (Vec<u32>, Vec<(u32, u32, i32)>) {
    let mut owner = Vec::new();
    let mut witnesses = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                ..
            } if method_index == ON_SEQUENCE => owner.push(entity_id),
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                ref args,
                entity_is_player,
            } if method_index == ON_SEQUENCE => {
                assert!(entity_is_player, "the subject is a player");
                let seq = i32::from_le_bytes(args[..4].try_into().unwrap());
                witnesses.push((witness_id, entity_id, seq));
            }
            _ => {}
        }
    }
    (owner, witnesses)
}

/// Stepping onto the pad plays Region_Teleport_Out (sequence 9000 in the
/// fixture) on the traveller's client and on every client that witnesses the
/// traveller. A player who is not a witness gets nothing.
///
/// Reverting the fan-out in `wire_helpers::send_play_sequence` leaves the
/// witness list empty and fails the second assertion.
#[tokio::test]
async fn teleport_out_sequence_reaches_every_witness_of_the_traveller() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock);
    spawn_player(&mut mgr, 42, 700); // traveller
    spawn_player(&mut mgr, 43, 701); // standing beside the pad
    spawn_player(&mut mgr, 44, 702); // elsewhere: not a witness
    mgr.get_entity_mut(43)
        .unwrap()
        .witnesses
        .insert(EntityId(42));
    // The traveller witnesses the bystander too; that must not echo the
    // sequence back to the traveller as a witness send.
    mgr.get_entity_mut(42)
        .unwrap()
        .witnesses
        .insert(EntityId(43));
    let (tx, mut rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    mgr.get_entity_mut(42).unwrap().ring_source_id = Some(1);
    handle_select_destination(1, 2, 42, &tx, &mut mgr, &engine).await;
    let _ = sequence_sends(&mut rx);
    handle_region_trigger(2001, true, 42, &tx, &mut mgr, &engine).await;

    let (owner, witnesses) = sequence_sends(&mut rx);
    assert_eq!(owner, vec![42], "the traveller keeps its own onSequence");
    assert_eq!(
        witnesses,
        vec![(43, 42, 9000)],
        "exactly the bystander who witnesses the traveller sees the rings rise"
    );
}
