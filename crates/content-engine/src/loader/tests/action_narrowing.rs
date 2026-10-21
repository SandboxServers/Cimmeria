//! Out-of-range `i32` action params drop the action instead of wrapping
//! through `as i32` (#246).

use super::super::action::convert_action;
use super::super::*;

/// One past `i32::MAX`: `as i32` would wrap this to `i32::MIN`.
const OVER_I32: i64 = i32::MAX as i64 + 1;

/// One below `i32::MIN`: `as i32` would wrap this to `i32::MAX`.
const UNDER_I32: i64 = i32::MIN as i64 - 1;

/// `(action_type, target_id, target_key, key)`, one row per integer param
/// that the loader reads as `i32`.
const CASES: &[(&str, Option<i32>, Option<&str>, &str)] = &[
    ("add_dialog_set", Some(1), None, "slot"),
    ("add_dialog_set", Some(1), None, "mission_id"),
    ("remove_dialog_set", Some(1), None, "slot"),
    ("add_item", Some(1), None, "qty"),
    ("add_item", Some(1), None, "container"),
    ("remove_item", Some(1), None, "qty"),
    ("set_aggression", None, Some("NID_Guard_01"), "level"),
    ("trigger_transporter", None, None, "regionId"),
    ("qr_combat_damage", None, None, "stat_id"),
    ("qr_combat_damage", None, None, "source_id"),
    ("change_stat", None, None, "stat_id"),
    ("change_stat", None, None, "min"),
    ("change_stat", None, None, "max"),
    ("change_stat", None, None, "amount"),
    ("increment_counter", None, Some("counter_a"), "amount"),
    ("set_active_slot", None, None, "bag_id"),
    ("set_active_slot", None, None, "slot"),
    ("add_dialog", Some(1), None, "entity_template"),
    ("add_dialog", Some(1), None, "mission_id"),
    (
        "generate_threat",
        None,
        Some("NID_Guard_01"),
        "threat_level",
    ),
];

fn row(
    action_type: &str,
    target_id: Option<i32>,
    target_key: Option<&str>,
    params: serde_json::Value,
) -> DbActionRow {
    DbActionRow {
        chain_id: 1,
        action_type: action_type.to_string(),
        target_id,
        target_key: target_key.map(str::to_string),
        params,
        delay_ms: 0,
        sort_order: 0,
    }
}

/// A params object holding only `key`, set to `value`.
fn params_with(key: &str, value: i64) -> serde_json::Value {
    let mut params = serde_json::json!({});
    params[key] = serde_json::json!(value);
    params
}

#[test]
fn out_of_range_i32_param_drops_the_action() {
    for &(action_type, target_id, target_key, key) in CASES {
        let r = row(
            action_type,
            target_id,
            target_key,
            params_with(key, OVER_I32),
        );
        assert!(
            convert_action(&r).is_none(),
            "{action_type}.{key} wrapped instead of dropping the action"
        );
    }
}

/// The negative side of `i32::try_from`: `as i32` would wrap this to
/// `i32::MAX`, which is a valid value and so would be kept.
#[test]
fn below_i32_min_param_drops_the_action() {
    for &(action_type, target_id, target_key, key) in CASES {
        let r = row(
            action_type,
            target_id,
            target_key,
            params_with(key, UNDER_I32),
        );
        assert!(
            convert_action(&r).is_none(),
            "{action_type}.{key} wrapped instead of dropping the action"
        );
    }
}

/// Integers above `i64::MAX` parse as u64; they must drop the action too,
/// not fall back to the param default.
#[test]
fn above_i64_max_param_drops_the_action() {
    for &(action_type, target_id, target_key, key) in CASES {
        let mut params = serde_json::json!({});
        params[key] = serde_json::json!(u64::MAX);
        let r = row(action_type, target_id, target_key, params);
        assert!(
            convert_action(&r).is_none(),
            "{action_type}.{key} applied a default for a u64 value"
        );
    }
}

/// Negative control for the `u64` fallthrough in `opt_i32`: a negative
/// in-range integer is kept, and a float or string reads as absent, so the
/// action converts exactly as it does with the param missing. Only a
/// `u64` above `i64::MAX` may take the drop path.
#[test]
fn negative_and_non_integer_params_are_not_out_of_range() {
    for &(action_type, target_id, target_key, key) in CASES {
        let build = |params: serde_json::Value| row(action_type, target_id, target_key, params);
        let absent = format!("{:?}", convert_action(&build(serde_json::json!({}))));

        for value in [serde_json::json!(1.5), serde_json::json!("7")] {
            let mut params = serde_json::json!({});
            params[key] = value.clone();
            let got = format!("{:?}", convert_action(&build(params)));
            assert_eq!(
                got, absent,
                "{action_type}.{key}={value} should read as absent"
            );
        }

        assert!(
            convert_action(&build(params_with(key, -1))).is_some(),
            "{action_type}.{key} dropped at -1"
        );
    }
}

#[test]
fn in_range_i32_param_keeps_the_action() {
    for &(action_type, target_id, target_key, key) in CASES {
        let r = row(
            action_type,
            target_id,
            target_key,
            params_with(key, i32::MAX as i64),
        );
        assert!(
            convert_action(&r).is_some(),
            "{action_type}.{key} dropped at i32::MAX"
        );
    }
}

#[test]
fn add_item_qty_five_grants_five() {
    let r = row("add_item", Some(1), None, params_with("qty", 5));
    match convert_action(&r) {
        Some(Action::GrantItem { count, .. }) => assert_eq!(count, 5),
        other => panic!("expected GrantItem, got {other:?}"),
    }
}

/// `spawn_entity.aggression` lives in `action_spawn.rs`, not `action.rs`,
/// and needs a full spawn descriptor, so it cannot be a `CASES` row.
#[test]
fn spawn_entity_aggression_out_of_range_drops_the_action() {
    let spawn = |aggression: i64| {
        row(
            "spawn_entity",
            Some(207),
            Some("Rinla_Malac"),
            serde_json::json!({ "x": 1.0, "y": 2.0, "z": 3.0, "aggression": aggression }),
        )
    };
    for value in [OVER_I32, UNDER_I32] {
        assert!(
            convert_action(&spawn(value)).is_none(),
            "spawn_entity.aggression={value} wrapped instead of dropping the action"
        );
    }
    match convert_action(&spawn(i32::MAX as i64)) {
        Some(Action::SpawnEntity { aggression, .. }) => {
            assert_eq!(aggression, Some(i32::MAX));
        }
        other => panic!("expected SpawnEntity at i32::MAX, got {other:?}"),
    }
}
