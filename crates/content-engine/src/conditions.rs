//! Condition evaluators for chain predicate checks.
//!
//! Conditions gate whether a chain's actions execute. All conditions on a chain
//! must evaluate to `true` (logical AND) for the action list to run.

use serde::{Deserialize, Serialize};

use crate::context::ExecutionContext;

/// A condition that must be satisfied for a chain's actions to fire.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// The named property on the source entity must equal the given value.
    PropertyEquals {
        property: String,
        value: serde_json::Value,
    },

    /// The named numeric property must fall within [min, max] inclusive.
    PropertyInRange {
        property: String,
        min: f64,
        max: f64,
    },

    /// The source entity must possess the given item.
    HasItem {
        item_id: i32,
        min_count: Option<i32>,
    },

    /// The source entity must have the specified ability.
    HasAbility { ability_id: i32 },

    /// The source entity must currently be within the specified region.
    InRegion { region_id: i32 },

    /// Faction standing check.
    FactionCheck {
        faction: String,
        relation: FactionRelation,
    },

    /// Free-form expression for complex conditions.
    CustomExpression { expression: String },

    // ── DB-driven condition types ─────────────────────────────────────────
    /// Check if a mission has a specific status (not_active, active, completed).
    MissionStatus {
        mission_id: i32,
        operator: ComparisonOp,
        expected_status: MissionStatusValue,
    },

    /// Check if a mission step has a specific status.
    StepStatus {
        mission_id: i32,
        step_id: i32,
        operator: ComparisonOp,
        expected_status: StepStatusValue,
    },

    /// Check if the player's archetype matches a value.
    Archetype {
        operator: ComparisonOp,
        archetype_id: i32,
    },

    /// Check if a mission objective has a specific status.
    ObjectiveStatus {
        mission_id: i32,
        objective_id: i32,
        operator: ComparisonOp,
        expected_status: String,
    },

    /// Check if a named counter meets a comparison threshold.
    Counter {
        counter_name: String,
        operator: ComparisonOp,
        value: i32,
    },

    /// True iff the source entity's stat is below its current max
    /// (i.e., the stat has headroom to grow). Used to gate consumable
    /// chains so e.g. Health Slappacks fizzle silently rather than
    /// burning a stack when the player is already at full HP.
    ///
    /// Reads `stat_<id>_cur` and `stat_<id>_max` from the context;
    /// callers must populate these via `populate_stats_context` before
    /// resolving. Returns `false` (treat as "no headroom, don't fire")
    /// if either param is missing — fail-closed so a wiring mistake
    /// can't accidentally make consumables free-to-spam at full stat.
    StatBelowMax { stat_id: i32 },

    /// True iff the acting player's current world matches `world_id` under
    /// `operator`. `world_id` is `resources.worlds.world_id` — the same id
    /// space `spawnlist`, `stargates` and `ring_transport_regions` use;
    /// `entities/spaces.xml` maps the id's world name to its bounds.
    ///
    /// Reads the typed [`ExecutionContext::world_id`] field, *not* a
    /// `params` key: the value is resolved from the player's space by the
    /// services-side populator, and a typed field makes "nobody populated
    /// it" distinguishable from "populated with 0".
    ///
    /// Exists because no trigger carries the world. `OnRegionEnter` matches
    /// a bare `point_sets.name` string, so two worlds that both contain a
    /// region called `X.CommandCenterTransition` fire the same chain, and
    /// `OnPlayerLoaded`'s optional `world_name` filter is the only other
    /// world-aware primitive in the engine. A `world` condition lets any
    /// chain — region, player_loaded, interact, death — be zone-scoped.
    ///
    /// **Fail-closed** when `ctx.world_id` is `None`, for *every* operator
    /// including `Neq`: an unpopulated context means we do not know where
    /// the player is, and firing a door teleport in the wrong world is the
    /// exact bug class this condition was added to prevent. An author who
    /// wants "anywhere but Harset" still writes `neq 57`, which holds
    /// wherever the populator ran.
    World {
        operator: ComparisonOp,
        world_id: i32,
    },
}

/// Faction relationship levels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FactionRelation {
    Friendly,
    Neutral,
    Hostile,
}

/// Comparison operators used by DB-driven conditions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ComparisonOp {
    Eq,
    Neq,
    Gte,
    Lte,
    Gt,
    Lt,
}

/// Mission status values for MissionStatus conditions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MissionStatusValue {
    NotActive,
    Active,
    Completed,
}

/// Step status values for StepStatus conditions.
///
/// `NotActive` is the catch-all for "this step is not the current step" — true
/// both before the mission has been accepted and after the step has been
/// advanced past. Use `Completed` (set by the populator from
/// `MissionInstance.completed_steps`) when a chain needs to distinguish a step
/// that's already been passed from one that hasn't been reached yet.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepStatusValue {
    NotActive,
    Active,
    Completed,
}

