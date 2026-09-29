//! An NPC's shot: the witnesses get the fire animation (`onSequence`) and
//! no `onTimerUpdate`.
//!
//! The client binds `onTimerUpdate` on `SGWPlayer` only (bind sweep
//! `0x00db3390`), so an NPC cooldown timer sent to a witness is dropped by
//! `Client_NetIn_EntityMethodDispatch`. Colo 2026-09-29: 191
//! `client.dispatch.method_dropped` (method 12, type 4) in one session, one
//! per NID Guard shot.

use super::sequence::{scene, NPC, ONLOOKER, SHOOTER};
use super::*;
use crate::cell::spawner::EVENT_ABILITY_END;

const NPC_ABILITY: i32 = 592;
const NPC_EVENT_SET: i32 = 77;
const NPC_END_SEQ: i32 = 7701;

/// Reverting the fix (`handle.rs` back to `send_entity_method(entity_id, 12,
/// ..)` and the `messaging` guard removed) fails the "no timer" assertion:
/// each witness gets a 21-byte method 12 about the NPC again. Verified
/// 2026-09-29. With only `handle.rs` reverted, the guard in
/// `send_entity_method` still holds the line and WARNs.
#[tokio::test]
async fn an_npc_shot_sends_its_witnesses_the_animation_and_no_timer() {
    let mut mgr = scene();
    mgr.get_entity_mut(NPC)
        .unwrap()
        .abilities
        .add_ability(NPC_ABILITY);
    let mut def = make_ability(NPC_ABILITY, 0, 40);
    def.cooldown = 2.0;
    def.event_set_id = Some(NPC_EVENT_SET);
    mgr.ability_defs.insert(NPC_ABILITY, def);
    mgr.sequence_map
        .insert((NPC_EVENT_SET, EVENT_ABILITY_END), NPC_END_SEQ);
    let witnesses = mgr.get_witnesses_of(NPC);
    assert!(
        witnesses.contains(&SHOOTER) && witnesses.contains(&ONLOOKER),
        "fixture: both players see the NPC"
    );
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(NPC, NPC_ABILITY, SHOOTER as i32, &tx, &mut mgr).await);
    let msgs = drain(&mut rx);

    let timers: Vec<&CellToBaseMsg> = msgs
        .iter()
        .filter(|m| match m {
            CellToBaseMsg::WitnessEntityMethod {
                entity_id,
                method_index,
                ..
            } => *entity_id == NPC && *method_index == ON_TIMER_UPDATE,
            CellToBaseMsg::EntityMethodCall {
                entity_id,
                method_index,
                ..
            } => *entity_id == NPC && *method_index == ON_TIMER_UPDATE,
            _ => false,
        })
        .collect();
    assert!(
        timers.is_empty(),
        "an NPC cooldown timer reached a client that has no handler for it: {timers:#?}"
    );

    for witness in [SHOOTER, ONLOOKER] {
        let fired = msgs.iter().any(|m| {
            matches!(
                m,
                CellToBaseMsg::WitnessEntityMethod {
                    witness_id,
                    entity_id: NPC,
                    method_index: method_idx::ON_SEQUENCE,
                    args,
                    ..
                } if *witness_id == witness && super::sequence::first_u32(args) == NPC_END_SEQ
            )
        });
        assert!(
            fired,
            "witness {witness} must still get the NPC's Ability_End onSequence: {msgs:#?}"
        );
    }
}

/// The same shot from a player still sends the player's own cooldown bar.
#[tokio::test]
async fn a_players_shot_still_sends_their_own_cooldown() {
    let mut mgr = scene();
    let (tx, mut rx) = mpsc::channel(256);

    assert!(handle_use_ability(SHOOTER, 559, NPC as i32, &tx, &mut mgr).await);
    let own: Vec<Vec<u8>> = drain(&mut rx)
        .into_iter()
        .filter_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: SHOOTER,
                method_index: ON_TIMER_UPDATE,
                args,
            } => Some(args),
            _ => None,
        })
        .collect();
    assert_eq!(own.len(), 1, "one cooldown timer to the shooter");
    assert_eq!(own[0].len(), 21);
    assert_eq!(&own[0][0..4], &559i32.to_le_bytes(), "ID");
    assert_eq!(
        own[0][4],
        cimmeria_entity::abilities::TIMER_ABILITY_COOLDOWN as u8,
        "Type"
    );
    assert_eq!(&own[0][5..9], &(SHOOTER as i32).to_le_bytes(), "SourceID");
}

const ON_TIMER_UPDATE: u16 = crate::cell::client_methods::being::ON_TIMER_UPDATE;
