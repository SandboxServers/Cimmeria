//! The NPC selector must not pick a melee ability for a target it cannot
//! reach, and must not freeze when melee is all it has.
//!
//! Until Harset packet H09 widened `ability_set_abilities`'s primary key,
//! an NPC could own exactly one ability, so no NPC ever held a ranged and a
//! melee auto-attack at once and the question could not arise. It does now:
//! set 4 is `584 Staff Auto Attack` + `710 Staff Melee AA` and set 5 is
//! `711 Ribbon Device Melee AA` + `712 Ribbon Device Auto Attack`.
//!
//! The defect that would have shipped without this gate: `is_ranged` is
//! read only by `calculate_qr` to pick the accuracy/defence branch and has
//! never gated distance, while `max_range` is the `0` sentinel on every
//! auto-attack in the seed. So a melee auto-attack resolved to the full
//! 30 m ranged default and the NPC played a staff or ribbon *swing* at a
//! target thirty metres away. Twelve of the fifteen stationary spawn rows
//! in the seed use set 4, and a pinned sentry can never close the gap to
//! make that swing truthful.
//!
//! The guards below pin all four required behaviours:
//!
//! 1. beyond melee reach, the ranged half of a set wins even when the melee
//!    half sorts first (set 5's shape);
//! 2. inside melee reach, the melee half wins again — the filter is a range
//!    gate, not a blanket preference for ranged;
//! 3. a mobile NPC with only melee available closes the distance and then
//!    swings, rather than freezing or swinging from range;
//! 4. a pinned NPC uses its ranged half when it has one, and holds
//!    (`stationary_holds`) rather than swinging at air when it does not.
//!
//! Plus the no-change guard for every set that predates H09.

use super::{make_ai_fixture, seed_target_with_threat};
use crate::cell::combat::{NPC_ATTACK_RANGE, NPC_MELEE_RANGE};
use crate::cell::service::npc_ai::{choose_npc_ability, choose_npc_ability_within_reach};
use cimmeria_entity::abilities::AbilityDef;
use tokio::sync::mpsc;

/// Ability set 4 — the Jaffa staff pair. Ranged half sorts first.
const STAFF_AUTO_ATTACK: i32 = 584;
const STAFF_MELEE_AA: i32 = 710;
/// Ability set 5 — the Goa'uld ribbon-device pair. **Melee** half sorts
/// first, which is what makes this set the interesting one.
const RIBBON_MELEE_AA: i32 = 711;
const RIBBON_AUTO_ATTACK: i32 = 712;
/// Ability set 1 — the pistol set, a single ranged ability. Stands in for
/// every pre-H09 set in the no-change guard.
const PISTOL_AUTO_ATTACK: i32 = 579;

/// A target 20 m out: beyond `NPC_MELEE_RANGE` (3) and well inside
/// `NPC_ATTACK_RANGE` (30), so the two gates disagree and the test can tell
/// which one ran. Also inside `LEASH_DISTANCE` (50) so the fight tick does
/// not leash out from under the assertion.
const BEYOND_MELEE: f32 = 20.0;
/// A target 2 m out — inside `NPC_MELEE_RANGE`, so both gates agree.
const WITHIN_MELEE: f32 = 2.0;

/// Seed the four auto-attack defs the H09 sets are built from.
///
/// Only `is_ranged` and `max_range` are load-bearing for the reach gate, so
/// those two carry their real seeded values (`max_range = 0` on all four —
/// the "use the server default" sentinel — and `is_ranged` false for the
/// two `Melee AA` rows). `cooldown` also mirrors the seed because the
/// fallback arm is exercised by cooling one half. `flags` and
/// `target_type_id` are deliberately zeroed rather than copied: the real
/// values (21 / 277 and 2) feed `handle_use_ability` gates that have
/// nothing to do with range, and pulling them in would make a failure here
/// ambiguous.
fn seed_weapon_pair_defs(mgr: &mut crate::cell::space_manager::SpaceManager) {
    for (ability_id, name, is_ranged, cooldown) in [
        (PISTOL_AUTO_ATTACK, "Pistol Auto Attack", true, 2.0),
        (STAFF_AUTO_ATTACK, "Staff Auto Attack", true, 3.0),
        (STAFF_MELEE_AA, "Staff Melee AA", false, 2.0),
        (RIBBON_MELEE_AA, "Ribbon Device Melee AA", false, 2.0),
        (RIBBON_AUTO_ATTACK, "Ribbon Device Auto Attack", true, 2.5),
    ] {
        mgr.ability_defs.insert(
            ability_id,
            AbilityDef {
                ability_id,
                name: name.to_string(),
                cooldown,
                warmup: 0.0,
                flags: 0,
                is_ranged,
                min_range: 0,
                // The seed sentinel. This is the whole point: without the
                // `is_ranged` branch a `0` here resolves to 30.0 for a
                // melee swing exactly as it does for a rifle shot.
                max_range: 0,
                target_type_id: 0,
                effect_ids: vec![],
                moniker_ids: vec![],
                required_ammo: 0,
                event_set_id: Some(300),
                velocity: 0.0,
            },
        );
    }
}

