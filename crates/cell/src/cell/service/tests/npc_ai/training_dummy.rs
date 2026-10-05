//! D-DA7: a seeded training dummy (`entity_templates.training_dummy`) never
//! fights back, however much threat it holds.
//!
//! The NPC is spawned through `spawn_npc_from_record`, the path the seeded
//! population and a GM `.spawn` take, from a hostile (faction 10) record
//! with Pistol Shot and a player on its threat list in range. The control
//! record without the flag shoots on its first AI tick; the flagged one
//! sends nothing, starts no cooldown and holds still.
//!
//! Revert proofs: drop `apply_training_dummy` from the spawn path and the
//! flagged run fires like the control (and loses its Health); drop the
//! `TrainingDummy` skip from `ai_driven_npc_entity_ids` and it fires too.

use super::*;
use crate::cell::messages::CellToBaseMsg;
use crate::cell::spawner::EVENT_ABILITY_END;
use crate::mercury::method_idx::ON_SEQUENCE;
use cimmeria_cell_world::cell::space_manager::{TrainingDummy, TRAINING_DUMMY_HEALTH};
use cimmeria_cell_world::test_fixtures::npc_spawn_record;
use cimmeria_entity::cell_entity::VaultScope;
use tokio::sync::mpsc;

const TARGET: u32 = 101;

/// A Castle space with a player at (10, 0, 0) and one NPC spawned from a
/// record: faction `faction`, level 50, `training_dummy` as given. Returns
/// the NPC's id.
fn fixture(training_dummy: bool, faction: i32) -> (SpaceManager, u32) {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    // The shared fixture's entity 200 is not under test here.
    mgr.destroy_entity(200);
    seed_default_ability(&mut mgr, 0, 30);
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

    let mut record = npc_spawn_record("Castle", [0.0; 3], 0, VaultScope::Personal);
    record.faction = Some(faction);
    record.level = Some(50);
    record.training_dummy = training_dummy;
    // Mobile, like `lab_dummy`'s fixture NPC: a pinned one may hold fire on
    // a meshless space, which would make the control prove nothing.
    record.is_stationary = false;
    let npc = mgr.allocate_npc_id();
    mgr.spawn_npc_from_record(npc, &record).unwrap();
    let e = mgr.get_entity_mut(npc).unwrap();
    crate::cell::service::npc_ai::force_ai_state(e, AiState::Fighting);
    e.threat_list.insert(TARGET, 10.0);
    let _ = mgr.compute_aoi_changes();
    (mgr, npc)
}

/// Three AI ticks; the `onSequence` sends about `npc`.
async fn tick(mgr: &mut SpaceManager, npc: u32) -> usize {
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
                CellToBaseMsg::WitnessEntityMethod { entity_id, method_index: ON_SEQUENCE, .. }
                    if *entity_id == npc
            )
        })
        .count()
}

#[tokio::test]
async fn a_seeded_training_dummy_never_attacks_its_threat_target() {
    let (mut control, npc) = fixture(false, 10);
    assert!(
        tick(&mut control, npc).await > 0,
        "fixture: the same record without the flag shoots"
    );

    let (mut mgr, npc) = fixture(true, 10);
    assert!(mgr.is_training_dummy(npc), "the spawn put the mark on it");
    assert!(!mgr.ai_driven_npc_entity_ids().contains(&npc));
    assert_eq!(tick(&mut mgr, npc).await, 0, "a dummy sends no attack");
    let e = mgr.get_entity(npc).unwrap();
    assert!(!e
        .abilities
        .is_on_cooldown(crate::cell::combat::NPC_DEFAULT_ABILITY));
    assert!(e.nav_path.is_empty(), "a dummy never paths");
    let hp = e.stats.get(HEALTH).unwrap();
    assert_eq!(
        (hp.cur, hp.max),
        (TRAINING_DUMMY_HEALTH, TRAINING_DUMMY_HEALTH)
    );
    assert_eq!(
        mgr.get_entity(TARGET)
            .unwrap()
            .stats
            .get(HEALTH)
            .unwrap()
            .cur,
        100,
        "the player is untouched"
    );
}

/// The friendly dummy (not faction 10) starts at half its Health, so a heal
/// has room to land; it carries the same mark.
#[tokio::test]
async fn a_friendly_training_dummy_starts_at_half_health() {
    let (mgr, npc) = fixture(true, 9);
    let e = mgr.get_entity(npc).unwrap();
    assert!(e.extensions.contains::<TrainingDummy>());
    let hp = e.stats.get(HEALTH).unwrap();
    assert_eq!(hp.max, TRAINING_DUMMY_HEALTH);
    assert_eq!(hp.cur, TRAINING_DUMMY_HEALTH / 2);
}
