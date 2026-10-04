//! AB-03 (B-21) regression guards: each `TCM_Single` damage effect resolves
//! on its own, an `EF_DontUseQR` effect resolves at its base whatever the
//! hit rolled (the AB-06 review carry-over), and cone/radius effects stay
//! with their fan-outs when a direct single-target effect is the target's
//! damage.
//!
//! The target is the NPC of `make_mgr_player_vs_npc` at 1000 Health and
//! 1000 Focus with Fortitude and Intelligence 0, so an unrolled NVP hit takes exactly its
//! base (`base × 0.5 × 2 × (1 + 0)`, no resist, no armour).

use super::single_damage_path_tests::seq_rolling;
use super::tests::{drain, make_ability, make_mgr_player_vs_npc};
use super::*;
use crate::cell::space_manager::SpaceManager;
use crate::mercury::method_idx;
use cimmeria_entity::abilities::{
    ClientEffectResult, EffectDef, EF_DONT_USE_QR, RC_HIT, SRC_NONE, TCM_AE_CONE, TCM_AE_RADIUS,
};
use cimmeria_entity::stats::{FOCUS, FORTITUDE, INTELLIGENCE};

const NPC: u32 = 2;

fn nvp_effect(id: i32, health: i32, focus: i32, flags: u32) -> EffectDef {
    EffectDef {
        effect_id: id,
        flags,
        params: [("HealthDamage", health), ("FocusDamage", focus)]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..Default::default()
    }
}

fn with_tcm(mut e: EffectDef, tcm: &str) -> EffectDef {
    e.target_collection_method = tcm.to_string();
    e
}

fn dot(mut e: EffectDef) -> EffectDef {
    e.pulse_count = 8;
    e.pulse_duration = 1.0;
    e
}

/// Player 1 vs NPC 2 with one ability of `effects`; the NPC at 1000/1000
/// and no Fortitude resist.
fn fixture(ability_id: i32, effects: Vec<EffectDef>) -> (SpaceManager, AbilityDef) {
    let mut mgr = make_mgr_player_vs_npc();
    let ability = make_ability(ability_id, effects.iter().map(|e| e.effect_id).collect());
    mgr.ability_defs.insert(ability_id, ability.clone());
    for e in effects {
        mgr.effect_defs.insert(e.effect_id, e);
    }
    let npc = mgr.get_entity_mut(NPC).unwrap();
    for (stat, cur) in [(HEALTH, 1000), (FOCUS, 1000)] {
        let s = npc.stats.get_mut(stat).unwrap();
        s.update(0, cur, 1000);
        s.clear_dirty();
    }
    // No Health or Focus resist, so an unrolled base lands whole.
    for resist in [FORTITUDE, INTELLIGENCE] {
        let s = npc.stats.get_mut(resist).unwrap();
        s.update(0, 0, s.max);
    }
    (mgr, ability)
}

fn pools(mgr: &SpaceManager) -> (i32, i32) {
    let stats = &mgr.get_entity(NPC).unwrap().stats;
    (
        stats.get(HEALTH).unwrap().cur,
        stats.get(FOCUS).unwrap().cur,
    )
}

async fn fire(mgr: &mut SpaceManager, ability: &AbilityDef, seq: u32) -> Vec<CellToBaseMsg> {
    let (tx, mut rx) = mpsc::channel(256);
    let id = ability.ability_id;
    apply_damage_to_target(1, NPC, id, &Some(ability.clone()), seq, false, &tx, mgr).await;
    drain(&mut rx)
}

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

/// **Guard (B-21), and the wire shape.** Two unrolled single-target
/// effects (100 H / 200 F and 50 H / 0 F) both land: Health 850, Focus
/// 800, and `onEffectResults` carries one HEALTH entry per effect,
/// byte-exact. On revert only the last positive value per pool counts:
/// `(950, 800)` and one HEALTH entry.
#[tokio::test]
async fn two_single_target_damage_effects_both_apply() {
    let (mut mgr, ability) = fixture(
        7300,
        vec![
            nvp_effect(7301, 100, 200, EF_DONT_USE_QR),
            nvp_effect(7302, 50, 0, EF_DONT_USE_QR),
        ],
    );

    let msgs = fire(&mut mgr, &ability, 1).await;

    assert_eq!(pools(&mgr), (850, 800), "both effects land");
    let entry = |delta| ClientEffectResult {
        stat_id: HEALTH as i8,
        delta,
        damage_code: DT_PHYSICAL,
        stat_result_code: SRC_NONE,
    };
    let expected =
        serialize_effect_results(1, 7300, 1, NPC as i32, RC_HIT, &[entry(-100), entry(-50)]);
    assert_eq!(
        effect_results_args(&msgs),
        expected,
        "one HEALTH entry per effect"
    );
}

/// **Guard (B-21), on the hit's roll.** Two rolled single-target effects
/// on one hit each take the same roll: the combined hit deals exactly
/// what the two effects deal fired alone on the same seed. On revert the
/// combined hit deals only the second effect's share.
#[tokio::test]
async fn rolled_single_target_effects_each_take_the_hit_roll() {
    let first = nvp_effect(7311, 300, 0, 0);
    let second = nvp_effect(7312, 120, 0, 0);
    let (probe, _) = fixture(7310, vec![first.clone()]);
    let seq = seq_rolling(&probe, (1, NPC), 7310, false);

    let mut alone = 0;
    for effect in [first.clone(), second.clone()] {
        let (mut mgr, ability) = fixture(7310, vec![effect]);
        fire(&mut mgr, &ability, seq).await;
        alone += 1000 - pools(&mgr).0;
    }
    let (mut mgr, ability) = fixture(7310, vec![first, second]);
    fire(&mut mgr, &ability, seq).await;

    assert!(alone > 0, "the seed rolls a hit that deals damage");
    assert_eq!(1000 - pools(&mgr).0, alone);
}

