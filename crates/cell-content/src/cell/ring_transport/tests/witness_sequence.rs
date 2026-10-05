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

use super::super::dispatch::dispatch_effects;
use super::super::runtime::advance_destination_after_warmup;
use super::support::{spawn_player, three_ring_mgr, FakeClock};
use crate::cell::messages::CellToBaseMsg;
use crate::cell::ring_transport::{handle_region_trigger, handle_select_destination, State};
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx::ON_SEQUENCE;
use cimmeria_wire::cell::kismet::{build_on_sequence_args, KISMET_VIEW_EVENT_INVOKER};

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

/// Every `onSequence` the traveller's own client was sent, as raw args.
fn owner_sequence_args(rx: &mut mpsc::Receiver<CellToBaseMsg>, traveller: u32) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let CellToBaseMsg::EntityMethodCall {
            entity_id,
            method_index,
            args,
        } = msg
        {
            if method_index == ON_SEQUENCE && entity_id == traveller {
                out.push(args);
            }
        }
    }
    out
}

/// Ring 1 -> ring 2, same world: select, step on, hide, warmup (teleport and
/// the destination's `all_players_loaded`), as the production tick drives it.
/// Returns after the destination has played Teleport In.
async fn ring_trip(
    mgr: &mut SpaceManager,
    traveller: u32,
    tx: &mpsc::Sender<CellToBaseMsg>,
    rx: &mut mpsc::Receiver<CellToBaseMsg>,
    engine: &ChainEngine,
) -> (Vec<Vec<u8>>, Vec<(u32, u32, i32)>) {
    mgr.get_entity_mut(traveller).unwrap().ring_source_id = Some(1);
    handle_select_destination(1, 2, traveller, tx, mgr, engine).await;
    while rx.try_recv().is_ok() {}
    handle_region_trigger(2001, true, traveller, tx, mgr, engine).await;
    let hide = mgr
        .ring_transporters
        .get_mut(1)
        .unwrap()
        .hide_timer_expired();
    dispatch_effects(hide, tx, mgr, engine).await;
    let dst = mgr.ring_regions.get(&2).unwrap();
    let (pos, world) = ([dst.x, dst.y, dst.z], dst.world_name.clone());
    let players = mgr.ring_transporters.get(1).unwrap().send_players.clone();
    let warmup = mgr
        .ring_transporters
        .get_mut(1)
        .unwrap()
        .warmup_timer_expired(pos, &world);
    advance_destination_after_warmup(2, players, tx, mgr, engine).await;
    dispatch_effects(warmup, tx, mgr, engine).await;
    assert_eq!(
        mgr.ring_transporters.get(2).unwrap().state,
        State::RemoteWarmup,
        "the destination must have played Teleport In"
    );
    let mut owner = Vec::new();
    let mut witnesses = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        match msg {
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                args,
            } if method_index == ON_SEQUENCE && entity_id == traveller => owner.push(args),
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id,
                method_index,
                args,
                ..
            } if method_index == ON_SEQUENCE => {
                let seq = i32::from_le_bytes(args[..4].try_into().unwrap());
                witnesses.push((witness_id, entity_id, seq));
            }
            _ => {}
        }
    }
    (owner, witnesses)
}

/// Teleport In (sequence 9001) also reaches a client that witnesses the
/// traveller when the destination fires it.
///
/// What this does and does not cover: the cell refreshes witness lists on its
/// AoI tick, not inside the teleport, so when Teleport In fires (same tick as
/// the teleport) the witnesses are the clients that already saw the traveller,
/// which here stands in for a player near both pads. A player who only sees
/// the destination pad is not a witness yet and gets no copy. A witness that
/// does get one may still drop it client-side: `FUN_00d06f30` erases a
/// view-type-3 request whose source entity has no pawn, and the traveller is
/// hidden until `ShowPlayer`. docs/gameplay/ring-transport-system.md records
/// both as a known cosmetic limitation; DA-06 checks it on two clients.
///
/// Reverting the fan-out fails the witness assertion.
#[tokio::test]
async fn teleport_in_sequence_reaches_a_witness_of_the_arriving_traveller() {
    let clock = FakeClock::new();
    let mut mgr = three_ring_mgr(clock);
    spawn_player(&mut mgr, 42, 700); // traveller
    spawn_player(&mut mgr, 45, 703); // watching, within AoI of both pads
    mgr.get_entity_mut(45)
        .unwrap()
        .witnesses
        .insert(EntityId(42));
    let (tx, mut rx) = mpsc::channel(256);
    let engine = ChainEngine::new();

    let (owner, witnesses) = ring_trip(&mut mgr, 42, &tx, &mut rx, &engine).await;
    assert_eq!(
        owner.len(),
        2,
        "Teleport Out and In to the traveller: {owner:?}"
    );
    assert_eq!(
        witnesses,
        vec![(45, 42, 9000), (45, 42, 9001)],
        "the watcher sees the source rings rise and the destination rings drop"
    );
}

/// The traveller's own client gets exactly what it got before the witness
/// fan-out: one owner `onSequence` per phase, byte for byte the frame
/// `build_on_sequence_args(seq, traveller, 3)` builds, with or without
/// witnesses present. Guards the fan-out against changing the stock ring
/// traffic (Castle, Harset, Lucia, Omega) for the traveller.
#[tokio::test]
async fn the_travellers_own_sequences_are_unchanged_by_the_witness_fan_out() {
    let expected = |seq: i32| build_on_sequence_args(seq, 42, KISMET_VIEW_EVENT_INVOKER);
    for with_witness in [false, true] {
        let clock = FakeClock::new();
        let mut mgr = three_ring_mgr(clock);
        spawn_player(&mut mgr, 42, 700);
        spawn_player(&mut mgr, 43, 701);
        if with_witness {
            mgr.get_entity_mut(43)
                .unwrap()
                .witnesses
                .insert(EntityId(42));
        }
        let (tx, mut rx) = mpsc::channel(256);
        let engine = ChainEngine::new();
        mgr.get_entity_mut(42).unwrap().ring_source_id = Some(1);
        handle_select_destination(1, 2, 42, &tx, &mut mgr, &engine).await;
        while rx.try_recv().is_ok() {}
        handle_region_trigger(2001, true, 42, &tx, &mut mgr, &engine).await;
        assert_eq!(
            owner_sequence_args(&mut rx, 42),
            vec![expected(9000)],
            "Teleport Out to the traveller (witness present: {with_witness})"
        );

        let mut mgr2 = three_ring_mgr(FakeClock::new());
        spawn_player(&mut mgr2, 42, 700);
        spawn_player(&mut mgr2, 43, 701);
        if with_witness {
            mgr2.get_entity_mut(43)
                .unwrap()
                .witnesses
                .insert(EntityId(42));
        }
        let (owner, _) = ring_trip(&mut mgr2, 42, &tx, &mut rx, &engine).await;
        assert_eq!(
            owner,
            vec![expected(9000), expected(9001)],
            "Teleport Out and In to the traveller (witness present: {with_witness})"
        );
    }
}
