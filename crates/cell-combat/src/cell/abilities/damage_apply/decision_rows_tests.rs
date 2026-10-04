//! AB-T3 regression guards: the hit's decision rows.
//!
//! - `abilities.qr` `qr_rolled`: one row per hit, with the roll and its
//!   result.
//! - `abilities.effect` `effect_planned`: one row per effect per target,
//!   with the path the effect took and why.
//! - `abilities.effect` `nvp_damage_resolved`: the pools before and after,
//!   the result code, the damage type and what the shields took; a shield
//!   that took some of it logs `shield_absorbed_damage` with the hit's ids.
//!
//! Removing a row, or logging it from somewhere that no longer sees the
//! decision, fails these asserts.

use super::single_damage_path_tests::{
    damage_effect, fire, fixture, pools, seq_rolling, NPC, PISTOL_SHOT, PISTOL_SHOT_EFFECT,
};
use super::*;
use crate::test_support::{Captured, LogCapture};
use cimmeria_entity::abilities::EffectDef;
use cimmeria_entity::stats::ABSORB_PHYSICAL;

/// The test player's `player_id` (`make_mgr_player_vs_npc`).
const PID: &str = "100";

fn rows<'a>(all: &'a [Captured], event: &str) -> Vec<&'a Captured> {
    all.iter().filter(|c| c.has_field("event", event)).collect()
}

/// The `effect_planned` rows for `effect_id`.
fn planned(all: &[Captured], effect_id: i32) -> Vec<&Captured> {
    rows(all, "effect_planned")
        .into_iter()
        .filter(|c| c.has_field("effect_id", &effect_id.to_string()))
        .collect()
}

fn the_one<'a>(found: Vec<&'a Captured>, what: &str) -> &'a Captured {
    assert_eq!(found.len(), 1, "exactly one {what} row: {found:#?}");
    found[0]
}

/// **Guard (AB-T3).** A missed Pistol Shot logs one `qr_rolled` row naming
/// the miss, and its damage script's `effect_planned` row says `skipped`
/// because of the miss. Without the QR row, or with the plan logged before
/// the miss gate, the asserts fail.
#[tokio::test]
async fn a_missed_shot_logs_the_miss_roll_and_a_skipped_plan() {
    let effect = damage_effect(PISTOL_SHOT_EFFECT, Some("RangedPhysicalDamage"), 15, 150, 0);
    let (mut mgr, ability) = fixture(PISTOL_SHOT, effect, 0);
    let seq = seq_rolling(&mgr, (1, NPC), PISTOL_SHOT, true);
    let logs = LogCapture::install();

    fire(&mut mgr, &ability, seq).await;

    let all = logs.all();
    let qr = the_one(rows(&all, "qr_rolled"), "qr_rolled");
    assert_eq!(qr.target, "abilities.qr");
    for (field, want) in [
        ("result_code", RC_MISS.to_string()),
        ("result", "miss".to_string()),
        ("dont_use_qr", "false".to_string()),
        ("forced", "false".to_string()),
        ("player_id", PID.to_string()),
        ("target_id", NPC.to_string()),
        ("ability_id", PISTOL_SHOT.to_string()),
    ] {
        assert!(
            qr.has_field(field, &want),
            "qr_rolled {field} = {want}: {qr:?}"
        );
    }
    assert!(qr.fields.contains_key("roll") && qr.fields.contains_key("qr"));

    let plan = the_one(planned(&all, PISTOL_SHOT_EFFECT), "effect_planned");
    assert_eq!(plan.target, "abilities.effect");
    assert!(
        plan.has_field("path", "skipped") && plan.has_field("reason", "miss"),
        "the damage script is planned skipped for the miss: {plan:?}"
    );
    assert!(plan.has_field("player_id", PID));
    assert_eq!(pools(&mgr), (1000, 0), "and the miss dealt nothing");
}