impl Condition {
    /// Evaluate this condition against the current execution context.
    pub fn evaluate(&self, ctx: &ExecutionContext) -> bool {
        match self {
            Condition::PropertyEquals { property, value } => {
                ctx.params.get(property) == Some(value)
            }
            Condition::PropertyInRange { property, min, max } => ctx
                .params
                .get(property)
                .and_then(|v| v.as_f64())
                .is_some_and(|val| val >= *min && val <= *max),
            Condition::HasItem { item_id, min_count } => {
                let key = format!("item_{}_count", item_id);
                let required = min_count.unwrap_or(1) as f64;
                ctx.params
                    .get(&key)
                    .and_then(|v| v.as_f64())
                    .is_some_and(|count| count >= required)
            }
            Condition::HasAbility { ability_id } => {
                let key = format!("ability_{}", ability_id);
                ctx.params
                    .get(&key)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            }
            Condition::InRegion { region_id } => {
                ctx.params.get("current_region").and_then(|v| v.as_i64()) == Some(*region_id as i64)
            }
            Condition::FactionCheck { faction, relation } => {
                let key = format!("faction_{}", faction);
                let expected = match relation {
                    FactionRelation::Friendly => "Friendly",
                    FactionRelation::Neutral => "Neutral",
                    FactionRelation::Hostile => "Hostile",
                };
                ctx.params.get(&key).and_then(|v| v.as_str()) == Some(expected)
            }
            Condition::CustomExpression { expression } => ctx
                .params
                .get(expression)
                .and_then(|v| v.as_bool())
                .unwrap_or(false),

            // ── DB-driven conditions ──────────────────────────────────────
            Condition::MissionStatus {
                mission_id,
                operator,
                expected_status,
            } => {
                let key = format!("mission_{}_status", mission_id);
                let actual_str = ctx
                    .params
                    .get(&key)
                    .and_then(|v| v.as_str())
                    .unwrap_or("not_active");
                let expected_str = match expected_status {
                    MissionStatusValue::NotActive => "not_active",
                    MissionStatusValue::Active => "active",
                    MissionStatusValue::Completed => "completed",
                };
                compare_str(actual_str, expected_str, operator)
            }
            Condition::StepStatus {
                mission_id,
                step_id,
                operator,
                expected_status,
            } => {
                let key = format!("mission_{}_step_{}_status", mission_id, step_id);
                let actual_str = ctx
                    .params
                    .get(&key)
                    .and_then(|v| v.as_str())
                    .unwrap_or("not_active");
                let expected_str = match expected_status {
                    StepStatusValue::NotActive => "not_active",
                    StepStatusValue::Active => "active",
                    StepStatusValue::Completed => "completed",
                };
                compare_str(actual_str, expected_str, operator)
            }
            Condition::Archetype {
                operator,
                archetype_id,
            } => {
                let actual = ctx
                    .params
                    .get("archetype")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(-1);
                compare_i64(actual, *archetype_id as i64, operator)
            }
            Condition::ObjectiveStatus {
                mission_id,
                objective_id,
                operator,
                expected_status,
            } => {
                let key = format!("mission_{}_obj_{}_status", mission_id, objective_id);
                let actual_str = ctx
                    .params
                    .get(&key)
                    .and_then(|v| v.as_str())
                    .unwrap_or("not_active");
                compare_str(actual_str, expected_status, operator)
            }
            Condition::Counter {
                counter_name,
                operator,
                value,
            } => {
                let key = format!("counter_{}", counter_name);
                let actual = ctx.params.get(&key).and_then(|v| v.as_i64()).unwrap_or(0);
                compare_i64(actual, *value as i64, operator)
            }
            Condition::StatBelowMax { stat_id } => {
                let cur_key = format!("stat_{}_cur", stat_id);
                let max_key = format!("stat_{}_max", stat_id);
                let cur = ctx.params.get(&cur_key).and_then(|v| v.as_i64());
                let max = ctx.params.get(&max_key).and_then(|v| v.as_i64());
                match (cur, max) {
                    (Some(c), Some(m)) => c < m,
                    // Fail-closed: missing context means we don't know the
                    // stat state, so treat as "no headroom" rather than
                    // firing the chain blindly. A missing populator call
                    // would otherwise let consumables burn at full stat.
                    _ => false,
                }
            }
            Condition::World { operator, world_id } => {
                let Some(actual) = ctx.world_id else {
                    // Fail-closed for every operator. A dispatcher that
                    // forgot `populate_world_context` must not make a
                    // world-gated chain fire everywhere — that is the
                    // `OnRegionEnter`-ignores-world bug the condition
                    // exists to close.
                    tracing::debug!(
                        expected_world_id = world_id,
                        "Condition::World evaluated against a context with no world_id — \
                         failing closed; the firing dispatcher did not populate it"
                    );
                    return false;
                };
                compare_world_id(actual, *world_id, operator)
            }
        }
    }
}

/// Compare two world ids. Only `Eq` and `Neq` are meaningful — a world id
/// is an opaque identifier, not a scale, so `world gt 57` has no sane
/// meaning and answers `false` rather than "Harset_CmdCenter (68) is
/// greater than Harset (57)". The loader keeps such a row (dropping it
/// would leave the chain ungated) but warns at load time.
fn compare_world_id(actual: i32, expected: i32, op: &ComparisonOp) -> bool {
    match op {
        ComparisonOp::Eq => actual == expected,
        ComparisonOp::Neq => actual != expected,
        _ => false,
    }
}

/// Compare two strings using a ComparisonOp (only Eq and Neq are meaningful).
fn compare_str(actual: &str, expected: &str, op: &ComparisonOp) -> bool {
    match op {
        ComparisonOp::Eq => actual == expected,
        ComparisonOp::Neq => actual != expected,
        _ => false,
    }
}

/// Compare two integers using a ComparisonOp.
fn compare_i64(actual: i64, expected: i64, op: &ComparisonOp) -> bool {
    match op {
        ComparisonOp::Eq => actual == expected,
        ComparisonOp::Neq => actual != expected,
        ComparisonOp::Gte => actual >= expected,
        ComparisonOp::Lte => actual <= expected,
        ComparisonOp::Gt => actual > expected,
        ComparisonOp::Lt => actual < expected,
    }
}

#[cfg(test)]
mod tests {
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
}
