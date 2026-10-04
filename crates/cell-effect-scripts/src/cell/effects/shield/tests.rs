//! `AbsorbShield` on the timed effect ledger: the pools it grants, its
//! expiry, its removal and its skip paths.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use cimmeria_entity::abilities::{EffectDef, DT_ENERGY, DT_HAZMAT, DT_PHYSICAL, DT_UNTYPED};
use cimmeria_entity::stats::{ABSORB_ENERGY, ABSORB_HAZMAT, ABSORB_PHYSICAL, ABSORB_UNTYPED};
use tracing::Level;

use super::*;
use crate::cell::effects::stat_buff::StatBuffRemoval;
use crate::cell::effects::test_fixtures::make_mgr_with_target;
use crate::cell::space_manager::SpaceManager;
use crate::test_support::LogCapture;

/// Effect 4306 as the `shield` family writes it: "Absorption: 500 Physical
/// / 500 Energy / 500 Contamination", 30 s, flags 342.
fn personal_shield() -> EffectDef {
    shield_effect(
        &[
            ("ShieldAmount", "500"),
            ("ShieldType", "Physical,Energy,Hazmat"),
        ],
        30.0,
    )
}

fn shield_effect(nvps: &[(&str, &str)], pulse_duration: f32) -> EffectDef {
    EffectDef {
        effect_id: 4306,
        ability_id: 1013,
        flags: 342,
        pulse_count: 1,
        pulse_duration,
        script_name: Some("AbsorbShield".to_string()),
        params: nvps
            .iter()
            .map(|&(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<_, _>>(),
        ..Default::default()
    }
}

fn cast(mgr: &mut SpaceManager, effect: &EffectDef) {
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect,
        space_mgr: mgr,
    };
    AbsorbShield.on_apply(&mut ctx);
}

fn absorb(mgr: &SpaceManager, stat: i32) -> i32 {
    mgr.get_entity(1).unwrap().stats.get(stat).unwrap().cur
}

#[test]
fn shield_types_reads_names_numbers_and_the_untyped_default() {
    let types = |v: Option<&str>| {
        let nvps: Vec<(&str, &str)> = v.map(|v| ("ShieldType", v)).into_iter().collect();
        shield_types(&shield_effect(&nvps, 0.0))
    };
    assert_eq!(
        types(Some("Physical, energy ,Contamination")),
        Some(vec![DT_PHYSICAL, DT_ENERGY, DT_HAZMAT])
    );
    assert_eq!(
        types(Some(&DT_PHYSICAL.to_string())),
        Some(vec![DT_PHYSICAL])
    );
    assert_eq!(types(None), Some(vec![DT_UNTYPED]));
    assert_eq!(types(Some(" ")), Some(vec![DT_UNTYPED]));
    assert_eq!(
        types(Some("Physical,Kinetic")),
        None,
        "Kinetic is a resist, not a damage type"
    );
    assert_eq!(
        types(Some("14")),
        None,
        "the client's DT_Physical is not a server damage type"
    );
}

/// Personal Shield: one ledger entry with three pools of 500, each in its
/// `absorb*` stat, expiring with the effect's 30 s.
#[test]
fn personal_shield_puts_three_pools_on_the_ledger() {
    let mut mgr = make_mgr_with_target();
    cast(&mut mgr, &personal_shield());
    for stat in [ABSORB_PHYSICAL, ABSORB_ENERGY, ABSORB_HAZMAT] {
        assert_eq!(absorb(&mgr, stat), 500, "stat {stat}");
    }
    assert_eq!(absorb(&mgr, ABSORB_UNTYPED), 0);
    let entries = &mgr.get_entity(1).unwrap().stat_buffs.entries;
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key(), (4306, 1));
    assert_eq!(entries[0].absorb_remaining(), 1500);
    assert_eq!(entries[0].duration_secs, 30.0);
    assert!(entries[0].expires_at.is_some());
}

/// The B-33 guard: the shield's expiry takes its pool back off. Before
/// AB-10 the script only added to the stat, no ledger entry existed, and
/// the capacity outlived the shield; the stat would still read 300 here.
#[test]
fn an_absorb_shield_expiry_removes_its_pool() {
    let mut mgr = make_mgr_with_target();
    cast(
        &mut mgr,
        &shield_effect(&[("ShieldAmount", "500"), ("ShieldType", "Physical")], 30.0),
    );
    // A hit drained 200, as the damage pipeline does, and the seam settled.
    mgr.get_entity_mut(1)
        .unwrap()
        .stats
        .get_mut(ABSORB_PHYSICAL)
        .unwrap()
        .change(-200);
    assert_eq!(mgr.settle_absorb_shields(1), 0, "300 left: still up");
    let later = Instant::now() + Duration::from_secs(31);
    let gone = mgr.remove_timed_effects(1, StatBuffRemoval::Expired, |b| b.is_expired(later));
    assert_eq!(gone.len(), 1);
    assert_eq!(
        absorb(&mgr, ABSORB_PHYSICAL),
        0,
        "the unspent 300 goes with it"
    );
}