/// NPC 200 at the origin, in `Fighting`, holding `abilities`, with player
/// 100 on its threat list at `dist` metres due east.
fn npc_with_set(abilities: &[i32], dist: f32) -> crate::cell::space_manager::SpaceManager {
    let mut mgr = make_ai_fixture([0.0; 3], [0.0; 3]);
    seed_weapon_pair_defs(&mut mgr);
    seed_target_with_threat(&mut mgr, 200, 100, [dist, 0.0, 0.0]);
    if let Some(npc) = mgr.get_entity_mut(200) {
        for &id in abilities {
            npc.abilities.add_ability(id);
        }
    }
    mgr
}

async fn tick(mgr: &mut crate::cell::space_manager::SpaceManager) {
    let (tx, _rx) = mpsc::channel(64);
    crate::cell::service::npc_ai::npc_ai_tick(
        &tx,
        mgr,
        &cimmeria_content_engine::chain::ChainEngine::new(),
    )
    .await;
}

// ──────────────────────────────────────────────────────────────────────
// 1 + 2. The selector itself.
// ──────────────────────────────────────────────────────────────────────

/// Set 5 at 20 m: the melee half sorts *first*, so the unfiltered
/// lowest-off-cooldown walk would hand back 711 and the Goa'uld would mime
/// a ribbon swing across twenty metres. The reach filter must skip it and
/// return the ranged half.
///
/// This is the revert canary for the whole packet: delete the
/// `effective_max_range` melee branch and this returns 711.
#[tokio::test]
async fn ranged_half_wins_when_the_target_is_beyond_melee_reach() {
    let mgr = npc_with_set(&[RIBBON_MELEE_AA, RIBBON_AUTO_ATTACK], BEYOND_MELEE);

    assert_eq!(
        choose_npc_ability_within_reach(200, &mgr, BEYOND_MELEE, NPC_ATTACK_RANGE),
        Some(RIBBON_AUTO_ATTACK),
        "at {BEYOND_MELEE} m the selector must skip the melee half ({RIBBON_MELEE_AA}) and \
         take the ranged half ({RIBBON_AUTO_ATTACK}); a {RIBBON_MELEE_AA} here means the \
         reach filter is gone and the NPC is swinging at a target it cannot touch"
    );
    // Control: the unfiltered selector — still the fallback arm — really
    // does prefer the melee half, so the assertion above is not passing
    // because 711 was unavailable for some unrelated reason.
    assert_eq!(
        choose_npc_ability(200, &mgr),
        Some(RIBBON_MELEE_AA),
        "control: without the reach filter the lowest-id walk picks the melee half"
    );
}

/// The same set at 2 m: now the melee half is genuinely usable, and since
/// it sorts first it wins. Pairs with the test above to prove the filter is
/// a distance gate and not a standing preference for ranged abilities.
#[tokio::test]
async fn melee_half_wins_when_the_target_is_inside_melee_reach() {
    let mgr = npc_with_set(&[RIBBON_MELEE_AA, RIBBON_AUTO_ATTACK], WITHIN_MELEE);

    assert_eq!(
        choose_npc_ability_within_reach(200, &mgr, WITHIN_MELEE, NPC_ATTACK_RANGE),
        Some(RIBBON_MELEE_AA),
        "inside {NPC_MELEE_RANGE} m the melee half is usable and sorts first, so it must \
         win; a {RIBBON_AUTO_ATTACK} here means the filter rejects melee unconditionally \
         rather than by distance"
    );
}

