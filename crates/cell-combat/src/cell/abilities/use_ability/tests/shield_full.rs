//! AB-10: a player's shield press with every pool already full is refused
//! with feedback before the cooldown, instead of putting up an icon that
//! absorbs nothing.

use std::time::Instant;

use cimmeria_entity::abilities::{AbilityType, EffectDef};
use cimmeria_entity::cell_entity::{TimedEffectSpec, TimedStacking};
use cimmeria_entity::stats::ABSORB_PHYSICAL;
use cimmeria_wire::cell::chat::{serialize_on_player_communication, CHAN_FEEDBACK};

use super::super::shield_full::{shield_has_no_room, SHIELD_FULL_TEXT};
use super::*;

const PLAYER: u32 = 1;
const SHIELD: i32 = 1013;
const SHIELD_EFFECT: i32 = 4306;

/// The player knowing Personal Shield's shape: a Self ability whose one
/// effect is a 500-point physical `AbsorbShield` for 30 s.
fn scene() -> (SpaceManager, AbilityDef) {
    let mut mgr = make_mgr();
    // The real registry: the no-mechanics gate counts `AbsorbShield` only
    // when the installed registry answers it.
    crate::test_support::install_effect_scripts(&mut mgr);
    make_player(&mut mgr, PLAYER, [0.0; 3]);
    let mut def = make_ability(SHIELD, 0, 0);
    def.effect_ids = vec![SHIELD_EFFECT];
    def.target_type_id = 1;
    def.cooldown = 30.0;
    def.type_id = AbilityType::DirectDamage;
    mgr.ability_defs.insert(SHIELD, def.clone());
    mgr.effect_defs.insert(
        SHIELD_EFFECT,
        EffectDef {
            effect_id: SHIELD_EFFECT,
            ability_id: SHIELD,
            flags: 342,
            pulse_count: 1,
            pulse_duration: 30.0,
            script_name: Some("AbsorbShield".into()),
            params: [
                ("ShieldAmount".to_string(), "500".to_string()),
                ("ShieldType".to_string(), "Physical".to_string()),
            ]
            .into(),
            ..Default::default()
        },
    );
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .abilities
        .add_ability(SHIELD);
    (mgr, def)
}

/// Another shield already fills the physical pool to its max (1000).
fn fill(mgr: &mut SpaceManager) {
    mgr.apply_timed_effect(
        PLAYER,
        TimedEffectSpec {
            effect_id: 9999,
            ability_id: 9999,
            invoker_id: PLAYER,
            effect_flags: 0,
            moniker_ids: vec![],
            stats: vec![],
            absorb: vec![(ABSORB_PHYSICAL, 1000)],
            state_flags: 0,
            duration_secs: Some(30.0),
            stacking: TimedStacking::PerSource,
            invoker_identity: Default::default(),
        },
        Instant::now(),
    )
    .unwrap();
}

#[test]
fn an_empty_pool_has_room_and_a_full_one_does_not() {
    let (mut mgr, def) = scene();
    assert!(!shield_has_no_room(PLAYER, Some(&def), 0, &mgr));
    fill(&mut mgr);
    assert!(shield_has_no_room(PLAYER, Some(&def), 0, &mgr));
}

/// **Regression guard: an area shield is never refused for a full caster.**
/// Personal Shield's real row is `TCM_AERadius`, which effect routing fans out
/// to allies, so the caster's full pool says nothing about the cast. On
/// revert of the area exclusion the gate refuses it.
#[test]
fn an_area_shield_is_not_refused_for_a_full_caster() {
    let (mut mgr, def) = scene();
    mgr.effect_defs
        .get_mut(&SHIELD_EFFECT)
        .unwrap()
        .target_collection_method = cimmeria_entity::abilities::TCM_AE_RADIUS.to_string();
    fill(&mut mgr);
    assert!(!shield_has_no_room(PLAYER, Some(&def), 0, &mgr));
}

/// **Regression guard: a full shield press gets feedback, not a silent
/// cooldown.** On revert of the launch gate the press commits, the cooldown
/// is charged and nothing is said.
#[tokio::test]
async fn a_full_shield_press_is_refused_with_feedback() {
    let (mut mgr, _) = scene();
    fill(&mut mgr);
    let (tx, mut rx) = mpsc::channel(64);
    let committed = handle_use_ability(PLAYER, SHIELD, 0, &tx, &mut mgr).await;
    let msgs = drain(&mut rx);
    assert!(!committed, "the press must not commit");
    let feedback = serialize_on_player_communication("SYSTEM", 0, CHAN_FEEDBACK, SHIELD_FULL_TEXT);
    assert!(
        msgs.iter().any(|m| matches!(m,
            CellToBaseMsg::EntityMethodCall { entity_id, method_index, args }
                if *entity_id == PLAYER
                    && *method_index == method_idx::ON_PLAYER_COMMUNICATION
                    && *args == feedback)),
        "the player is told the shields are full: {msgs:#?}"
    );
    let player = mgr.get_entity(PLAYER).unwrap();
    assert!(
        !player.abilities.is_on_cooldown(SHIELD),
        "no cooldown charged"
    );
    assert_eq!(
        player.stat_buffs.entries.len(),
        1,
        "no second, empty shield"
    );
}
