//! The D-SS25 guard: an interactable NPC stays interactable after a duel
//! starts and ends.
//!
//! AoI makes an NPC clickable by sending `onDuelEntitiesRemove` [152] with
//! the NPC's id, which forces the client to recompute that NPC's
//! interaction flags (SS-E1 D-Q5). The duel's own sends, 151 at the engage
//! and 153 at the end, act on the same client-side set, so the guard pins
//! what makes them safe: 151 names only the two duelists, 153 carries no
//! ids, the duel never sends 152, and neither the NPC's server-side
//! interaction flags nor AoI's 152 on a later introduction change.

use std::time::Instant;

use tokio::sync::mpsc;

use cimmeria_common::Vector3;
use cimmeria_wire::cell::client_methods::duel::build_on_duel_entities_set;

use super::*;
use crate::cell::duel::limits::{COUNTDOWN, ENGAGED_LIMIT};
use crate::cell::duel::tick::run_at;

const NPC: u32 = 50;
const TEMPLATE: i32 = 7;
const INT_TALK: i64 = 0x4;

/// The 152 sends AoI queued for A about `npc`.
fn interactable_sends(events: &[CellToBaseMsg], npc: u32) -> usize {
    events
        .iter()
        .filter(|m| {
            matches!(m, CellToBaseMsg::EntityMethodCall { entity_id: A_EID, method_index: 152, args }
                if *args == (npc as i32).to_le_bytes().to_vec())
        })
        .count()
}

#[tokio::test]
async fn interactable_npc_stays_interactable_across_a_duel() {
    let mut mgr = make_mgr();
    for eid in [A_EID, B_EID, C_EID] {
        mgr.get_entity_mut(eid).unwrap().archetype_id = Some(1);
    }
    mgr.spawn_npc(NPC, "Agnos", [2.0, 0.0, 2.0], [0.0; 3])
        .unwrap();
    {
        let npc = mgr.get_entity_mut(NPC).unwrap();
        npc.template_id = Some(TEMPLATE);
        npc.has_dynamic_properties = true;
        npc.interaction_type_flags = 0;
    }
    mgr.get_entity_mut(A_EID)
        .unwrap()
        .available_interactions
        .insert(TEMPLATE, vec![(1, None, INT_TALK)]);

    // AoI introduces the NPC to A as interactable.
    let events = mgr.compute_aoi_changes();
    assert_eq!(interactable_sends(&events, NPC), 1, "AoI's 152 for the NPC");

    // The duel starts and ends.
    let (tx, mut rx) = mpsc::channel(256);
    let t0 = Instant::now();
    challenge(&mut mgr, &tx, (A_EID, A_PID), (B_EID, B_PID), t0).await;
    crate::cell::duel::response::handle_at(B_EID, &[1], &tx, &mut mgr, t0).await;
    run_at(&tx, &mut mgr, t0 + COUNTDOWN).await;
    run_at(&tx, &mut mgr, t0 + COUNTDOWN + ENGAGED_LIMIT).await;
    let sent = drain(&mut rx);

    let set = build_on_duel_entities_set(&[A_EID as i32, B_EID as i32]);
    for s in &sent {
        match s.method_index {
            151 => assert_eq!(s.args, set, "151 names only the duelists: {s:?}"),
            153 => assert!(s.args.is_empty(), "153 carries no ids: {s:?}"),
            152 => panic!("the duel sent onDuelEntitiesRemove: {s:?}"),
            _ => {}
        }
    }
    assert_eq!(own(&sent, A_EID, 151).len(), 1);
    assert_eq!(own(&sent, A_EID, 153).len(), 1);
    assert_eq!(
        mgr.get_entity(NPC).unwrap().interaction_type_flags,
        0,
        "the NPC's base interaction flags are untouched"
    );
    assert_eq!(
        mgr.get_entity(A_EID).unwrap().available_interactions[&TEMPLATE],
        vec![(1, None, INT_TALK)],
        "A's interaction binds are untouched"
    );

    // A walks out of range and back: AoI still makes the NPC interactable.
    mgr.get_entity_mut(A_EID).unwrap().position = Vector3::new(900.0, 0.0, 0.0);
    let _ = mgr.compute_aoi_changes_for_player(A_EID);
    mgr.get_entity_mut(A_EID).unwrap().position = Vector3::new(0.0, 0.0, 0.0);
    let events = mgr.compute_aoi_changes_for_player(A_EID);
    assert_eq!(
        interactable_sends(&events, NPC),
        1,
        "AoI's 152 still fires after the duel"
    );
}
