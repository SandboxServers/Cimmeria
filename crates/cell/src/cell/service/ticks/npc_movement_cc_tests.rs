//! Crowd control on the NPC movement tick (ability mechanics AB-09a/b): a
//! stun or knockdown freezes the NPC's route, a snare slows its steps. Both
//! arrive as timed-effect ledger entries, the way the effect scripts apply
//! them.

use std::collections::HashMap;
use std::time::Instant;

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};

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

/// Run `script` for `effect` from caster 7 on the NPC (`on_remove` when
/// `remove`), through the installed registry.
fn run(mgr: &mut SpaceManager, effect: &EffectDef, remove: bool) {
    let name = effect.script_name.clone().unwrap();
    let mut ctx = crate::cell::effects::EffectContext {
        source_id: 7,
        target_id: NPC,
        effect,
        space_mgr: mgr,
    };
    if remove {
        crate::cell::effects::dispatch_on_remove(&name, &mut ctx);
    } else {
        assert!(crate::cell::effects::dispatch_by_name(&name, &mut ctx));
    }
}

fn effect(id: i32, script: &str, nvp: &str, value: &str, secs: f32) -> EffectDef {
    EffectDef {
        effect_id: id,
        ability_id: 717,
        script_name: Some(script.to_string()),
        pulse_count: 1,
        pulse_duration: secs,
        params: HashMap::from([(nvp.to_string(), value.to_string())]),
        ..Default::default()
    }
}

/// **Regression guard (snare and slow stacking on an NPC's chase).** Snare
/// Shot's -30 snare steps the NPC 70 % as far; a Tranquilizer dart on top
/// (-40) and a deep snare (-70) stall it; when they come off, in the order
/// that clamped the old dart, it steps its full 5 u again. The pre-ledger
/// `MovementSlow` restored 40 it had only partly taken, so the NPC ran at
/// 110 % afterwards (`full speed after` fails).
#[test]
fn snares_and_slows_set_an_npcs_chase_speed_and_revert_exactly() {
    let mut mgr = walking_npc();
    crate::test_support::install_effect_scripts(&mut mgr);
    let snare = effect(1462, "TimedStat", "MovementSpeedMod", "-30", 15.0);
    run(&mut mgr, &snare, false);
    npc_movement_tick(&mut mgr);
    let x = mgr.get_entity(NPC).unwrap().position.x;
    assert!((x - 3.5).abs() < 1e-5, "snared step: {x}");

    let deep = effect(1463, "TimedStat", "MovementSpeedMod", "-70", 15.0);
    let dart = effect(9142, "MovementSlow", "SpeedReduction", "40", 6.0);
    run(&mut mgr, &snare, true);
    run(&mut mgr, &deep, false);
    run(&mut mgr, &dart, false);
    npc_movement_tick(&mut mgr);
    assert_eq!(mgr.get_entity(NPC).unwrap().position.x, x, "stalled");

    run(&mut mgr, &deep, true);
    run(&mut mgr, &dart, true);
    npc_movement_tick(&mut mgr);
    let step = mgr.get_entity(NPC).unwrap().position.x - x;
    assert!(
        (step - 5.0).abs() < 1e-5,
        "full speed after: stepped {step}"
    );
}
