//! EMP rounds through `apply_damage_to_target` (ammo campaign AM-09): a
//! player's shot with Bullet_EMP loaded runs on-hit effect 9120
//! (`EmpDisrupt`), which drains a living target's Focus and takes Health
//! from a mechanical one.
//!
//! Like `ammo_tests`, these turn `ammo.finite_special` on for the process
//! and never off.

use super::tests::{make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::spawner::{AmmoCatalog, AmmoModifier};
use cimmeria_entity::abilities::{EffectDef, RC_HIT};
use cimmeria_entity::ammo_type::{BULLET_DEFAULT, BULLET_EMP};
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::FOCUS;

/// The seeded row and effect (`ammo_modifiers_emp.sql`); the live-DB guard
/// `ammo_emp::tests::live_db_emp_seed_rows` in `cimmeria-cell-world` pins
/// the seed to these numbers.
const EMP_DAMAGE: f32 = 1.1;
const EMP_PENETRATION: f32 = 0.75;
const EMP_EFFECT: i32 = 9120;
const FOCUS_DAMAGE: i32 = 10;
const MECH_DAMAGE: i32 = 5;

const ABILITY: i32 = 7;
const EFFECT: i32 = 100;
const NPC_HEALTH: i32 = 100_000;
const NPC_FOCUS: i32 = 1_000;

fn emp_row(on_hit: Option<i32>) -> AmmoModifier {
    AmmoModifier {
        ammo_type: BULLET_EMP,
        damage_mult: EMP_DAMAGE,
        penetration_mult: EMP_PENETRATION,
        damage_type: Some(i32::from(DT_PHYSICAL)),
        on_hit_effect_id: on_hit,
        toggle_ability_id: 1445,
    }
}

fn emp_effect() -> EffectDef {
    let mut params = std::collections::HashMap::new();
    params.insert("FocusDamage".to_string(), FOCUS_DAMAGE.to_string());
    params.insert(
        "MechanicalHealthDamage".to_string(),
        MECH_DAMAGE.to_string(),
    );
    EffectDef {
        effect_id: EMP_EFFECT,
        ability_id: 1445,
        script_name: Some("EmpDisrupt".to_string()),
        params,
        ..Default::default()
    }
}

/// Player 1 fires ability 7 (a one-round weapon shot of 1000 HealthDamage,
/// no FocusDamage) loaded with `ammo_type` at NPC 2 wearing `body_set`.
/// Returns `(health lost, focus lost, result code)`.
async fn fire(
    ammo_type: i32,
    row: AmmoModifier,
    body_set: &str,
    effect_seq: u32,
) -> (i32, i32, u8) {
    cimmeria_entity::ammo_feature::set_finite_special(true);
    let mut mgr = make_mgr_player_vs_npc();
    let mut ability = make_ability(ABILITY, vec![EFFECT]);
    ability.required_ammo = 1;
    ability.is_ranged = true;
    mgr.ability_defs.insert(ABILITY, ability.clone());
    let mut shot_params = std::collections::HashMap::new();
    shot_params.insert("HealthDamage".to_string(), "1000".to_string());
    mgr.effect_defs.insert(
        EFFECT,
        EffectDef {
            effect_id: EFFECT,
            params: shot_params,
            ..Default::default()
        },
    );
    mgr.effect_defs.insert(EMP_EFFECT, emp_effect());
    mgr.ammo_catalog = AmmoCatalog::from_rows([row], [(BULLET_EMP, 9003)]);
    let p = mgr.get_entity_mut(1).unwrap();
    p.active_bandolier_slot = 0;
    p.bandolier_items.insert(
        0,
        BandolierItem {
            instance_id: 1,
            item_id: 3241,
            clip_size: 15,
            default_ammo_type: BULLET_DEFAULT,
            current_ammo: 15,
            cur_ammo_type: ammo_type,
        },
    );
    let npc = mgr.get_entity_mut(2).unwrap();
    npc.body_set = Some(body_set.to_string());
    npc.stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, NPC_HEALTH, NPC_HEALTH);
    npc.stats
        .get_mut(FOCUS)
        .unwrap()
        .update(0, NPC_FOCUS, NPC_FOCUS);

    let seed = pseudo_random_seed(1, ABILITY, effect_seq);
    let qr = combat::calculate_qr(
        &mgr.get_entity(1).unwrap().stats,
        &mgr.get_entity(2).unwrap().stats,
        true,
    );
    let code = combat::calculate_result(qr, seed).result_code;

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
    let npc = mgr.get_entity(2).unwrap();
    (
        NPC_HEALTH - npc.stats.get(HEALTH).unwrap().cur,
        NPC_FOCUS - npc.stats.get(FOCUS).unwrap().cur,
        code,
    )
}

const JAFFA: &str = "BS_JaffaMale.BS_JaffaMale";
const DRONE: &str = "MOB_CA_DroneTank.BS_MOB_DroneFlyer";

/// An `effect_seq` whose roll is a plain hit.
async fn hit_seq() -> u32 {
    for seq in 1..200 {
        let (dmg, _, code) = fire(BULLET_DEFAULT, emp_row(None), JAFFA, seq).await;
        if code == RC_HIT && dmg > 0 {
            return seq;
        }
    }
    panic!("no plain hit in 200 rolls");
}

/// A living target hit by an EMP round loses the seeded Focus on top of
/// the shot, and no extra Health. Fails if the row's on-hit effect is not
/// wired to `EmpDisrupt` (registry arm) or not dispatched on the shot.
#[tokio::test]
async fn emp_round_drains_a_living_targets_focus() {
    let seq = hit_seq().await;
    let (h_plain, f_plain, _) = fire(BULLET_EMP, emp_row(None), JAFFA, seq).await;
    let (h_emp, f_emp, _) = fire(BULLET_EMP, emp_row(Some(EMP_EFFECT)), JAFFA, seq).await;
    assert_eq!(f_plain, 0, "the shot itself has no FocusDamage");
    assert_eq!(f_emp, FOCUS_DAMAGE);
    assert_eq!(h_emp, h_plain);
}

/// A mechanical target hit by an EMP round loses the seeded extra Health
/// and no Focus.
#[tokio::test]
async fn emp_round_damages_a_mechanical_targets_health() {
    let seq = hit_seq().await;
    let (h_plain, _, _) = fire(BULLET_EMP, emp_row(None), DRONE, seq).await;
    let (h_emp, f_emp, _) = fire(BULLET_EMP, emp_row(Some(EMP_EFFECT)), DRONE, seq).await;
    assert_eq!(f_emp, 0);
    assert_eq!(h_emp - h_plain, MECH_DAMAGE);
}

/// The EMP row scales the shot by its seeded damage factor against the same
/// roll with default ammo.
#[tokio::test]
async fn emp_round_scales_the_shot_by_the_seeded_factor() {
    let seq = hit_seq().await;
    let (base, _, _) = fire(BULLET_DEFAULT, emp_row(None), JAFFA, seq).await;
    let (emp, _, _) = fire(BULLET_EMP, emp_row(None), JAFFA, seq).await;
    let want = f64::from(base) * f64::from(EMP_DAMAGE);
    assert!(
        emp > base && (f64::from(emp) - want).abs() <= 1.0,
        "EMP {emp} vs default {base}"
    );
}
