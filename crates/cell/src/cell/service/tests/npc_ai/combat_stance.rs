//! An NPC that fights is announced `BSF_InCombat` to its witnesses before its
//! first shot, and announced clear when it stops.
//!
//! Colo telemetry, 2026-09-29 (session `dc4c716a`): the identical `onSequence`
//! (ability 579, sequence 3) spawned an `EmitterSpawnable` and a `sing` sound
//! on the client when the player was the Source, and nothing for 11 shots in a
//! row from guard 100307. The legacy `SGWMob.aiBeginCombat` set the bit; the
//! Rust NPC side never did. See `npc_ai/combat_stance.rs`.

use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::spawner::EVENT_ABILITY_END;
use crate::mercury::method_idx::{ON_SEQUENCE, ON_STATE_FIELD_UPDATE};
use crate::test_support::LogCapture;
use tokio::sync::mpsc;

const NPC: u32 = 200;
const TARGET: u32 = 101;
const EVENT_SET: i32 = 3;
const END_SEQ: i32 = 3;
const IN_COMBAT: u32 = 1 << 3;

fn add_player(mgr: &mut SpaceManager, id: u32, pos: [f32; 3]) {
    mgr.create_entity(id, "Castle", pos, [0.0; 3]).unwrap();
    let p = mgr.get_entity_mut(id).unwrap();
    p.is_player = true;
    p.player_id = Some(id as i32);
    if let Some(h) = p.stats.get_mut(HEALTH) {
        h.update(0, 100, 100);
        h.clear_dirty();
    }
    mgr.connect_entity(id);
}

/// A Fighting NPC with Pistol Shot wired to a real Ability_End, one player
/// 10 u away who is both its target and its only witness.
fn fixture() -> SpaceManager {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    seed_default_ability(&mut mgr, 0, 30);
    mgr.ability_defs
        .get_mut(&crate::cell::combat::NPC_DEFAULT_ABILITY)
        .unwrap()
        .event_set_id = Some(EVENT_SET);
    mgr.sequence_map
        .insert((EVENT_SET, EVENT_ABILITY_END), END_SEQ);
    add_player(&mut mgr, TARGET, [10.0, 0.0, 0.0]);
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.abilities
        .add_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
    npc.threat_list.insert(TARGET, 10.0);
    let _ = mgr.compute_aoi_changes();
    assert_eq!(mgr.get_witnesses_of(NPC), vec![TARGET]);
    mgr
}

async fn tick(mgr: &mut SpaceManager) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(512);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new()),
    )
    .await;
    let mut msgs = Vec::new();
    while let Ok(m) = rx.try_recv() {
        msgs.push(m);
    }
    msgs
}

/// `(method_index, args)` of each NPC method the witness was sent, in order.
fn npc_methods_to(msgs: &[CellToBaseMsg], witness: u32) -> Vec<(u16, Vec<u8>)> {
    msgs.iter()
        .filter_map(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                witness_id,
                entity_id: NPC,
                method_index,
                args,
                ..
            } if *witness_id == witness => Some((*method_index, args.clone())),
            _ => None,
        })
        .collect()
}

/// The first Fighting tick sends the witness `onStateFieldUpdate` with
/// `BSF_InCombat` set, and that message precedes the Ability_End
/// `onSequence` on the same channel. Reverting `sync_combat_stance` (or
/// moving it after the ability launch) fails the ordering assertion.
#[tokio::test]
async fn a_fighting_npc_is_announced_in_combat_before_its_first_shot() {
    let mut mgr = fixture();
    let logs = LogCapture::install();

    let msgs = tick(&mut mgr).await;

    let sent = npc_methods_to(&msgs, TARGET);
    let stance = sent
        .iter()
        .position(|(m, _)| *m == ON_STATE_FIELD_UPDATE)
        .unwrap_or_else(|| panic!("no onStateFieldUpdate reached the witness: {sent:?}"));
    let shot = sent
        .iter()
        .position(|(m, _)| *m == ON_SEQUENCE)
        .unwrap_or_else(|| panic!("no onSequence reached the witness: {sent:?}"));
    assert!(
        stance < shot,
        "the stance must precede the first shot: {sent:?}"
    );
    let field = u32::from_le_bytes(sent[stance].1[..4].try_into().unwrap());
    assert_ne!(
        field & IN_COMBAT,
        0,
        "BSF_InCombat set on the wire: {field:#x}"
    );
    assert_ne!(
        mgr.get_entity(NPC).unwrap().state_field & IN_COMBAT,
        0,
        "the entity carries the bit, so a witness arriving mid-fight gets it in the AoI cascade"
    );
    assert!(
        logs.all()
            .into_iter()
            .any(|c| c.target == "npc_ai.stance" && c.has_field("event", "in_combat")),
        "an npc_ai.stance in_combat row: {:#?}",
        logs.all()
    );
    assert!(
        !logs
            .all()
            .into_iter()
            .any(|c| c.target == "abilities.sequence"
                && c.has_field("outcome", "stance_not_announced")),
        "a Fighting NPC's shot must not trip the stance WARN"
    );
}

/// The announcement is sent once per fight, not once per tick.
#[tokio::test]
async fn the_stance_is_not_repeated_while_the_fight_continues() {
    let mut mgr = fixture();
    let first = tick(&mut mgr).await;
    let second = tick(&mut mgr).await;

    let count = |m: &[CellToBaseMsg]| {
        npc_methods_to(m, TARGET)
            .iter()
            .filter(|(i, _)| *i == ON_STATE_FIELD_UPDATE)
            .count()
    };
    assert_eq!(count(&first), 1, "announced on the first tick");
    assert_eq!(count(&second), 0, "not again on the next: {second:#?}");
}

/// Leaving Fighting clears the bit on the entity and on the wire.
#[tokio::test]
async fn an_npc_that_leaves_the_fight_is_announced_out_of_combat() {
    let mut mgr = fixture();
    let _ = tick(&mut mgr).await;

    // The surrender path: `npc_ai_submit` clears the bit raw and sends the
    // witnesses nothing of its own, so the stance sync is the only thing that
    // can tell them (Leashing would mask it: the leash broadcasts the field).
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.threat_list.clear();
    crate::cell::service::npc_ai::force_ai_state(npc, AiState::Submit);

    let msgs = tick(&mut mgr).await;
    let cleared = npc_methods_to(&msgs, TARGET)
        .into_iter()
        .filter(|(m, _)| *m == ON_STATE_FIELD_UPDATE)
        .map(|(_, a)| u32::from_le_bytes(a[..4].try_into().unwrap()))
        .next_back()
        .unwrap_or_else(|| panic!("no state update after leaving the fight: {msgs:#?}"));
    assert_eq!(cleared & IN_COMBAT, 0, "cleared on the wire: {cleared:#x}");
}
