//! Ability mechanics AB-09a: a stunned or knocked-down NPC holds its fire.
//!
//! The stun arrives the way the `Stun` script lands it: a timed-effect
//! ledger entry holding `BSF_MovementLock`. The fight tick reads the bit and
//! holds (`decision_outcome = "stunned"`); when the entry comes off, the
//! next tick fires.

use std::time::Instant;

use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use tokio::sync::mpsc;

use super::*;
use crate::cell::combat::BSF_MOVEMENT_LOCK;
use crate::cell::effects::stat_buff::StatBuffRemoval;
use crate::cell::messages::CellToBaseMsg;
use crate::mercury::method_idx::ON_SEQUENCE;

const NPC: u32 = 200;
const TARGET: u32 = 101;
const EVENT_SET: i32 = 3;
const END_SEQ: i32 = 3;

/// A Fighting NPC at the origin with Pistol Shot (Ability_End sequence 3)
/// and a player 10 u away on its threat list, watching it.
fn fixture() -> SpaceManager {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    seed_default_ability(&mut mgr, 0, 30);
    mgr.ability_defs
        .get_mut(&crate::cell::combat::NPC_DEFAULT_ABILITY)
        .unwrap()
        .event_set_id = Some(EVENT_SET);
    mgr.sequence_map.insert(
        (EVENT_SET, crate::cell::spawner::EVENT_ABILITY_END),
        END_SEQ,
    );
    mgr.create_entity(TARGET, "Castle", [10.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let p = mgr.get_entity_mut(TARGET).unwrap();
    p.is_player = true;
    p.player_id = Some(TARGET as i32);
    if let Some(h) = p.stats.get_mut(HEALTH) {
        h.update(0, 100, 100);
        h.clear_dirty();
    }
    mgr.connect_entity(TARGET);
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.abilities
        .add_ability(crate::cell::combat::NPC_DEFAULT_ABILITY);
    npc.threat_list.insert(TARGET, 10.0);
    let _ = mgr.compute_aoi_changes();
    mgr
}

fn stun(mgr: &mut SpaceManager) {
    let spec = TimedEffectSpec {
        effect_id: 1599,
        ability_id: 1355,
        invoker_id: TARGET,
        state_flags: BSF_MOVEMENT_LOCK,
        duration_secs: Some(5.0),
        stacking: TimedStacking::PerSource,
        ..Default::default()
    };
    mgr.apply_timed_effect(NPC, spec, Instant::now()).unwrap();
}

/// How many attack animations (`onSequence` from the NPC) one AI tick sent.
async fn shots_in_one_tick(mgr: &mut SpaceManager) -> usize {
    let (tx, mut rx) = mpsc::channel(512);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new()),
    )
    .await;
    let mut shots = 0;
    while let Ok(m) = rx.try_recv() {
        if matches!(
            m,
            CellToBaseMsg::WitnessEntityMethod {
                entity_id: NPC,
                method_index: ON_SEQUENCE,
                ..
            }
        ) {
            shots += 1;
        }
    }
    shots
}

/// **Regression guard (stuns did nothing to NPCs).** A stunned NPC with a
/// target in range launches nothing and its target keeps its health; once
/// the stun comes off it fires on the next tick. Before AB-09 the fight
/// tick never read `BSF_MovementLock` and the stunned NPC fired
/// (`shots while stunned` fails).
#[tokio::test]
async fn a_stunned_npc_does_not_fire_until_the_stun_ends() {
    let mut mgr = fixture();
    stun(&mut mgr);

    assert_eq!(shots_in_one_tick(&mut mgr).await, 0, "shots while stunned");
    assert_eq!(
        mgr.get_entity(TARGET)
            .unwrap()
            .stats
            .get(HEALTH)
            .unwrap()
            .cur,
        100,
        "the target took no hit"
    );
    assert_eq!(
        mgr.get_entity(NPC).unwrap().ai_state(),
        AiState::Fighting,
        "a stun holds the fight, it does not end it"
    );

    let _ = mgr.remove_timed_effects(NPC, StatBuffRemoval::Expired, |_| true);
    assert!(!mgr
        .get_entity(NPC)
        .unwrap()
        .has_state_flag(BSF_MOVEMENT_LOCK));
    assert_eq!(shots_in_one_tick(&mut mgr).await, 1, "fires once free");
}
