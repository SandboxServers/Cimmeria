//! Where the weapon requirement sits in the launch (CS-07 review finding 7):
//! ahead of the cooldown, on the ground-target path too, and with a
//! throttled log row on its client-controlled path.

use std::time::Duration;

use tracing::Level;

use super::weapon_requirement::{
    assert_wrong_weapon, press, scene, PLAYER, QUICK_BURST, SGHC_SMG, SK37_LMG, TARGET,
};
use super::*;
use crate::test_support::{Captured, LogCapture};

const ITEM_GRENADE_LAUNCHER: i64 = 312_541_303;
const LAUNCH_GRENADE_SINGLE: i32 = 2419;

fn refusal_rows(all: &[Captured]) -> Vec<Captured> {
    all.iter()
        .filter(|c| c.target == "abilities" && c.has_field("event", "wrong_weapon_refused"))
        .cloned()
        .collect()
}

/// **Regression guard.** The requirement is checked before the cooldown:
/// a press of 598 with the SK37 LMG while 598 is cooling down still gets
/// WrongWeaponType and its feedback line. Checked after the cooldown (as
/// python did) the press would be refused silently as `OnCooldown`.
#[tokio::test]
async fn the_requirement_is_checked_before_the_cooldown() {
    let mut mgr = scene(QUICK_BURST, Some(SK37_LMG));
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .abilities
        .start_ability_cooldown(QUICK_BURST, Duration::from_secs(30));
    let (committed, msgs) = press(&mut mgr, QUICK_BURST).await;
    assert!(!committed);
    let errors: Vec<_> = super::warmup::calls(&msgs)
        .into_iter()
        .filter(|(_, m, _)| *m == method_idx::ON_ERROR_CODE)
        .collect();
    assert_eq!(
        errors.len(),
        1,
        "the wrong weapon is reported even while the ability cools down: {errors:?}"
    );
}

/// **Regression guard.** The ground-target path goes through the same gate:
/// Launch Grenade: Single (ITEM_Grenade_Launcher, ground-targeted, so python
/// never checked it) on the hostile's feet with the SK37 LMG is refused
/// with WrongWeaponType; it charges nothing and hurts no one.
#[tokio::test]
async fn a_ground_cast_with_the_wrong_weapon_is_refused() {
    let mut mgr = scene(QUICK_BURST, Some(SK37_LMG));
    mgr.ability_defs.insert(
        LAUNCH_GRENADE_SINGLE,
        AbilityDef {
            item_monikers: vec![ITEM_GRENADE_LAUNCHER],
            target_type_id: 3,
            ..make_ability(LAUNCH_GRENADE_SINGLE, 0, 30)
        },
    );
    mgr.get_entity_mut(PLAYER)
        .unwrap()
        .abilities
        .add_ability(LAUNCH_GRENADE_SINGLE);
    let health_before = mgr
        .get_entity(TARGET)
        .unwrap()
        .stats
        .get(cimmeria_entity::stats::HEALTH)
        .map(|s| s.cur);

    let (tx, mut rx) = mpsc::channel(256);
    let deaths = crate::cell::abilities::handle_use_ability_on_ground(
        PLAYER,
        LAUNCH_GRENADE_SINGLE,
        [3.0, 0.0, 0.0],
        &tx,
        &mut mgr,
    )
    .await;
    let msgs = drain(&mut rx);

    assert!(deaths.is_empty());
    assert_wrong_weapon(&mgr, LAUNCH_GRENADE_SINGLE, false, &msgs);
    assert_eq!(
        mgr.get_entity(TARGET)
            .unwrap()
            .stats
            .get(cimmeria_entity::stats::HEALTH)
            .map(|s| s.cur),
        health_before,
        "a refused ground cast damages no one"
    );
}

/// Pattern D, both guards, on `wrong_weapon_refused`:
///
/// 1. **Burst.** Five refused presses inside the window write one row; every
///    press still gets its feedback (the player sees each one).
/// 2. **Independence.** A second player's first refusal is written although
///    the first player's window is open.
#[tokio::test]
async fn the_refusal_row_is_throttled_per_player() {
    const OTHER: u32 = 3;
    let mut mgr = scene(QUICK_BURST, Some(SK37_LMG));
    make_player(&mut mgr, OTHER, [0.0, 0.0, 1.0]);
    {
        let bandolier = mgr.get_entity(PLAYER).unwrap().bandolier_items.clone();
        let o = mgr.get_entity_mut(OTHER).unwrap();
        o.abilities.add_ability(QUICK_BURST);
        o.weapon_holstered = false;
        o.active_bandolier_slot = 0;
        o.bandolier_items = bandolier;
    }
    let logs = LogCapture::install();

    let mut feedback_lines = 0;
    for _ in 0..5 {
        let (committed, msgs) = press(&mut mgr, QUICK_BURST).await;
        assert!(!committed);
        feedback_lines += super::warmup::calls(&msgs)
            .iter()
            .filter(|(_, m, _)| *m == method_idx::ON_PLAYER_COMMUNICATION)
            .count();
    }
    assert_eq!(feedback_lines, 5, "every press is answered");
    let rows = refusal_rows(&logs.all());
    assert_eq!(rows.len(), 1, "one row for the burst: {rows:#?}");
    assert!(rows[0].has_field("suppressed", "0"));
    assert_eq!(rows[0].level, Level::INFO);

    let (tx, _rx) = mpsc::channel(256);
    assert!(!handle_use_ability(OTHER, QUICK_BURST, TARGET as i32, &tx, &mut mgr).await);
    let rows = refusal_rows(&logs.all());
    assert_eq!(rows.len(), 2, "another player's first refusal is written");
    assert!(rows[1].has_field("entity_id", &OTHER.to_string()));

    // The window is per entity and released with it.
    assert_eq!(mgr.ability_refusal_log.tracked(), 2);
    mgr.destroy_entity(OTHER);
    assert_eq!(mgr.ability_refusal_log.tracked(), 1);

    // The SMG (which carries the moniker) fires: the throttle never gates a
    // legitimate cast.
    let mut ok = scene(QUICK_BURST, Some(SGHC_SMG));
    let (committed, _) = press(&mut ok, QUICK_BURST).await;
    assert!(committed);
}