/// Set 4 at 20 m with the ranged half cooling — the case where *nothing*
/// is in reach. The selector must still hand back the melee ability rather
/// than `None`, because `None` reads to the fight tick as "all cooling,
/// hold fire" and a melee-only NPC would stand still forever instead of
/// walking in.
#[tokio::test]
async fn out_of_reach_falls_back_to_the_melee_pick_rather_than_holding_fire() {
    let mut mgr = npc_with_set(&[STAFF_AUTO_ATTACK, STAFF_MELEE_AA], BEYOND_MELEE);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.abilities
            .start_ability_cooldown(STAFF_AUTO_ATTACK, std::time::Duration::from_secs(60));
    }

    assert_eq!(
        choose_npc_ability_within_reach(200, &mgr, BEYOND_MELEE, NPC_ATTACK_RANGE),
        Some(STAFF_MELEE_AA),
        "with nothing in reach the selector must fall back to the unfiltered pick so the \
         fight tick's out-of-range arm can walk the NPC in; `None` would be read as \
         'all cooling' and the NPC would freeze"
    );
}

/// Every ability set that predates H09 holds one `is_ranged = true`
/// ability, so the reach filter must be a no-op for them: it returns
/// exactly what the unfiltered walk returns, at any distance inside
/// `NPC_ATTACK_RANGE`.
#[tokio::test]
async fn single_ranged_ability_sets_are_unchanged_by_the_reach_filter() {
    let mgr = npc_with_set(&[PISTOL_AUTO_ATTACK], BEYOND_MELEE);

    let filtered = choose_npc_ability_within_reach(200, &mgr, BEYOND_MELEE, NPC_ATTACK_RANGE);
    assert_eq!(
        filtered,
        choose_npc_ability(200, &mgr),
        "a one-ranged-ability set must select identically with and without the reach \
         filter — sets 1, 2, 3 and every Castle set have this shape"
    );
    assert_eq!(filtered, Some(PISTOL_AUTO_ATTACK));
}

// ──────────────────────────────────────────────────────────────────────
// 3. Mobile NPC: close the distance, then swing.
// ──────────────────────────────────────────────────────────────────────

/// A mobile NPC whose only ability is melee must not fire at 20 m, and must
/// not take the stationary hold branch either — it belongs in the movement
/// arm. Then, with the target inside melee reach, the same NPC fires.
///
/// The "walked in" half is asserted through the movement arm's own log
/// rather than through `nav_path`: the Castle test fixture carries no
/// navmesh, so `find_path` returns `None` and the arm emits `no_path`
/// instead of writing waypoints. What matters for this guard is which arm
/// ran, and `no_path` is emitted only by the non-stationary out-of-range
/// branch.
///
/// Revert shape: without the melee branch in `effective_max_range`,
/// `ability_ranges` reports 30 m for `710`, `in_range` is true at 20 m, and
/// the NPC fires — so `is_on_cooldown` flips and the `no_path` log never
/// appears.
#[tokio::test]
async fn melee_only_npc_closes_the_distance_instead_of_swinging_from_range() {
    use crate::test_support::LogCapture;
    use tracing::Level;

    let mut mgr = npc_with_set(&[STAFF_MELEE_AA], BEYOND_MELEE);

    let capture = LogCapture::install();
    tick(&mut mgr).await;

    assert!(
        !mgr.get_entity(200)
            .unwrap()
            .abilities
            .is_on_cooldown(STAFF_MELEE_AA),
        "a melee ability must not fire at {BEYOND_MELEE} m; a started cooldown is the \
         pre-fix signature, where `max_range = 0` resolved to the 30 m ranged default"
    );
    // The movement arm reports through the shared `npc_ai.path_fail`
    // emitter (WARN, one shape for every AI state), not a fight-local
    // INFO line.
    assert!(
        capture
            .find_message(Level::WARN, "npc_ai.path_fail: fight found no navmesh path")
            .is_some(),
        "the NPC must take the movement arm at {BEYOND_MELEE} m (this fixture has no \
         navmesh, so that arm reports `no_path`). Captured: {:#?}",
        capture.all()
    );
    assert!(
        capture
            .find_message(Level::INFO, "NPC AI: stationary mob holding fire")
            .is_none(),
        "a mobile NPC must never take the stationary hold branch"
    );
    drop(capture);

    // Now it has closed the distance. The same ability fires.
    if let Some(p) = mgr.get_entity_mut(100) {
        p.position = cimmeria_common::Vector3::new(WITHIN_MELEE, 0.0, 0.0);
    }
    tick(&mut mgr).await;

    assert!(
        mgr.get_entity(200)
            .unwrap()
            .abilities
            .is_on_cooldown(STAFF_MELEE_AA),
        "once inside {NPC_MELEE_RANGE} m the melee ability must fire — a gate that only \
         ever refuses would replace 'swings at air' with 'never attacks'"
    );
}