/// Damage that empties the pool takes the shield off, logged `drained`.
#[test]
fn a_drained_shield_comes_off_logged_drained() {
    let mut mgr = make_mgr_with_target();
    cast(
        &mut mgr,
        &shield_effect(&[("ShieldAmount", "100"), ("ShieldType", "Physical")], 30.0),
    );
    mgr.get_entity_mut(1)
        .unwrap()
        .stats
        .get_mut(ABSORB_PHYSICAL)
        .unwrap()
        .change(-100);
    let capture = LogCapture::install();
    assert_eq!(mgr.settle_absorb_shields(1), 1);
    assert!(mgr.get_entity(1).unwrap().stat_buffs.entries.is_empty());
    assert!(
        capture
            .find_event(Level::INFO, "timed effect removed", "drained")
            .is_some(),
        "the removal row carries reason = drained"
    );
}

/// A pulsing instance that carried the script ended: `on_remove` takes the
/// entry and its capacity off.
#[test]
fn on_remove_takes_the_entry_and_its_capacity_off() {
    let mut mgr = make_mgr_with_target();
    let effect = personal_shield();
    cast(&mut mgr, &effect);
    let mut ctx = EffectContext {
        source_id: 1,
        target_id: 1,
        effect: &effect,
        space_mgr: &mut mgr,
    };
    AbsorbShield.on_remove(&mut ctx);
    assert!(mgr.get_entity(1).unwrap().stat_buffs.entries.is_empty());
    assert_eq!(absorb(&mgr, ABSORB_ENERGY), 0);
}

/// A row with no duration holds until drained or removed.
#[test]
fn a_shield_with_no_duration_is_held() {
    let mut mgr = make_mgr_with_target();
    cast(&mut mgr, &shield_effect(&[("ShieldAmount", "50")], 0.0));
    let entries = &mgr.get_entity(1).unwrap().stat_buffs.entries;
    assert_eq!(entries.len(), 1);
    assert!(entries[0].expires_at.is_none());
    assert_eq!(absorb(&mgr, ABSORB_UNTYPED), 50);
}

#[test]
fn a_shield_without_an_amount_warns_and_grants_nothing() {
    let mut mgr = make_mgr_with_target();
    let capture = LogCapture::install();
    cast(
        &mut mgr,
        &shield_effect(&[("ShieldType", "Physical")], 30.0),
    );
    assert!(mgr.get_entity(1).unwrap().stat_buffs.entries.is_empty());
    assert_eq!(absorb(&mgr, ABSORB_PHYSICAL), 0);
    assert!(capture
        .find_event(
            Level::WARN,
            "AbsorbShield has no positive ShieldAmount",
            "no_amount"
        )
        .is_some());
}

#[test]
fn a_shield_with_an_unknown_type_warns_and_grants_nothing() {
    let mut mgr = make_mgr_with_target();
    let capture = LogCapture::install();
    cast(
        &mut mgr,
        &shield_effect(&[("ShieldAmount", "50"), ("ShieldType", "Kinetic")], 30.0),
    );
    assert!(mgr.get_entity(1).unwrap().stat_buffs.entries.is_empty());
    assert!(capture
        .find_event(Level::WARN, "unknown ShieldType", "unknown_shield_type")
        .is_some());
}

/// The `shield` family's NVP names (between the `nvp-names` markers in
/// `tools/ability_mechanics/families/shield.py`) must each be read by the
/// script it binds: `ShieldAmount` / `ShieldType` by `AbsorbShield`, the
/// rest by `TimedStat` through `STAT_BUFF_NVPS`. A name nobody reads would
/// be a shield that silently does nothing.
#[test]
fn shield_nvp_names_match_the_generator() {
    let src = include_str!("../../../../../../tools/ability_mechanics/families/shield.py");
    let start = src.find("# nvp-names begin").expect("begin marker");
    let end = src.find("# nvp-names end").expect("end marker");
    let names: Vec<&str> = src[start..end].split('"').skip(1).step_by(2).collect();
    assert_eq!(names.len(), 3, "parsed {names:?}");
    for name in names {
        let read = [SHIELD_AMOUNT_NVP, SHIELD_TYPE_NVP].contains(&name)
            || crate::cell::effects::stat_buff::STAT_BUFF_NVPS
                .iter()
                .any(|&(n, _)| n == name);
        assert!(
            read,
            "the shield family writes {name}, which no script reads"
        );
    }
}
