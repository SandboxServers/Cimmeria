//! Tests for the `StartMinigame` effect script (MG-24). The world is the
//! effect target: entity 1 is a player (`player_id` 100) in space `W`, and
//! casts on itself. Each test dispatches the script by its registry name, so
//! a missing registry row fails the first test.

use cimmeria_entity::abilities::EffectDef;
use cimmeria_wire::state_field::BSF_PLAYING_MINIGAME;

use super::test_fixtures::make_mgr_with_target;
use super::{dispatch_by_name, EffectContext};
use crate::cell::space_manager::SpaceManager;

/// Dispatch `StartMinigame` for entity 1 casting on itself, with `params`. The
/// script is found and runs; a refusal shows only in the queue.
fn cast(mgr: &mut SpaceManager, params: &[(&str, &str)]) {
    let effect = EffectDef {
        effect_id: 1643,
        ability_id: 1,
        script_name: Some("StartMinigame".to_string()),
        params: params
            .iter()
            .map(|&(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    };
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: mgr,
    };
    assert!(
        dispatch_by_name("StartMinigame", &mut ctx),
        "StartMinigame is registered"
    );
}

/// A player caster queues one launch with the named game, the difficulty and
/// the `effect_script` source.
/// Revert: without the registry row `dispatch_by_name` finds no script and
/// nothing is queued, so this fails.
#[test]
fn player_caster_queues_one_launch() {
    let mut mgr = make_mgr_with_target();

    let params = [
        ("minigame_type", "ConverseBasicHumanoid"),
        ("difficulty", "2"),
    ];
    cast(&mut mgr, &params);

    let queue = &mgr.minigame_launch.queue;
    assert_eq!(queue.len(), 1, "{queue:?}");
    assert_eq!(queue[0].game_name, "ConverseBasicHumanoid");
    assert_eq!(queue[0].difficulty, 2);
    assert_eq!(queue[0].source, "effect_script");
    assert_eq!(queue[0].entity_id, 1);
    assert_eq!(queue[0].player_id, 100);
}

/// An NPC caster (no player id) queues nothing.
/// Revert: dropping the `npc_caster` refusal queues a launch for an NPC.
#[test]
fn npc_caster_queues_nothing() {
    let mut mgr = make_mgr_with_target();
    mgr.get_entity_mut(1).unwrap().player_id = None;

    cast(&mut mgr, &[("minigame_type", "Livewire")]);

    assert!(mgr.minigame_launch.queue.is_empty());
}

/// A caster already holding the playing bit queues nothing.
/// Revert: dropping the `already_playing` refusal queues a second launch.
#[test]
fn caster_already_playing_queues_nothing() {
    let mut mgr = make_mgr_with_target();
    mgr.get_entity_mut(1).unwrap().state_field |= BSF_PLAYING_MINIGAME;

    cast(&mut mgr, &[("minigame_type", "Livewire")]);

    assert!(mgr.minigame_launch.queue.is_empty());
}

/// An unknown or missing `minigame_type` queues nothing.
/// Revert: dropping the name check queues a launch the client cannot play.
#[test]
fn unknown_or_missing_minigame_type_queues_nothing() {
    let mut mgr = make_mgr_with_target();

    cast(&mut mgr, &[("minigame_type", "Bogus")]);
    cast(&mut mgr, &[("minigame_type", "livewire")]);
    cast(&mut mgr, &[("difficulty", "2")]);

    assert!(mgr.minigame_launch.queue.is_empty());
}

/// A difficulty above 5 clamps to 5, and a missing one is 1.
/// Revert: without the clamp the queued difficulty is 9.
#[test]
fn difficulty_clamps_to_the_range_and_defaults_to_one() {
    let mut mgr = make_mgr_with_target();

    cast(
        &mut mgr,
        &[("minigame_type", "Livewire"), ("difficulty", "9")],
    );
    cast(&mut mgr, &[("minigame_type", "Livewire")]);

    let difficulties: Vec<u32> = mgr
        .minigame_launch
        .queue
        .iter()
        .map(|r| r.difficulty)
        .collect();
    assert_eq!(difficulties, vec![5, 1]);
}
