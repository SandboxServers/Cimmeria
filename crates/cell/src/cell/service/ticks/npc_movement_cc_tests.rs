//! Crowd control on the NPC movement tick (ability mechanics AB-09a/b): a
//! stun or knockdown freezes the NPC's route, a snare slows its steps. Both
//! arrive as timed-effect ledger entries, the way the effect scripts apply
//! them.

use std::time::{Duration, Instant};

use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::MOVEMENT_SPEED_MOD;

use super::*;
use crate::cell::space_manager::SpaceManager;

const NPC: u32 = 200;

/// One NPC at the origin walking a 5 u/tick route to x = 10.
fn walking_npc() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    let xml = r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#;
    mgr.parse_spaces_xml(xml).unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(NPC, "Castle", [0.0; 3], [0.0; 3])
        .unwrap();
    let npc = mgr.get_entity_mut(NPC).unwrap();
    npc.is_player = false;
    npc.class_id = 0x04;
    npc.move_speed = 5.0;
    npc.nav_path
        .push_back(cimmeria_common::Vector3::new(20.0, 0.0, 0.0));
    mgr
}

fn entry(effect_id: i32, stats: Vec<(i32, i32)>, state_flags: u32) -> TimedEffectSpec {
    TimedEffectSpec {
        effect_id,
        ability_id: 1,
        invoker_id: 1,
        stats,
        state_flags,
        duration_secs: Some(5.0),
        stacking: TimedStacking::PerSource,
        ..Default::default()
    }
}

/// **Regression guard (stuns did nothing to NPCs).** A stunned NPC does not
/// step along its route and reports zero velocity, keeps the route, and
/// walks on once the stun comes off. Before AB-09 the tick never read
/// `BSF_MovementLock`, so the NPC stepped to x = 5 (`stunned position`
/// fails).
#[test]
fn a_stunned_npc_holds_its_route_until_the_lock_clears() {
    let mut mgr = walking_npc();
    npc_movement_tick(&mut mgr);
    assert_eq!(mgr.get_entity(NPC).unwrap().position.x, 5.0);

    mgr.apply_timed_effect(NPC, entry(1599, vec![], BSF_MOVEMENT_LOCK), Instant::now())
        .unwrap();
    for _ in 0..3 {
        npc_movement_tick(&mut mgr);
    }
    let npc = mgr.get_entity(NPC).unwrap();
    assert_eq!(npc.position.x, 5.0, "stunned position");
    assert_eq!(npc.velocity, [0.0; 3], "no run-in-place velocity");
    assert_eq!(npc.nav_path.len(), 1, "the route is kept");

    let _ = mgr.remove_timed_effects(
        NPC,
        crate::cell::effects::stat_buff::StatBuffRemoval::Expired,
        |_| true,
    );
    npc_movement_tick(&mut mgr);
    assert_eq!(mgr.get_entity(NPC).unwrap().position.x, 10.0, "walks on");
}

/// A -30 % snare (`movementSpeedMod` -30) steps the NPC 70 % as far, and its
/// expiry restores full speed.
#[test]
fn a_snared_npc_chases_at_the_reduced_speed() {
    let mut mgr = walking_npc();
    let now = Instant::now();
    mgr.apply_timed_effect(NPC, entry(1462, vec![(MOVEMENT_SPEED_MOD, -30)], 0), now)
        .unwrap();
    npc_movement_tick(&mut mgr);
    assert!((mgr.get_entity(NPC).unwrap().position.x - 3.5).abs() < 1e-5);

    let later = now + Duration::from_secs(6);
    let _ = mgr.remove_timed_effects(
        NPC,
        crate::cell::effects::stat_buff::StatBuffRemoval::Expired,
        |e| e.is_expired(later),
    );
    npc_movement_tick(&mut mgr);
    assert!((mgr.get_entity(NPC).unwrap().position.x - 8.5).abs() < 1e-5);
}