/// **Guard (AB-T3, B-22).** A Pistol Shot hit logs exactly one damage path
/// for its effect: the script. No `nvp_damage_resolved` row names the
/// effect. If the NVP pipeline took the effect too (the B-22 double hit),
/// or the plan row went missing, the asserts fail.
#[tokio::test]
async fn pistol_shot_logs_exactly_one_damage_path() {
    let effect = damage_effect(PISTOL_SHOT_EFFECT, Some("RangedPhysicalDamage"), 15, 150, 0);
    let (mut mgr, ability) = fixture(PISTOL_SHOT, effect, 1000);
    let seq = seq_rolling(&mgr, (1, NPC), PISTOL_SHOT, false);
    let logs = LogCapture::install();

    fire(&mut mgr, &ability, seq).await;

    let all = logs.all();
    let plan = the_one(planned(&all, PISTOL_SHOT_EFFECT), "effect_planned");
    assert!(
        plan.has_field("path", "script") && plan.has_field("reason", "damage_script"),
        "{plan:?}"
    );
    assert!(plan.has_field("nvp", "false") && plan.has_field("script", "true"));
    assert!(
        rows(&all, "nvp_damage_resolved").is_empty(),
        "the NVP pipeline never touches a damage script's effect"
    );
    let qr = the_one(rows(&all, "qr_rolled"), "qr_rolled");
    assert!(!qr.has_field("result_code", &RC_MISS.to_string()));
}

/// An NVP-only effect of 100 Health, and the NPC at full pools.
fn nvp_fixture() -> (SpaceManager, AbilityDef) {
    fixture(7200, damage_effect(7201, None, 100, 0, 0), 1000)
}

/// **Guard (AB-T3).** An NVP hit logs its plan (`nvp`, at the hit roll) and
/// one `nvp_damage_resolved` row with the result code, the damage type and
/// the target's pools before and after, which match the target.
#[tokio::test]
async fn an_nvp_hit_logs_the_pools_before_and_after() {
    let (mut mgr, ability) = nvp_fixture();
    let seq = seq_rolling(&mgr, (1, NPC), 7200, false);
    let logs = LogCapture::install();

    fire(&mut mgr, &ability, seq).await;

    let all = logs.all();
    let plan = the_one(planned(&all, 7201), "effect_planned");
    assert!(plan.has_field("path", "nvp") && plan.has_field("reason", "hit_roll"));
    let qr = the_one(rows(&all, "qr_rolled"), "qr_rolled");
    let result_code = qr.fields["result_code"].clone();

    let resolved = the_one(rows(&all, "nvp_damage_resolved"), "nvp_damage_resolved");
    assert_eq!(resolved.target, "abilities.effect");
    let (health_after, focus_after) = pools(&mgr);
    assert!(health_after < 1000, "the hit landed");
    for (field, want) in [
        ("health_before", "1000".to_string()),
        ("health_after", health_after.to_string()),
        ("focus_before", "1000".to_string()),
        ("focus_after", focus_after.to_string()),
        ("health_dealt", (1000 - health_after).to_string()),
        ("result_code", result_code),
        ("damage_type", DT_PHYSICAL.to_string()),
        ("absorbed", "0".to_string()),
        ("player_id", PID.to_string()),
        ("effect_id", "7201".to_string()),
    ] {
        assert!(
            resolved.has_field(field, &want),
            "nvp_damage_resolved {field} = {want}: {resolved:?}"
        );
    }
    assert!(rows(&all, "shield_absorbed_damage").is_empty());
}

/// **Guard (AB-T3, the shield row's ids).** A shield in front of an NVP
/// hit: `nvp_damage_resolved` reports what it took, and the
/// `shield_absorbed_damage` row names the attacker, the target and the
/// effect. Before AB-T3 the pipeline logged that row with no ids at all.
#[tokio::test]
async fn a_shielded_nvp_hit_logs_the_absorb_with_the_hit_ids() {
    let (mut mgr, ability) = nvp_fixture();
    let shield = mgr
        .get_entity_mut(NPC)
        .unwrap()
        .stats
        .get_mut(ABSORB_PHYSICAL)
        .unwrap();
    shield.update(0, 10_000, 10_000);
    let seq = seq_rolling(&mgr, (1, NPC), 7200, false);
    let logs = LogCapture::install();

    fire(&mut mgr, &ability, seq).await;

    let all = logs.all();
    let resolved = the_one(rows(&all, "nvp_damage_resolved"), "nvp_damage_resolved");
    let absorbed: i32 = resolved.fields["absorbed"].parse().unwrap();
    assert!(absorbed > 0, "the shield took the hit: {resolved:?}");
    assert!(resolved.has_field("health_after", "1000"));
    let row = the_one(
        rows(&all, "shield_absorbed_damage"),
        "shield_absorbed_damage",
    );
    for (field, want) in [
        ("player_id", PID.to_string()),
        ("entity_id", "1".to_string()),
        ("target_id", NPC.to_string()),
        ("ability_id", "7200".to_string()),
        ("effect_id", "7201".to_string()),
        ("absorbed", absorbed.to_string()),
    ] {
        assert!(
            row.has_field(field, &want),
            "shield row {field} = {want}: {row:?}"
        );
    }
}

