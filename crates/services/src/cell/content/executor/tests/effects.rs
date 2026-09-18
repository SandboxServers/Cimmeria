//! `Action::LaunchAbility` / `Action::ApplyEffect` executor coverage.
//!
//! These pin the server-initiated effect entry point (`content::effect_apply`)
//! that content chains use instead of `handle_use_ability`. The combat entry
//! point rejects a scripted self-debuff three ways — the caster doesn't know
//! the ability, self-target trips the friendly-fire gate, and the path
//! resolves as damage — so the arms below must reach the effect layer
//! directly. Reverting either arm to the `other =>` fallback drops the
//! effect on the floor, which is the shape every guard here reproduces.

use super::*;

use cimmeria_entity::abilities::{AbilityDef, EffectDef};

/// Sentinel ids for the synthetic ability/effect defs. Chosen well clear of
/// the seeded resource ranges so a fixture can never be confused for real
/// content.
const TEST_ABILITY: i32 = 90_001;
const TEST_EFFECT: i32 = 90_002;
const TEST_EFFECT_SINGLE_SHOT: i32 = 90_003;

fn make_ability(ability_id: i32, effect_ids: Vec<i32>) -> AbilityDef {
    AbilityDef {
        ability_id,
        name: format!("TestAbility{ability_id}"),
        cooldown: 0.0,
        warmup: 0.0,
        flags: 0,
        is_ranged: false,
        min_range: 0,
        max_range: 0,
        target_type_id: 1,
        effect_ids,
        moniker_ids: Vec::new(),
        required_ammo: 0,
        event_set_id: None,
        velocity: 0.0,
    }
}

/// A pulsing (DoT-shaped) effect. `pulse_count > 1` is what makes
/// `register_active_effect` store a lasting `ActiveEffectInstance` — a
/// `pulse_count == 1` effect is single-shot and registers nothing.
fn make_pulsing_effect(effect_id: i32, ability_id: i32) -> EffectDef {
    EffectDef {
        effect_id,
        ability_id,
        pulse_count: 4,
        pulse_duration: 2.0,
        ..Default::default()
    }
}

/// Space with a player (entity 1, player 42) and a tagged NPC (entity 101),
/// plus the synthetic ability/effect defs registered on the manager.
fn make_mgr_with_defs() -> SpaceManager {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0; 3], [0.0; 3]).unwrap();
    if let Some(p) = mgr.get_entity_mut(1) {
        p.is_player = true;
        p.player_id = Some(42);
    }
    mgr.spawn_npc(101, "Agnos", [10.0, 0.0, 10.0], [0.0; 3])
        .unwrap();
    if let Some(npc) = mgr.get_entity_mut(101) {
        npc.tag = Some("Drone".to_string());
    }

    mgr.ability_defs
        .insert(TEST_ABILITY, make_ability(TEST_ABILITY, vec![TEST_EFFECT]));
    mgr.effect_defs
        .insert(TEST_EFFECT, make_pulsing_effect(TEST_EFFECT, TEST_ABILITY));
    mgr.effect_defs.insert(
        TEST_EFFECT_SINGLE_SHOT,
        EffectDef {
            effect_id: TEST_EFFECT_SINGLE_SHOT,
            ability_id: TEST_ABILITY,
            pulse_count: 1,
            ..Default::default()
        },
    );
    mgr
}

fn resolved(action: Action) -> ResolvedActions {
    ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(5000, action)],
    }
}

