//! Special-ammo damage through `apply_damage_to_target` (ammo campaign AM-04,
//! D-AM07): the modifier row of the loaded ammo type scales the shot, its
//! penetration divides the armour, and its on-hit effect runs.
//!
//! These tests turn `ammo.finite_special` on for the process and never off:
//! nextest runs each test in its own process, and under `cargo test` every
//! other test in this crate starts with an empty `ammo_catalog`, which fires
//! unmodified whatever the flag says. The flag-off case is the unit test
//! `ammo_damage::tests::flag_off_fires_unmodified` in `cimmeria-cell-world`.

use super::tests::{make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::spawner::{AmmoCatalog, AmmoModifier};
use crate::test_support::LogCapture;
use cimmeria_entity::abilities::{EffectDef, RC_HIT};
use cimmeria_entity::ammo_type::{BULLET_ARMOR_PIERCING, BULLET_DEFAULT, BULLET_HOLLOW_POINT};
use cimmeria_entity::cell_entity::BandolierItem;
use cimmeria_entity::stats::{MITIGATION, PHYSICAL_AF};
use tracing::Level;

/// The reconstructed rows `ammo_modifiers_hp_ap.sql` seeds; the live-DB
/// guard `live_db_hp_ap_seed_rows` in `cimmeria-cell-world` pins the seed to
/// these numbers.
const HP_DAMAGE: f32 = 1.25;
const HP_PENETRATION: f32 = 0.5;
const AP_DAMAGE: f32 = 0.9;
const AP_PENETRATION: f32 = 2.0;

const ABILITY: i32 = 7;
const EFFECT: i32 = 100;
const ON_HIT_EFFECT: i32 = 101;
const NPC_HEALTH: i32 = 100_000;

fn row(ammo_type: i32, damage_mult: f32, penetration_mult: f32, toggle: i32) -> AmmoModifier {
    AmmoModifier {
        ammo_type,
        damage_mult,
        penetration_mult,
        damage_type: Some(i32::from(DT_PHYSICAL)),
        on_hit_effect_id: None,
        toggle_ability_id: toggle,
        beneficial: false,
    }
}

fn seeded_rows() -> Vec<AmmoModifier> {
    vec![
        row(BULLET_HOLLOW_POINT, HP_DAMAGE, HP_PENETRATION, 715),
        row(BULLET_ARMOR_PIERCING, AP_DAMAGE, AP_PENETRATION, 719),
    ]
}

fn shot_effect(id: i32, health_damage: i32, script: Option<&str>) -> EffectDef {
    let mut params = std::collections::HashMap::new();
    params.insert("HealthDamage".to_string(), health_damage.to_string());
    EffectDef {
        effect_id: id,
        script_name: script.map(str::to_string),
        params,
        ..Default::default()
    }
}

/// Player 1 fires ability 7 (a one-round weapon shot of 1000 HealthDamage)
/// from a pistol loaded with `ammo_type` at NPC 2, whose armour is
/// `(armour, mitigation)`. Returns the HEALTH the NPC lost and the result
/// code.
async fn fire(
    ammo_type: i32,
    rows: Vec<AmmoModifier>,
    armour: (i32, i32),
    effect_seq: u32,
    setup: impl FnOnce(&mut SpaceManager),
) -> (i32, u8) {
    cimmeria_entity::ammo_feature::set_finite_special(true);
    let mut mgr = make_mgr_player_vs_npc();
    let mut ability = make_ability(ABILITY, vec![EFFECT]);
    ability.required_ammo = 1;
    ability.is_ranged = true;
    mgr.ability_defs.insert(ABILITY, ability.clone());
    mgr.effect_defs
        .insert(EFFECT, shot_effect(EFFECT, 1000, None));
    mgr.ammo_catalog = AmmoCatalog::from_rows(rows, [(BULLET_HOLLOW_POINT, 9001)]);
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
    npc.stats
        .get_mut(HEALTH)
        .unwrap()
        .update(0, NPC_HEALTH, NPC_HEALTH);
    npc.stats
        .get_mut(PHYSICAL_AF)
        .unwrap()
        .update(0, armour.0, 50_000);
    // MITIGATION is capped at 0 in the default stat list; raise the cap.
    npc.stats
        .get_mut(MITIGATION)
        .unwrap()
        .update(0, armour.1, 100);
    setup(&mut mgr);

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
    let hp = mgr.get_entity(2).unwrap().stats.get(HEALTH).unwrap().cur;
    (NPC_HEALTH - hp, code)
}

/// An `effect_seq` whose roll is a plain hit, so every comparison below is
/// between non-zero, non-critical damage of the same roll.
async fn hit_seq() -> u32 {
    for seq in 1..200 {
        let (dmg, code) = fire(BULLET_DEFAULT, vec![], (0, 0), seq, |_| {}).await;
        if code == RC_HIT && dmg > 0 {
            return seq;
        }
    }
    panic!("no plain hit in 200 rolls");
}

fn near(got: i32, want: f64) -> bool {
    (f64::from(got) - want).abs() <= 1.0
}

/// Hollow Point hits harder and Armor Piercing softer than default ammo,
/// by the seeded factor, against the same unarmoured target on the same
/// roll. Fails if the modifier is not threaded into the pipeline.
#[tokio::test]
async fn hollow_point_raises_and_armor_piercing_lowers_damage_by_the_seeded_factor() {
    let seq = hit_seq().await;
    let (base, _) = fire(BULLET_DEFAULT, seeded_rows(), (0, 0), seq, |_| {}).await;
    let (hp, _) = fire(BULLET_HOLLOW_POINT, seeded_rows(), (0, 0), seq, |_| {}).await;
    let (ap, _) = fire(BULLET_ARMOR_PIERCING, seeded_rows(), (0, 0), seq, |_| {}).await;
    assert!(base > 0);
    assert!(
        hp > base && near(hp, f64::from(base) * f64::from(HP_DAMAGE)),
        "HP {hp} vs default {base}"
    );
    assert!(
        ap < base && near(ap, f64::from(base) * f64::from(AP_DAMAGE)),
        "AP {ap} vs default {base}"
    );
}

/// Default ammo with the catalog loaded fires exactly as with no catalog at
/// all: the framework adds nothing to an unmodified shot.
#[tokio::test]
async fn default_ammo_is_unchanged() {
    let seq = hit_seq().await;
    let armour = (40, 50);
    let (bare, _) = fire(BULLET_DEFAULT, vec![], armour, seq, |_| {}).await;
    let (with_catalog, _) = fire(BULLET_DEFAULT, seeded_rows(), armour, seq, |_| {}).await;
    assert_eq!(bare, with_catalog);
}

/// Penetration divides the armour mitigation: against armour, a
/// penetration-only row (damage 1.0) out-damages default ammo, a
/// low-penetration row under-damages it, and against no armour both equal
/// default. Fails if `penetration_mult` is dropped on the way to the
/// pipeline, or applied to the damage instead of the armour.
#[tokio::test]
async fn penetration_scales_the_armour_mitigation() {
    let seq = hit_seq().await;
    let rows = vec![
        row(BULLET_ARMOR_PIERCING, 1.0, 2.0, 719),
        row(BULLET_HOLLOW_POINT, 1.0, 0.5, 715),
    ];
    let armour = (40, 50);
    let (base, _) = fire(BULLET_DEFAULT, rows.clone(), armour, seq, |_| {}).await;
    let (pierce, _) = fire(BULLET_ARMOR_PIERCING, rows.clone(), armour, seq, |_| {}).await;
    let (blunt, _) = fire(BULLET_HOLLOW_POINT, rows.clone(), armour, seq, |_| {}).await;
    // 40 armour at 50 mitigation stops 20; 2.0 penetration stops 10,
    // 0.5 stops 40.
    assert_eq!(pierce - base, 10, "pierce {pierce} base {base}");
    assert_eq!(base - blunt, 20, "blunt {blunt} base {base}");

    let (bare_base, _) = fire(BULLET_DEFAULT, rows.clone(), (0, 0), seq, |_| {}).await;
    let (bare_pierce, _) = fire(BULLET_ARMOR_PIERCING, rows, (0, 0), seq, |_| {}).await;
    assert_eq!(bare_base, bare_pierce);
}

/// A row's `on_hit_effect_id` runs on the target when the shot hits: here a
/// `MeleeDamage` effect of 7, so the target loses exactly 7 more. Fails if
/// the on-hit effect is not dispatched.
#[tokio::test]
async fn on_hit_effect_fires_when_set() {
    let seq = hit_seq().await;
    let plain = vec![row(BULLET_HOLLOW_POINT, 1.0, 1.0, 715)];
    let mut with_hit = plain.clone();
    with_hit[0].on_hit_effect_id = Some(ON_HIT_EFFECT);
    let add_effect = |m: &mut SpaceManager| {
        m.effect_defs.insert(
            ON_HIT_EFFECT,
            shot_effect(ON_HIT_EFFECT, 7, Some("MeleeDamage")),
        );
    };
    let (without, _) = fire(BULLET_HOLLOW_POINT, plain, (0, 0), seq, add_effect).await;
    let (with, _) = fire(BULLET_HOLLOW_POINT, with_hit, (0, 0), seq, add_effect).await;
    assert_eq!(with - without, 7);
}

/// A modified shot logs `ammo_damage_applied` on target `ammo` with the
/// correlators and the row's numbers; an unmodified one logs nothing.
#[tokio::test]
async fn modified_shot_logs_ammo_damage_applied() {
    let seq = hit_seq().await;
    let logs = LogCapture::install();
    let _ = fire(BULLET_DEFAULT, seeded_rows(), (0, 0), seq, |_| {}).await;
    let applied = |logs: &crate::test_support::LogCaptureGuard| {
        logs.all()
            .into_iter()
            .filter(|c| c.target == "ammo" && c.has_field("event", "ammo_damage_applied"))
            .collect::<Vec<_>>()
    };
    assert!(applied(&logs).is_empty(), "default ammo must not log");

    let _ = fire(BULLET_HOLLOW_POINT, seeded_rows(), (0, 0), seq, |_| {}).await;
    let rows = applied(&logs);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let r = &rows[0];
    assert_eq!(r.level, Level::DEBUG);
    for (k, v) in [
        ("player_id", "100"),
        ("entity_id", "1"),
        ("item_id", "9001"),
        ("ammo_type", "3"),
        ("target_entity_id", "2"),
        ("damage_mult", "1.25"),
        ("penetration_mult", "0.5"),
        ("damage_type", "3"),
        ("toggle_ability_id", "715"),
    ] {
        assert!(r.has_field(k, v), "field {k}={v} missing: {r:?}");
    }
}
