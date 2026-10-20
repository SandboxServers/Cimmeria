//! Tests for the `StartMinigame` effect script (MG-24). The world is the
//! effect target: entity 1 is a player (`player_id` 100) in space `W`, and
//! casts on itself. Each test dispatches the script by its registry name, so
//! a missing registry row fails the first test.

use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::cell_entity::PlayerIdentity;
use cimmeria_wire::state_field::BSF_PLAYING_MINIGAME;

use super::test_fixtures::make_mgr_with_target;
use super::{dispatch_by_name, EffectContext};
use crate::cell::space_manager::SpaceManager;

/// Dispatch `StartMinigame` for `source` cast at `target`, with `params`. The
/// script is found and runs; a refusal shows only in the queue.
fn cast_between(mgr: &mut SpaceManager, source: u32, target: u32, params: &[(&str, &str)]) {
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
        source_id: source,
        target_id: target,
        effect: &effect,
        space_mgr: mgr,
    };
    assert!(
        dispatch_by_name("StartMinigame", &mut ctx),
        "StartMinigame is registered"
    );
}

/// Dispatch `StartMinigame` for entity 1 casting on itself, with `params`.
fn cast(mgr: &mut SpaceManager, params: &[(&str, &str)]) {
    cast_between(mgr, 1, 1, params);
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
    // Take the first launch off the queue: a second cast for the same caster
    // is refused while one waits (`second_cast_before_a_drain_queues_nothing`).
    let mut queued = std::mem::take(&mut mgr.minigame_launch.queue);
    cast(&mut mgr, &[("minigame_type", "Livewire")]);
    queued.extend(std::mem::take(&mut mgr.minigame_launch.queue));

    let difficulties: Vec<u32> = queued.iter().map(|r| r.difficulty).collect();
    assert_eq!(difficulties, vec![5, 1]);
}

/// MG-24 review N2: one caster gets one queued launch per drain, so an AoE or
/// a second effect of one cast does not queue two.
/// Revert: without the queue clause of `already_playing` the queue holds two.
#[test]
fn second_cast_before_a_drain_queues_nothing() {
    let mut mgr = make_mgr_with_target();

    cast(&mut mgr, &[("minigame_type", "Livewire")]);
    cast(&mut mgr, &[("minigame_type", "Livewire")]);

    assert_eq!(mgr.minigame_launch.queue.len(), 1);
}

/// MG-24 review N4 (D-MG20): the caster plays, not the target. Entity 2 is an
/// NPC cast on by player 1.
/// Revert: queuing `ctx.target_id` as the entity fails this test.
#[test]
fn caster_plays_not_the_target() {
    let mut mgr = make_mgr_with_target();
    mgr.create_entity(2, "W", [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    cast_between(&mut mgr, 1, 2, &[("minigame_type", "Livewire")]);

    let queue = &mgr.minigame_launch.queue;
    assert_eq!(queue.len(), 1, "{queue:?}");
    assert_eq!(queue[0].entity_id, 1);
    assert_eq!(queue[0].player_id, 100);
}

/// MG-24 review N4: an NPC caster with a player target queues nothing, since
/// the target's player never plays the caster's game.
/// Revert: queuing for the target (player 1) with the caster's player id
/// would launch a game for the wrong player.
#[test]
fn npc_caster_with_a_player_target_queues_nothing() {
    let mut mgr = make_mgr_with_target();
    mgr.create_entity(2, "W", [1.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    cast_between(&mut mgr, 2, 1, &[("minigame_type", "Livewire")]);

    assert!(mgr.minigame_launch.queue.is_empty());
}

/// MG-24 review F1: the caster's id now names another player (it was recycled
/// after the invoker's snapshot was taken). The deferred cast names the old
/// player, so nothing is queued for the new occupant.
/// Revert: without the `caster_gone` check the launch goes to the new player.
#[test]
fn recycled_caster_id_queues_nothing() {
    let mut mgr = make_mgr_with_target();
    let departed = PlayerIdentity {
        player_id: Some(55),
        ..Default::default()
    };
    mgr.enter_effect_scope(None, 1, departed, Some("Departed"));
    mgr.get_entity_mut(1).unwrap().player_id = Some(200);

    cast(&mut mgr, &[("minigame_type", "Livewire")]);

    assert!(mgr.minigame_launch.queue.is_empty());
}
