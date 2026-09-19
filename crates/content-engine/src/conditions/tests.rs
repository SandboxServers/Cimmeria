//! Unit tests for the condition evaluators in the parent module.
//!
//! Split out of `conditions.rs` in the PR #662 review (finding 10): the
//! file had grown past the 500-line soft cap with ~250 of those lines
//! being tests, which is the natural seam the repo asks for. Same
//! `mod.rs` + `tests.rs` shape as `loader/` and `triggers/`.

use super::*;
use crate::context::ExecutionContext;

#[test]
fn faction_relation_serialization_roundtrip() {
    let relations = vec![
        FactionRelation::Friendly,
        FactionRelation::Neutral,
        FactionRelation::Hostile,
    ];
    for rel in &relations {
        let json = serde_json::to_string(rel).unwrap();
        let deserialized: FactionRelation = serde_json::from_str(&json).unwrap();
        assert_eq!(*rel, deserialized);
    }
}

#[test]
fn condition_serialization_roundtrip() {
    let condition = Condition::PropertyEquals {
        property: "health".to_string(),
        value: serde_json::json!(100),
    };
    let json = serde_json::to_string(&condition).unwrap();
    let deserialized: Condition = serde_json::from_str(&json).unwrap();
    let _ = format!("{:?}", deserialized);
}

#[test]
fn has_item_condition_serialization() {
    let condition = Condition::HasItem {
        item_id: 42,
        min_count: Some(3),
    };
    let json = serde_json::to_string(&condition).unwrap();
    assert!(json.contains("42"));
    assert!(json.contains("3"));
}

#[test]
fn mission_status_eq_not_active() {
    let condition = Condition::MissionStatus {
        mission_id: 622,
        operator: ComparisonOp::Eq,
        expected_status: MissionStatusValue::NotActive,
    };
    let ctx = ExecutionContext::new();
    // No param set → defaults to "not_active"
    assert!(condition.evaluate(&ctx));
}

#[test]
fn mission_status_neq_active() {
    let condition = Condition::MissionStatus {
        mission_id: 622,
        operator: ComparisonOp::Neq,
        expected_status: MissionStatusValue::Active,
    };
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "mission_622_status".to_string(),
        serde_json::json!("not_active"),
    );
    assert!(condition.evaluate(&ctx));
}

#[test]
fn step_status_active() {
    let condition = Condition::StepStatus {
        mission_id: 638,
        step_id: 2114,
        operator: ComparisonOp::Eq,
        expected_status: StepStatusValue::Active,
    };
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "mission_638_step_2114_status".to_string(),
        serde_json::json!("active"),
    );
    assert!(condition.evaluate(&ctx));
}

/// `StepStatusValue::Completed` lets a chain check whether a step has
/// already been advanced past, distinct from "step never reached"
/// (the unwrap_or("not_active") fallback). Population is the
/// populator's job in `services::cell::content::mission_context`;
/// this test only pins the comparison rule.
#[test]
fn step_status_completed() {
    let condition = Condition::StepStatus {
        mission_id: 641,
        step_id: 2121,
        operator: ComparisonOp::Eq,
        expected_status: StepStatusValue::Completed,
    };
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "mission_641_step_2121_status".to_string(),
        serde_json::json!("completed"),
    );
    assert!(condition.evaluate(&ctx));

    // Same step, but populator hasn't fired yet → param missing →
    // evaluator falls back to "not_active" → `eq completed` must be false.
    let empty = ExecutionContext::new();
    assert!(!condition.evaluate(&empty));
}

/// A step that's currently active is NOT completed. Locks down the
/// "active overrides completed if both somehow set" behaviour at the
/// evaluator level — population order in `mission_context.rs` writes
/// `active` last so it wins, and this test pins that the comparator
/// doesn't accidentally treat them as equivalent.
#[test]
fn step_status_active_is_not_completed() {
    let condition = Condition::StepStatus {
        mission_id: 641,
        step_id: 3563,
        operator: ComparisonOp::Eq,
        expected_status: StepStatusValue::Completed,
    };
    let mut ctx = ExecutionContext::new();
    ctx.set_param(
        "mission_641_step_3563_status".to_string(),
        serde_json::json!("active"),
    );
    assert!(!condition.evaluate(&ctx));
}

#[test]
fn archetype_eq() {
    let condition = Condition::Archetype {
        operator: ComparisonOp::Eq,
        archetype_id: 8,
    };
    let mut ctx = ExecutionContext::new();
    ctx.set_param("archetype".to_string(), serde_json::json!(8));
    assert!(condition.evaluate(&ctx));
}

#[test]
fn archetype_neq() {
    let condition = Condition::Archetype {
        operator: ComparisonOp::Neq,
        archetype_id: 8,
    };
    let mut ctx = ExecutionContext::new();
    ctx.set_param("archetype".to_string(), serde_json::json!(3));
    assert!(condition.evaluate(&ctx));
}

#[test]
fn counter_gte() {
    let condition = Condition::Counter {
        counter_name: "hallway01_kills".to_string(),
        operator: ComparisonOp::Gte,
        value: 3,
    };
    let mut ctx = ExecutionContext::new();
    ctx.set_param("counter_hallway01_kills".to_string(), serde_json::json!(3));
    assert!(condition.evaluate(&ctx));

    ctx.set_param("counter_hallway01_kills".to_string(), serde_json::json!(2));
    assert!(!condition.evaluate(&ctx));
}

