//! AM-11d: a beneficial row's on-hit effect never runs inside
//! `apply_damage_to_target`. The single-target fire sends a support shot
//! down `use_ability::support_shot`; any other path that reaches the damage
//! pipeline with support darts loaded (an AoE or cone secondary) is aiming
//! at a hostile, and a Stim dart must not heal it.
//!
//! Like `ammo_tests`, these turn `ammo.finite_special` on for the process
//! and never off.

use super::tests::{make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::spawner::{AmmoCatalog, AmmoModifier};
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::ammo_type::{DART_DEFAULT, DART_STIM};
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::FOCUS;

const ABILITY: i32 = 7;
const STIM_EFFECT: i32 = 9160;
const FOCUS_MAX: i32 = 1_000;

/// The Focus hostile NPC 2 has after one shot from player 1 with Stim darts
/// loaded, on the first effect sequence whose roll is not a miss.
async fn npc_focus_after_hit(beneficial: bool) -> i32 {
    cimmeria_entity::ammo_feature::set_finite_special(true);
    let mut mgr = make_mgr_player_vs_npc();
    let mut ability = make_ability(ABILITY, vec![]);
    ability.required_ammo = 1;
    ability.is_ranged = true;
    mgr.ability_defs.insert(ABILITY, ability.clone());
    mgr.effect_defs.insert(
        STIM_EFFECT,
        EffectDef {
            effect_id: STIM_EFFECT,
            ability_id: 992,
            script_name: Some("HealFocus".to_string()),
            params: std::collections::HashMap::from([(
                "HealPercentage".to_string(),
                "10".to_string(),
            )]),
            ..Default::default()
        },
    );
    mgr.ammo_catalog = AmmoCatalog::from_rows(
        [AmmoModifier {
            ammo_type: DART_STIM,
            damage_mult: 0.0001,
            penetration_mult: 1.0,
            damage_type: None,
            on_hit_effect_id: Some(STIM_EFFECT),
            toggle_ability_id: 992,
            beneficial,
        }],
        [(DART_STIM, 9010)],
    );
    let p = mgr.get_entity_mut(1).unwrap();
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 1,
            item_id: 1520,
            clip_size: 100,
            default_ammo_type: DART_DEFAULT,
            current_ammo: 100,
            cur_ammo_type: DART_STIM,
        },
    );
    mgr.get_entity_mut(2)
        .unwrap()
        .stats
        .get_mut(FOCUS)
        .unwrap()
        .update(0, 0, FOCUS_MAX);

    let qr = combat::calculate_qr(
        &mgr.get_entity(1).unwrap().stats,
        &mgr.get_entity(2).unwrap().stats,
        true,
    );
    let effect_seq = (0..64)
        .find(|&seq| {
            let seed = pseudo_random_seed(1, ABILITY, seq);
            combat::calculate_result(qr, seed).result_code != RC_MISS
        })
        .expect("some roll hits");

    let (tx, _rx) = mpsc::channel(256);
    apply_damage_to_target(
        1,
        2,
        ABILITY,
        &Some(ability),
        effect_seq,
        false,
        &tx,
        &mut mgr,
    )
    .await;
    mgr.get_entity(2).unwrap().stats.get(FOCUS).unwrap().cur
}

/// A beneficial row reaching the damage pipeline heals nobody; the same row
/// with the flag off runs its on-hit heal, so the zero is the flag's doing.
#[tokio::test]
async fn a_beneficial_row_never_heals_through_the_damage_pipeline() {
    assert_eq!(
        npc_focus_after_hit(true).await,
        0,
        "the hostile is not healed"
    );
    assert_eq!(
        npc_focus_after_hit(false).await,
        FOCUS_MAX / 10,
        "control: a non-beneficial row runs its on-hit effect"
    );
}