/// **Guard (AB-06 review carry-over).** A mixed ability on a seed that
/// misses: the `EF_DontUseQR` effect still deals its whole base (100) and
/// the rolled effect deals nothing. On revert the flagged effect's
/// damage takes the missed roll's `qr_rand` (below 0.07), so Health ends
/// above 980.
#[tokio::test]
async fn dont_use_qr_effect_in_a_missed_hit_deals_its_base() {
    let flagged = nvp_effect(7321, 100, 0, EF_DONT_USE_QR);
    let rolled = nvp_effect(7322, 300, 0, 0);
    let (mut mgr, ability) = fixture(7320, vec![flagged, rolled]);
    let seq = seq_rolling(&mgr, (1, NPC), 7320, true);

    let msgs = fire(&mut mgr, &ability, seq).await;

    assert_eq!(
        effect_results_args(&msgs)[16],
        RC_MISS,
        "the hit rolled a miss"
    );
    assert_eq!(pools(&mgr).0, 900, "flagged base only");
}

/// **Guard (AB-06 review carry-over).** On a seed that hits, the flagged
/// effect deals exactly its base beside a rolled effect that carries no
/// damage (a snare row): the hit is rolled, the flagged damage is not. On
/// revert the base is scaled by the rolled `qr_rand` × 2 × (1 + QR).
#[tokio::test]
async fn dont_use_qr_effect_in_a_rolled_hit_deals_its_base() {
    let flagged = nvp_effect(7331, 100, 0, EF_DONT_USE_QR);
    let snare = nvp_effect(7332, 0, 0, 0);
    let (mut mgr, ability) = fixture(7330, vec![flagged, snare]);
    let qr = combat::calculate_qr(
        &mgr.get_entity(1).unwrap().stats,
        &mgr.get_entity(NPC).unwrap().stats,
        false,
    );
    // A hit whose roll is not the midpoint, so a rolled base would differ.
    let seq = (1..10_000)
        .find(|&s| {
            let r = combat::calculate_result(qr, pseudo_random_seed(1, 7330, s));
            r.result_code != RC_MISS && (r.qr_rand * 2.0 * (1.0 + qr) - 1.0).abs() > 0.05
        })
        .unwrap();

    fire(&mut mgr, &ability, seq).await;

    assert_eq!(pools(&mgr).0, 900);
}

/// **Guard (cone and radius stay with their fan-outs).** A direct
/// single-target effect is the primary's damage; the cone half is the
/// fan-out's. Health 900, not 600 (both) and not 700 (the cone alone, the
/// last positive value on revert).
#[tokio::test]
async fn cone_damage_stays_with_the_fan_out_beside_a_direct_single_hit() {
    let single = nvp_effect(7341, 100, 0, EF_DONT_USE_QR);
    let cone = with_tcm(nvp_effect(7342, 300, 0, EF_DONT_USE_QR), TCM_AE_CONE);
    let (mut mgr, ability) = fixture(7340, vec![single, cone]);

    fire(&mut mgr, &ability, 1).await;

    assert_eq!(pools(&mgr).0, 900);
}

/// A pure cone ability still hurts its primary: the cone fan-out skips the
/// primary, so the hit itself carries the cone's damage.
#[tokio::test]
async fn a_cone_only_ability_hurts_its_primary() {
    let cone = with_tcm(nvp_effect(7351, 300, 0, EF_DONT_USE_QR), TCM_AE_CONE);
    let (mut mgr, ability) = fixture(7350, vec![cone]);

    fire(&mut mgr, &ability, 1).await;

    assert_eq!(pools(&mgr).0, 700);
}

/// **Guard (B-21, Devastating Blast shape).** A radius blast with a
/// single-target DoT: the DoT is not a direct hit, so the blast (500)
/// and the DoT's first pulse (150) both land, and the DoT registers. On
/// revert only the DoT (the last positive value) lands: Health 850.
#[tokio::test]
async fn a_radius_blast_and_its_dot_both_land() {
    let blast = with_tcm(nvp_effect(7361, 500, 0, EF_DONT_USE_QR), TCM_AE_RADIUS);
    let tick = dot(nvp_effect(7362, 150, 0, EF_DONT_USE_QR));
    let (mut mgr, ability) = fixture(7360, vec![blast, tick]);

    fire(&mut mgr, &ability, 1).await;

    assert_eq!(pools(&mgr).0, 350);
    assert_eq!(mgr.get_entity(NPC).unwrap().active_effects.len(), 1);
}

/// **Guard (B-21, Point Blank Shot shape).** A direct hit (200 H) and a
/// DoT (30 H per tick) on one ability: both first applications land, and
/// the DoT registers its remaining pulses. On revert only the DoT's 30
/// lands.
#[tokio::test]
async fn a_direct_hit_and_its_dot_both_land() {
    let hit = nvp_effect(7371, 20, 200, EF_DONT_USE_QR);
    let tick = dot(nvp_effect(7372, 30, 150, EF_DONT_USE_QR));
    let (mut mgr, ability) = fixture(7370, vec![hit, tick]);

    fire(&mut mgr, &ability, 1).await;

    assert_eq!(pools(&mgr), (1000 - 20 - 30, 1000 - 200 - 150));
    assert_eq!(mgr.get_entity(NPC).unwrap().active_effects.len(), 1);
}