/// **Guard (AB-T3 review).** A damage script's absorb row carries the
/// cast's `cast_id` and `stage`, as the NVP one does, so a shielded
/// Pistol Shot joins its `ability_launched` row.
#[tokio::test]
async fn a_shielded_damage_script_logs_the_absorb_in_its_cast() {
    let effect = damage_effect(PISTOL_SHOT_EFFECT, Some("RangedPhysicalDamage"), 15, 150, 0);
    let (mut mgr, ability) = fixture(PISTOL_SHOT, effect, 0);
    let shield = mgr
        .get_entity_mut(NPC)
        .unwrap()
        .stats
        .get_mut(ABSORB_PHYSICAL)
        .unwrap();
    shield.update(0, 10_000, 10_000);
    let seq = seq_rolling(&mgr, (1, NPC), PISTOL_SHOT, false);
    let outer = mgr.enter_cast_scope(Some(42));
    let logs = LogCapture::install();

    fire(&mut mgr, &ability, seq).await;
    mgr.exit_cast_scope(outer);

    let all = logs.all();
    let row = the_one(
        rows(&all, "shield_absorbed_damage"),
        "shield_absorbed_damage",
    );
    for (field, want) in [
        ("cast_id", "42"),
        ("stage", "apply"),
        ("player_id", PID),
        ("effect_id", "654"),
    ] {
        assert!(row.has_field(field, want), "{field} = {want}: {row:?}");
    }
}

/// **Guard (AB-T3 review, the area collapse).** The seed's Point Blank Fire
/// shape: two radius effects that both deal Health, no direct single. The
/// legacy collapse keeps the last one's value, so only it resolves; the
/// first is planned `skipped` / `area_collapsed`, never `nvp`.
#[tokio::test]
async fn a_collapsed_area_effect_is_planned_skipped() {
    use cimmeria_entity::abilities::TCM_AE_RADIUS;
    let area = |id| EffectDef {
        target_collection_method: TCM_AE_RADIUS.to_string(),
        ..damage_effect(id, None, 40, 0, 0)
    };
    let (mut mgr, mut ability) = fixture(1332, area(1575), 1000);
    mgr.effect_defs.insert(1574, area(1574));
    ability.effect_ids = vec![1575, 1574];
    mgr.ability_defs.insert(1332, ability.clone());
    let seq = seq_rolling(&mgr, (1, NPC), 1332, false);
    let logs = LogCapture::install();

    fire(&mut mgr, &ability, seq).await;

    let all = logs.all();
    let first = the_one(planned(&all, 1575), "effect_planned 1575");
    assert!(
        first.has_field("path", "skipped") && first.has_field("reason", "area_collapsed"),
        "{first:?}"
    );
    assert!(first.has_field("nvp", "false"));
    let last = the_one(planned(&all, 1574), "effect_planned 1574");
    assert!(last.has_field("path", "nvp"), "{last:?}");
    let resolved = the_one(rows(&all, "nvp_damage_resolved"), "nvp_damage_resolved");
    assert!(resolved.has_field("effect_id", "1574"), "{resolved:?}");
}

/// **Guard (AB-T3 with #1170 god mode).** An NVP hit on a god-mode target:
/// the target keeps its Health, and `nvp_damage_resolved` says
/// `god_mode = true`, so its `health_after` (logged before the restore)
/// is read as the hit's damage, not what the target kept. Without the
/// field the row claims a loss that was put back.
#[tokio::test]
async fn a_god_mode_target_s_nvp_row_says_god_mode() {
    let (mut mgr, ability) = nvp_fixture();
    mgr.get_entity_mut(NPC).unwrap().god_mode = true;
    let seq = seq_rolling(&mgr, (1, NPC), 7200, false);
    let logs = LogCapture::install();

    fire(&mut mgr, &ability, seq).await;

    assert_eq!(pools(&mgr).0, 1000, "god mode put the Health back");
    let all = logs.all();
    let resolved = the_one(rows(&all, "nvp_damage_resolved"), "nvp_damage_resolved");
    assert!(resolved.has_field("god_mode", "true"), "{resolved:?}");
    assert!(
        !rows(&all, "god_mode_absorbed").is_empty(),
        "the restore row says what was put back"
    );
}