/// `Condition::StatBelowMax` — pin the headroom semantics that gate
/// consumable chains. Slappack chain 4001 (`stat_below_max stat_id 7`)
/// relies on three branches: cur < max → fire (heal lands), cur == max
/// → no-op (chain doesn't fire, stack preserved), and missing populator
/// → fail-closed (treat as "no headroom" so a wiring mistake doesn't
/// silently make consumables free at full stat).
#[test]
fn stat_below_max_fires_when_cur_below_max() {
    let condition = Condition::StatBelowMax { stat_id: 7 };
    let mut ctx = ExecutionContext::new();
    ctx.set_param("stat_7_cur".to_string(), serde_json::json!(50));
    ctx.set_param("stat_7_max".to_string(), serde_json::json!(100));
    assert!(condition.evaluate(&ctx));
}

#[test]
fn stat_below_max_blocks_when_cur_equals_max() {
    let condition = Condition::StatBelowMax { stat_id: 7 };
    let mut ctx = ExecutionContext::new();
    ctx.set_param("stat_7_cur".to_string(), serde_json::json!(100));
    ctx.set_param("stat_7_max".to_string(), serde_json::json!(100));
    assert!(
        !condition.evaluate(&ctx),
        "at full stat the chain must NOT fire — burning a slappack at \
         full HP is the bug class this gates against",
    );
}

#[test]
fn stat_below_max_fails_closed_on_missing_params() {
    let condition = Condition::StatBelowMax { stat_id: 7 };

    // Empty context — both keys missing.
    assert!(
        !condition.evaluate(&ExecutionContext::new()),
        "missing populator must fail closed; otherwise a wiring mistake \
         that drops `populate_stats_context` makes consumables free at \
         full stat",
    );

    // Half-populated context — only `cur` set, `max` missing. Same
    // fail-closed branch.
    let mut partial = ExecutionContext::new();
    partial.set_param("stat_7_cur".to_string(), serde_json::json!(50));
    assert!(!condition.evaluate(&partial));
}

// ── Condition::World (Harset H07) ────────────────────────────────────
//
// World ids below are the real `resources.worlds.world_id` values:
// Harset = 57, Harset_CmdCenter = 68.

/// `world eq 57` fires in Harset and nowhere else. The negative half is
/// the whole point: `OnRegionEnter` matches a bare `point_sets.name`,
/// so the Command Center door chain and its mirror in
/// `Harset_CmdCenter` are only distinguishable by world.
#[test]
fn world_eq_matches_only_the_named_world() {
    let condition = Condition::World {
        operator: ComparisonOp::Eq,
        world_id: 57,
    };
    assert!(condition.evaluate(&ExecutionContext::new().with_world(57)));
    assert!(
        !condition.evaluate(&ExecutionContext::new().with_world(68)),
        "a chain gated on Harset (57) must not fire in Harset_CmdCenter (68)",
    );
}

#[test]
fn world_neq_matches_every_other_world() {
    let condition = Condition::World {
        operator: ComparisonOp::Neq,
        world_id: 57,
    };
    assert!(condition.evaluate(&ExecutionContext::new().with_world(68)));
    assert!(!condition.evaluate(&ExecutionContext::new().with_world(57)));
}

/// Fail-closed on an unpopulated context — for `Neq` as well as `Eq`.
///
/// `Neq` is the branch worth pinning: the "obvious" implementation
/// (`ctx.world_id != Some(expected)`) answers **true** for `None`, so a
/// dispatcher that forgot `populate_world_context` would fire every
/// `neq`-gated chain in every world — silently, and only in the
/// dispatch paths nobody tested.
#[test]
fn world_fails_closed_when_context_has_no_world() {
    let unset = ExecutionContext::new();
    assert!(!Condition::World {
        operator: ComparisonOp::Eq,
        world_id: 57,
    }
    .evaluate(&unset),);
    assert!(
        !Condition::World {
            operator: ComparisonOp::Neq,
            world_id: 57,
        }
        .evaluate(&unset),
        "an unpopulated world_id must not satisfy `neq` — a missing \
         populator would otherwise fire the chain in every world",
    );
}

/// Ordered operators are meaningless on an opaque id. They must answer
/// `false`, not compare numerically: `68 > 57` is true as arithmetic
/// and nonsense as a world gate.
///
/// Each operator is checked against a world on *both* sides of the
/// authored id. Against 68 alone, `Lt`/`Lte` answer `false` under a
/// numeric fall-through too (`68 < 57` is false), so half the loop
/// would be a tautology that passes with the bug present.
#[test]
fn world_ordered_operators_never_match() {
    for op in [
        ComparisonOp::Gt,
        ComparisonOp::Gte,
        ComparisonOp::Lt,
        ComparisonOp::Lte,
    ] {
        let condition = Condition::World {
            operator: op.clone(),
            world_id: 57,
        };
        for actual in [68, 57, 8] {
            assert!(
                !condition.evaluate(&ExecutionContext::new().with_world(actual)),
                "world {op:?} against world {actual} must not fall through to \
                 numeric comparison — a world id is an opaque key",
            );
        }
    }
}
