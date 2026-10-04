//! AB-06 (D-AB07) regression guards: a damage script is its effect's only
//! damage path, a missed QR roll lands nothing, and `EF_DontUseQR` (16)
//! never misses.
//!
//! The fixtures are the seed's own rows: Pistol Shot (592, effect 654,
//! `RangedPhysicalDamage`, `HealthDamage` 15, `FocusDamage` 150) and Strike
//! (594, effect 656, `MeleePhysicalDamage`, 10 / 100). Each test picks an
//! `effect_seq` whose deterministic roll at the fixture's real QR is a hit
//! or a miss, so no stat is bent to force the result.

use super::super::rng::pseudo_random_seed;
use super::tests::{drain, make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;
use cimmeria_entity::abilities::{ClientEffectResult, EffectDef, EF_DONT_USE_QR, RC_HIT, SRC_NONE};
use cimmeria_entity::stats::{FOCUS, FORTITUDE};

pub(super) const PISTOL_SHOT: i32 = 592;
pub(super) const PISTOL_SHOT_EFFECT: i32 = 654;
const STRIKE: i32 = 594;
const STRIKE_EFFECT: i32 = 656;
pub(super) const NPC: u32 = 2;

/// `RangedPhysicalDamage` against an empty Focus pool: the whole 150
/// overflows, `(150*100/150)*150/300 = 50` spills, plus the base 15.
const PISTOL_SHOT_BLEED: i32 = 65;

pub(super) fn damage_effect(
    id: i32,
    script: Option<&str>,
    health: i32,
    focus: i32,
    flags: u32,
) -> EffectDef {
    EffectDef {
        effect_id: id,
        script_name: script.map(str::to_string),
        flags,
        params: [("HealthDamage", health), ("FocusDamage", focus)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    }
}

/// Player 1 vs NPC 2 with one ability of one effect; the NPC at 1000
/// Health and `focus`/1000 Focus.
pub(super) fn fixture(
    ability_id: i32,
    effect: EffectDef,
    focus: i32,
) -> (SpaceManager, AbilityDef) {
    let mut mgr = make_mgr_player_vs_npc();
    let ability = make_ability(ability_id, vec![effect.effect_id]);
    mgr.ability_defs.insert(ability_id, ability.clone());
    mgr.effect_defs.insert(effect.effect_id, effect);
    let npc = mgr.get_entity_mut(NPC).unwrap();
    for (stat, cur) in [(HEALTH, 1000), (FOCUS, focus)] {
        let s = npc.stats.get_mut(stat).unwrap();
        s.update(0, cur, 1000);
        s.clear_dirty();
    }
    (mgr, ability)
}

/// The first `effect_seq` whose roll for `attacker` → `target` with this
/// (melee) ability, at their real QR, is (`want_miss`) or is not a miss.
/// Shared with `bleed_death_tests`, whose kills need a hit since a miss
/// runs no script.
pub(super) fn seq_rolling(
    mgr: &SpaceManager,
    (attacker, target): (u32, u32),
    ability_id: i32,
    want_miss: bool,
) -> u32 {
    let qr = combat::calculate_qr(
        &mgr.get_entity(attacker).unwrap().stats,
        &mgr.get_entity(target).unwrap().stats,
        false,
    );
    (1..10_000)
        .find(|&seq| {
            let seed = pseudo_random_seed(attacker, ability_id, seq);
            (combat::calculate_result(qr, seed).result_code == RC_MISS) == want_miss
        })
        .expect("a seed in range rolls the wanted result")
}

pub(super) fn pools(mgr: &SpaceManager) -> (i32, i32) {
    let stats = &mgr.get_entity(NPC).unwrap().stats;
    (
        stats.get(HEALTH).unwrap().cur,
        stats.get(FOCUS).unwrap().cur,
    )
}

/// The `onEffectResults` args the attacker was sent.
fn effect_results_args(msgs: &[CellToBaseMsg]) -> Vec<u8> {
    msgs.iter()
        .find_map(|m| match m {
            CellToBaseMsg::EntityMethodCall {
                entity_id: 1,
                method_index,
                args,
            } if *method_index == method_idx::ON_EFFECT_RESULTS => Some(args.clone()),
            _ => None,
        })
        .expect("the attacker is sent onEffectResults")
}

/// `onEffectResults` carries the result code at byte 16 (four i32 ids).
fn result_code(args: &[u8]) -> u8 {
    args[16]
}

pub(super) async fn fire(
    mgr: &mut SpaceManager,
    ability: &AbilityDef,
    seq: u32,
) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    let id = ability.ability_id;
    apply_damage_to_target(1, NPC, id, &Some(ability.clone()), seq, false, &tx, mgr).await;
    drain(&mut rx)
}

/// **Guard (B-22).** Pistol Shot at full Focus takes exactly the script's
/// 150 Focus and no Health. On revert the NVP pipeline takes its own
/// rolled share of both pools first: Focus ends below 850 and Health below
/// 1000.
#[tokio::test]
async fn pistol_shot_at_full_focus_takes_the_script_focus_once() {
    let effect = damage_effect(PISTOL_SHOT_EFFECT, Some("RangedPhysicalDamage"), 15, 150, 0);
    let (mut mgr, ability) = fixture(PISTOL_SHOT, effect, 1000);
    let seq = seq_rolling(&mgr, (1, NPC), PISTOL_SHOT, false);

    fire(&mut mgr, &ability, seq).await;

    assert_eq!(
        pools(&mgr),
        (1000, 850),
        "one damage path: Focus 1000 - 150, Health untouched (the Focus held)"
    );
}

/// **Guard (B-22).** Strike, the melee twin: 100 Focus once, no Health.
#[tokio::test]
async fn strike_at_full_focus_takes_the_script_focus_once() {
    let effect = damage_effect(STRIKE_EFFECT, Some("MeleePhysicalDamage"), 10, 100, 0);
    let (mut mgr, ability) = fixture(STRIKE, effect, 1000);
    let seq = seq_rolling(&mgr, (1, NPC), STRIKE, false);

    fire(&mut mgr, &ability, seq).await;

    assert_eq!(pools(&mgr), (1000, 900));
}

/// **Guard (B-22), and the wire shape.** Pistol Shot against an empty
/// Focus pool bleeds 65 Health once, and `onEffectResults` reports that
/// one HEALTH change byte-exactly. On revert Health ends below 935 and the
/// list carries the NVP pipeline's own entry instead.
#[tokio::test]
async fn pistol_shot_at_empty_focus_bleeds_once_and_reports_the_bleed() {
    let effect = damage_effect(PISTOL_SHOT_EFFECT, Some("RangedPhysicalDamage"), 15, 150, 0);
    let (mut mgr, ability) = fixture(PISTOL_SHOT, effect, 0);
    let seq = seq_rolling(&mgr, (1, NPC), PISTOL_SHOT, false);

    let msgs = fire(&mut mgr, &ability, seq).await;

    assert_eq!(pools(&mgr), (1000 - PISTOL_SHOT_BLEED, 0));
    let args = effect_results_args(&msgs);
    let expected = serialize_effect_results(
        1,
        PISTOL_SHOT,
        seq as i32,
        NPC as i32,
        result_code(&args),
        &[ClientEffectResult {
            stat_id: HEALTH as i8,
            delta: -PISTOL_SHOT_BLEED,
            damage_code: DT_PHYSICAL,
            stat_result_code: SRC_NONE,
        }],
    );
    assert_eq!(args, expected, "one HEALTH entry, the script's bleed");
}

/// **Guard (B-23).** A missed Pistol Shot at an empty Focus pool runs no
/// script: both pools untouched. On revert the script ignores the miss
/// and bleeds 65 Health.
#[tokio::test]
async fn missed_pistol_shot_leaves_both_pools_untouched() {
    let effect = damage_effect(PISTOL_SHOT_EFFECT, Some("RangedPhysicalDamage"), 15, 150, 0);
    let (mut mgr, ability) = fixture(PISTOL_SHOT, effect, 0);
    let seq = seq_rolling(&mgr, (1, NPC), PISTOL_SHOT, true);

    let msgs = fire(&mut mgr, &ability, seq).await;

    assert_eq!(result_code(&effect_results_args(&msgs)), RC_MISS);
    assert_eq!(pools(&mgr), (1000, 0), "a miss deals nothing");
}

/// **Guard.** A missed NVP-only hit deals nothing either (the pipeline
/// used to chip up to 14 % of base on a miss), and a missed DoT registers
/// no pulses. On revert Health drops and the DoT instance is registered.
#[tokio::test]
async fn missed_nvp_hit_deals_nothing_and_registers_no_pulses() {
    let mut effect = damage_effect(7001, None, 500, 0, 0);
    effect.pulse_count = 8;
    effect.pulse_duration = 1.0;
    let (mut mgr, ability) = fixture(7000, effect, 1000);
    let seq = seq_rolling(&mgr, (1, NPC), 7000, true);

    fire(&mut mgr, &ability, seq).await;

    assert_eq!(pools(&mgr), (1000, 1000));
    assert!(
        mgr.get_entity(NPC).unwrap().active_effects.is_empty(),
        "a missed DoT must not tick"
    );
}

/// **Guard (B-24).** An `EF_DontUseQR` (16) effect never misses: on the
/// same seed that misses without the flag, it takes no roll, lands as
/// `RC_Hit` and bleeds. On revert (`EF_DONT_USE_QR = 32`, unread) the
/// seed rolls a miss and Health stays at 1000.
#[tokio::test]
async fn dont_use_qr_effect_never_misses() {
    let plain = damage_effect(PISTOL_SHOT_EFFECT, Some("RangedPhysicalDamage"), 15, 150, 0);
    let (probe, _) = fixture(PISTOL_SHOT, plain, 0);
    let seq = seq_rolling(&probe, (1, NPC), PISTOL_SHOT, true);

    let flagged = damage_effect(
        PISTOL_SHOT_EFFECT,
        Some("RangedPhysicalDamage"),
        15,
        150,
        EF_DONT_USE_QR,
    );
    let (mut mgr, ability) = fixture(PISTOL_SHOT, flagged, 0);
    let msgs = fire(&mut mgr, &ability, seq).await;

    assert_eq!(
        i64::from(EF_DONT_USE_QR),
        crate::cell::abilities::enumerations_xml::token("EEffectFlag", "EF_DontUseQR"),
        "the client's EEffectFlag bit"
    );
    assert_eq!(result_code(&effect_results_args(&msgs)), RC_HIT);
    assert_eq!(pools(&mgr), (1000 - PISTOL_SHOT_BLEED, 0));
}

/// An unrolled NVP hit deals the authored base: `qr_rand` 0.5 × the 2.0
/// multiplier × (1 + QR 0) = 1.0.
#[tokio::test]
async fn dont_use_qr_nvp_hit_deals_the_authored_base() {
    let effect = damage_effect(7101, None, 100, 0, EF_DONT_USE_QR);
    let (mut mgr, ability) = fixture(7100, effect, 1000);
    // No Fortitude resist, so the base reaches Health whole.
    let fortitude = mgr
        .get_entity_mut(NPC)
        .unwrap()
        .stats
        .get_mut(FORTITUDE)
        .unwrap();
    fortitude.update(0, 0, fortitude.max);

    fire(&mut mgr, &ability, 1).await;

    assert_eq!(pools(&mgr).0, 900);
}

/// **Guard (splash keeps the damage script, at the splash scale).** An
/// explosive round's splash target (`HitKind::Splash`, fraction 0.5) takes
/// Pistol Shot's script at half its NVPs: `FocusDamage` 75, `HealthDamage`
/// round(7.5) = 8. Against 50 Focus: 50 absorbed, overflow 25, spill
/// `(25*100/75)*75/300 = 8`, plus 8 = 16 Health. If splash cleared the
/// damage scripts both pools stay `(1000, 50)`; if the scale were not
/// applied the full 150/15 would leave `(952, 0)`.
#[tokio::test]
async fn splash_runs_the_damage_script_at_the_splash_scale() {
    let effect = damage_effect(PISTOL_SHOT_EFFECT, Some("RangedPhysicalDamage"), 15, 150, 0);
    let (mut mgr, ability) = fixture(PISTOL_SHOT, effect, 50);
    let seq = seq_rolling(&mgr, (1, NPC), PISTOL_SHOT, false);
    let (tx, _rx) = mpsc::channel(256);

    apply_hit(
        1,
        NPC,
        PISTOL_SHOT,
        &Some(ability),
        seq,
        false,
        HitKind::Splash { fraction: 0.5 },
        &tx,
        &mut mgr,
    )
    .await;

    assert_eq!(pools(&mgr), (1000 - 16, 0));
}
