//! DB-row → `Condition` conversions.

use super::super::condition::convert_condition;
use super::super::*;
use crate::conditions::{ComparisonOp, Condition};

#[test]
fn convert_counter_condition() {
    let row = DbConditionRow {
        chain_id: 1,
        condition_type: "counter".to_string(),
        target_id: None,
        target_key: Some("hallway01_kills".to_string()),
        operator: "gte".to_string(),
        value: Some("3".to_string()),
        sort_order: 0,
    };
    let condition = convert_condition(&row).unwrap();
    match condition {
        Condition::Counter {
            counter_name,
            operator,
            value,
        } => {
            assert_eq!(counter_name, "hallway01_kills");
            assert_eq!(operator, ComparisonOp::Gte);
            assert_eq!(value, 3);
        }
        other => panic!("Expected Counter, got {:?}", other),
    }
}

// ── `world` (Harset H07) ─────────────────────────────────────────────────

/// Authoring shape for a `world` row: the numeric
/// `resources.worlds.world_id` in `target_id`, `eq`/`neq` in `operator`,
/// `target_key` and `value` unused. 57 is Harset.
fn world_row(operator: &str, world_id: Option<i32>) -> DbConditionRow {
    DbConditionRow {
        chain_id: 6001,
        condition_type: "world".to_string(),
        target_id: world_id,
        target_key: None,
        operator: operator.to_string(),
        value: None,
        sort_order: 0,
    }
}

#[test]
fn convert_world_condition_eq() {
    let condition = convert_condition(&world_row("eq", Some(57)))
        .expect("`world` must have a loader arm — without one the row is dropped as an unknown condition_type and the chain loads UNGATED");
    match condition {
        Condition::World { operator, world_id } => {
            assert_eq!(operator, ComparisonOp::Eq);
            assert_eq!(world_id, 57);
        }
        other => panic!("Expected World, got {:?}", other),
    }
}

#[test]
fn convert_world_condition_neq() {
    let condition = convert_condition(&world_row("neq", Some(68))).expect("neq must convert");
    match condition {
        Condition::World { operator, world_id } => {
            assert_eq!(operator, ComparisonOp::Neq);
            assert_eq!(world_id, 68);
        }
        other => panic!("Expected World, got {:?}", other),
    }
}

/// A row with no `target_id` has no world to compare against, so the
/// `world` arm must reject it rather than convert it into a condition that
/// compares against a default.
///
/// The positive case is asserted in the same test on purpose: on its own,
/// `is_none()` holds just as well when the `"world"` arm does not exist at
/// all and the row falls through to `_ => None`. Pairing them makes the
/// rejection relative to a demonstrably live arm.
#[test]
fn convert_world_condition_requires_target_id() {
    assert!(
        convert_condition(&world_row("eq", Some(57))).is_some(),
        "control: the `world` arm must be live for the rejection below to mean anything",
    );
    assert!(convert_condition(&world_row("eq", None)).is_none());
}

/// An ordered operator is nonsense on a world id, but the row is still
/// converted on purpose: `build_chains_from_rows` *drops* a row whose
/// conversion returns `None`, which would leave the chain ungated and fire
/// a door teleport in every world. Keeping the condition makes the chain
/// fail closed instead (the evaluator answers `false`); the loader warns.
#[test]
fn convert_world_condition_keeps_ordered_operator_so_the_chain_stays_gated() {
    let condition = convert_condition(&world_row("gte", Some(57))).expect(
        "an ordered operator must NOT drop the row — a dropped condition ungates the chain",
    );
    assert!(matches!(
        condition,
        Condition::World {
            operator: ComparisonOp::Gte,
            world_id: 57,
        }
    ));
}