// ──────────────────────────────────────────────────────────────────────
// 4. Pinned NPC: use the ranged half, or hold.
// ──────────────────────────────────────────────────────────────────────

/// A pinned NPC holding set 5 at 20 m must fire the ranged half. This is
/// the half of the fix that matters most for the seed as shipped: a sentry
/// can never close distance, so if the selector hands it the melee ability
/// the NPC does nothing at all for the whole fight.
///
/// Revert shape: without the reach filter the selector returns 711 (it
/// sorts first), and 711 — not 712 — ends up on cooldown.
#[tokio::test]
async fn pinned_npc_uses_its_ranged_half_rather_than_miming_a_swing() {
    let mut mgr = npc_with_set(&[RIBBON_MELEE_AA, RIBBON_AUTO_ATTACK], BEYOND_MELEE);
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.is_stationary = true;
    }

    tick(&mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        npc.abilities.is_on_cooldown(RIBBON_AUTO_ATTACK),
        "a pinned NPC that owns a ranged ability must use it at {BEYOND_MELEE} m"
    );
    assert!(
        !npc.abilities.is_on_cooldown(RIBBON_MELEE_AA),
        "the melee half must not have been fired — a cooldown on {RIBBON_MELEE_AA} is the \
         pre-fix signature, where the lowest-id walk handed the sentry its swing"
    );
}

/// A pinned NPC whose only ability is melee must hold fire — and take the
/// `stationary_holds` branch, which since packet H51 also turns it to face
/// its target — rather than swinging at air.
///
/// Revert shape: without the melee branch, `in_range` is true at 20 m, the
/// NPC fires, and the hold log never appears.
#[tokio::test]
async fn pinned_melee_only_npc_holds_fire_instead_of_swinging_at_air() {
    use crate::test_support::LogCapture;
    use cimmeria_common::Vector3;
    use std::f32::consts::FRAC_PI_2;
    use tracing::Level;

    // Target due WEST so the expected yaw is -PI/2 and a merely-zeroed yaw
    // cannot pass the facing assertion.
    let mut mgr = npc_with_set(&[STAFF_MELEE_AA], BEYOND_MELEE);
    if let Some(p) = mgr.get_entity_mut(100) {
        p.position = Vector3::new(-BEYOND_MELEE, 0.0, 0.0);
    }
    if let Some(npc) = mgr.get_entity_mut(200) {
        npc.is_stationary = true;
        npc.direction = Vector3::new(0.0, FRAC_PI_2, 0.0);
    }

    let capture = LogCapture::install();
    tick(&mut mgr).await;

    let npc = mgr.get_entity(200).unwrap();
    assert!(
        !npc.abilities.is_on_cooldown(STAFF_MELEE_AA),
        "a pinned NPC must not swing at a target {BEYOND_MELEE} m away; a started cooldown \
         means the melee reach gate is gone"
    );
    assert!(
        capture
            .find_message(Level::INFO, "NPC AI: stationary mob holding fire")
            .is_some(),
        "holding fire must land in the `stationary_holds` branch, not be a silent \
         no-op. Captured: {:#?}",
        capture.all()
    );
    let yaw = npc.direction.y;
    assert!(
        (yaw + FRAC_PI_2).abs() < 1e-4,
        "`stationary_holds` turns the NPC to face its target (-PI/2 due west); got {yaw}"
    );
}
