//! AB-L2 (D-AU6): a lab dummy never attacks, however much threat it holds.
//!
//! The fixture is `attack_sequence`'s: a Fighting NPC with Pistol Shot and a
//! player in range on its threat list, which fires on the first AI tick. The
//! control run proves the fixture attacks; the marked run must send nothing,
//! start no cooldown and stay where it is.

use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::spawner::EVENT_ABILITY_END;
use crate::mercury::method_idx::ON_SEQUENCE;
use cimmeria_cell_world::cell::space_manager::{LabCaster, LabDummy};
use cimmeria_entity::cell_entity::{MobAggression, PlayerIdentity};
use tokio::sync::mpsc;

const NPC: u32 = 200;
const TARGET: u32 = 101;

fn fixture(marked: bool) -> SpaceManager {
    fixture_with(marked, false)
}

/// `caster` also gives the dummy a [`LabCaster`] mark (`.dummy caster`),
/// due long ago, casting the ability the AI would fire.
fn fixture_with(marked: bool, caster: bool) -> SpaceManager {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    seed_default_ability(&mut mgr, 0, 30);
    // Pistol Shot's seeded Ability_End sequence, so a shot is visible on the
    // wire as an `onSequence` (as in `attack_sequence`).
    mgr.ability_defs
        .get_mut(&crate::cell::combat::NPC_DEFAULT_ABILITY)
        .unwrap()
        .event_set_id = Some(3);
    mgr.sequence_map.insert((3, EVENT_ABILITY_END), 3);
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
    if marked {
        npc.extensions.insert(LabDummy {
            owner_id: TARGET,
            owner_identity: PlayerIdentity::UNKNOWN,
            disposition: MobAggression::Hostile,
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(600),
        });
    }
    if caster {
        npc.extensions.insert(LabCaster {
            ability_id: crate::cell::combat::NPC_DEFAULT_ABILITY,
            interval: std::time::Duration::from_secs(8),
            next_cast_at: std::time::Instant::now(),
        });
    }
    let _ = mgr.compute_aoi_changes();
    mgr
}

/// One AI tick; returns the `onSequence` sends about the NPC.
async fn tick(mgr: &mut SpaceManager) -> usize {
    let (tx, mut rx) = mpsc::channel(512);
    for _ in 0..3 {
        crate::cell::service::npc_ai::npc_ai_tick(
            &tx,
            mgr,
            &crate::cell::content::EngineEvents(&cimmeria_content_engine::chain::ChainEngine::new()),
        )
        .await;
    }
    std::iter::from_fn(|| rx.try_recv().ok())
        .filter(|m| {
            matches!(
                m,
                CellToBaseMsg::WitnessEntityMethod {
                    entity_id: NPC,
                    method_index: ON_SEQUENCE,
                    ..
                }
            )
        })
        .count()
}

/// Revert proof: drop the `LabDummy` skip from
/// `SpaceManager::ai_driven_npc_entity_ids` and the marked run fires exactly
/// like the control.
#[tokio::test]
async fn ab_l2_a_lab_dummy_never_attacks_its_threat_target() {
    let mut control = fixture(false);
    assert!(
        tick(&mut control).await > 0,
        "fixture: an unmarked NPC with this threat shoots on its AI tick"
    );
    assert!(control
        .get_entity(NPC)
        .unwrap()
        .abilities
        .is_on_cooldown(crate::cell::combat::NPC_DEFAULT_ABILITY));

    let mut dummy = fixture(true);
    assert_eq!(tick(&mut dummy).await, 0, "a dummy sends no attack");
    let npc = dummy.get_entity(NPC).unwrap();
    assert!(
        !npc.abilities
            .is_on_cooldown(crate::cell::combat::NPC_DEFAULT_ABILITY),
        "a dummy launches nothing"
    );
    assert!(npc.nav_path.is_empty(), "a dummy never paths");
    assert_eq!(
        (npc.position.x, npc.position.z),
        (0.0, 0.0),
        "a dummy holds still"
    );
    let target = dummy.get_entity(TARGET).unwrap();
    assert_eq!(target.stats.get(HEALTH).unwrap().cur, 100);
}

/// A `.dummy caster` is still out of the AI tick: its casts come only from
/// the console's caster sweep, at its owner, never from a fight turn at a
/// threat target. Revert proof: admit a `LabCaster` NPC in
/// `ai_driven_npc_entity_ids` and it shoots its threat target here.
#[tokio::test]
async fn ab_l2_a_caster_lab_dummy_gets_no_ai_turn_either() {
    let mut dummy = fixture_with(true, true);
    assert!(!dummy.ai_driven_npc_entity_ids().contains(&NPC));
    assert_eq!(tick(&mut dummy).await, 0, "no AI attack");
    let npc = dummy.get_entity(NPC).unwrap();
    assert!(!npc
        .abilities
        .is_on_cooldown(crate::cell::combat::NPC_DEFAULT_ABILITY));
    assert!(npc.nav_path.is_empty());
}