/// The core guard for `Action::LaunchAbility`. A chain firing an ability with
/// `entity_tag: None` must apply that ability's effects to the *triggering
/// player themselves* — the exact self-target the combat pipeline's
/// friendly-fire gate rejects.
///
/// Assert on the registered instance's identity, not just "the list is
/// non-empty": the effect id, the owning ability id, and the invoker must all
/// be right, or a future refactor that registers the wrong def still passes.
#[tokio::test]
async fn launch_ability_registers_effects_on_the_triggering_player() {
    let mut mgr = make_mgr_with_defs();
    let (tx, _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    assert!(
        mgr.get_entity(1).unwrap().active_effects.is_empty(),
        "fixture drift: player must start with no active effects",
    );

    execute_actions(
        resolved(Action::LaunchAbility {
            ability_id: TEST_ABILITY,
            entity_tag: None,
        }),
        1,
        42,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let active = &mgr.get_entity(1).unwrap().active_effects;
    assert_eq!(
        active.len(),
        1,
        "LaunchAbility must register exactly one active effect on the \
         triggering player -- got {active:?}",
    );
    assert_eq!(
        active[0].effect_id, TEST_EFFECT,
        "registered instance must be the ability's own effect",
    );
    assert_eq!(
        active[0].ability_id, TEST_ABILITY,
        "instance must carry the owning ability id (self-sourced from the \
         effect def, not the action)",
    );
    assert_eq!(
        active[0].invoker_id, 1,
        "the triggering player is the invoker for a self-applied content \
         debuff -- a different invoker would stack instead of refresh on \
         re-fire",
    );
}

/// The NPC is never a party to a self-targeted launch. Pins that the effect
/// lands on the player and *only* the player — a target-resolution bug that
/// sprayed the whole space would still satisfy a player-only assertion.
#[tokio::test]
async fn launch_ability_without_tag_does_not_touch_other_entities() {
    let mut mgr = make_mgr_with_defs();
    let (tx, _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    execute_actions(
        resolved(Action::LaunchAbility {
            ability_id: TEST_ABILITY,
            entity_tag: None,
        }),
        1,
        42,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(
        mgr.get_entity(101).unwrap().active_effects.is_empty(),
        "an untagged LaunchAbility targets the trigger source only -- the \
         co-located NPC must be untouched",
    );
}

/// `entity_tag: Some(..)` resolves through `find_entity_by_tag`, so a chain
/// can debuff a named NPC. Asserts both halves: the NPC gets it, the player
/// does not.
#[tokio::test]
async fn launch_ability_with_tag_targets_the_tagged_npc_not_the_player() {
    let mut mgr = make_mgr_with_defs();
    let (tx, _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    execute_actions(
        resolved(Action::LaunchAbility {
            ability_id: TEST_ABILITY,
            entity_tag: Some("Drone".to_string()),
        }),
        1,
        42,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let npc_active = &mgr.get_entity(101).unwrap().active_effects;
    assert_eq!(
        npc_active.len(),
        1,
        "a tagged LaunchAbility must apply to the tagged NPC -- got {npc_active:?}",
    );
    assert_eq!(npc_active[0].effect_id, TEST_EFFECT);
    assert_eq!(
        npc_active[0].invoker_id, 1,
        "the triggering player is the invoker even when the target is an NPC",
    );
    assert!(
        mgr.get_entity(1).unwrap().active_effects.is_empty(),
        "a tagged LaunchAbility must NOT also land on the triggering player",
    );
}

/// Unresolvable tag must be a no-op, never a silent fall-back to self. An
/// NPC debuff landing on the player because the NPC wasn't spawned is the
/// bug shape here.
#[tokio::test]
async fn launch_ability_with_unresolvable_tag_does_not_fall_back_to_self() {
    let mut mgr = make_mgr_with_defs();
    let (tx, _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    execute_actions(
        resolved(Action::LaunchAbility {
            ability_id: TEST_ABILITY,
            entity_tag: Some("NotSpawned".to_string()),
        }),
        1,
        42,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(
        mgr.get_entity(1).unwrap().active_effects.is_empty(),
        "an unresolvable entity_tag must skip the launch, not apply the \
         ability to the triggering player",
    );
    assert!(mgr.get_entity(101).unwrap().active_effects.is_empty());
}

/// Seed drift: a `launch_ability` row naming an ability the server has no
/// def for must no-op rather than panic or apply something arbitrary.
#[tokio::test]
async fn launch_ability_with_unknown_ability_id_is_a_noop() {
    let mut mgr = make_mgr_with_defs();
    let (tx, _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    execute_actions(
        resolved(Action::LaunchAbility {
            ability_id: 90_999,
            entity_tag: None,
        }),
        1,
        42,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(
        mgr.get_entity(1).unwrap().active_effects.is_empty(),
        "an unknown ability id must apply nothing",
    );
}

/// `Action::ApplyEffect` is the single-effect degenerate case of the same
/// helper — no ability lookup, the effect id comes straight off the action.
///
/// This arm cannot fire from seeded content today (the only `apply_effect`
/// row sits on an `effect`-scoped chain and no `effect_*` trigger is
/// dispatched), so this test is the only thing exercising it. It is
/// deliberately a direct `execute_actions` call rather than a chain replay.
#[tokio::test]
async fn apply_effect_registers_the_named_effect_on_the_source_entity() {
    let mut mgr = make_mgr_with_defs();
    let (tx, _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    execute_actions(
        resolved(Action::ApplyEffect {
            effect_id: TEST_EFFECT,
            duration_secs: None,
        }),
        1,
        42,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    let active = &mgr.get_entity(1).unwrap().active_effects;
    assert_eq!(
        active.len(),
        1,
        "ApplyEffect must register the named effect on the source entity -- \
         got {active:?}",
    );
    assert_eq!(active[0].effect_id, TEST_EFFECT);
    assert_eq!(
        active[0].invoker_id, 1,
        "ApplyEffect self-sources the invoker from the trigger entity",
    );
}

#[tokio::test]
async fn apply_effect_with_unknown_effect_id_is_a_noop() {
    let mut mgr = make_mgr_with_defs();
    let (tx, _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    execute_actions(
        resolved(Action::ApplyEffect {
            effect_id: 90_998,
            duration_secs: None,
        }),
        1,
        42,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(mgr.get_entity(1).unwrap().active_effects.is_empty());
}

/// A `pulse_count == 1` effect with no `script_name` registers nothing and
/// runs nothing — the arm reached the effect layer, the effect definition
/// simply asked for no lasting state.
///
/// This is not a hypothetical: it is the shape of effect 1634, the only
/// effect on the seeded Castle Cellblock wake-up ability. Pinning it here
/// stops a future reader from diagnosing "no active effect appeared" as a
/// bug in the executor arm when it is the seed's own definition.
#[tokio::test]
async fn single_shot_scriptless_effect_registers_no_active_instance() {
    let mut mgr = make_mgr_with_defs();
    mgr.ability_defs.insert(
        TEST_ABILITY,
        make_ability(TEST_ABILITY, vec![TEST_EFFECT_SINGLE_SHOT]),
    );
    let (tx, _rx) = mpsc::channel(64);
    let engine = ChainEngine::new();

    execute_actions(
        resolved(Action::LaunchAbility {
            ability_id: TEST_ABILITY,
            entity_tag: None,
        }),
        1,
        42,
        &tx,
        &mut mgr,
        &engine,
    )
    .await;

    assert!(
        mgr.get_entity(1).unwrap().active_effects.is_empty(),
        "a single-shot script-less effect has no duration to track -- \
         register_active_effect must decline it",
    );
}
