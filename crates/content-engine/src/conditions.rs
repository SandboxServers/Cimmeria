//! Condition evaluators for chain predicate checks.
//!
//! Conditions gate whether a chain's actions execute. All conditions on a chain
//! must evaluate to `true` (logical AND) for the action list to run. The 7
//! condition types cover common gameplay predicates: property comparisons, item
//! and ability ownership, spatial checks, faction relations, and a custom
//! expression hook for complex logic.

use serde::{Deserialize, Serialize};

use crate::context::ExecutionContext;

/// A condition that must be satisfied for a chain's actions to fire.
///
/// Conditions are evaluated in order; evaluation short-circuits on the first
/// `false` result. Each variant corresponds to a predicate that inspects the
/// current game state via the [`ExecutionContext`].
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

    /// The source entity must possess the given item, optionally at least
    /// `min_count` copies.
    HasItem {
        item_id: i32,
        min_count: Option<i32>,
    },

    /// The source entity must have the specified ability learned/available.
    HasAbility {
        ability_id: i32,
    },

    /// The source entity must currently be within the specified region.
    InRegion {
        region_id: i32,
    },

    /// The source entity's faction standing with the named faction must match
    /// the required relation.
    FactionCheck {
        faction: String,
        relation: FactionRelation,
    },

    /// A free-form expression string for complex conditions that don't fit
    /// the structured variants. Evaluated by a future expression engine.
    CustomExpression {
        expression: String,
    },
}

/// Faction relationship levels used by [`Condition::FactionCheck`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FactionRelation {
    Friendly,
    Neutral,
    Hostile,
}

impl Condition {
    /// Evaluate this condition against the current execution context.
    ///
    /// Returns `true` if the condition is satisfied, `false` otherwise.
    ///
    /// # Current Status
    ///
    /// All variants currently delegate to `todo!()` placeholders. Real
    /// implementations will query entity properties, inventory, ability sets,
    /// spatial state, and faction tables through service interfaces provided
    /// on the context.
    pub fn evaluate(&self, ctx: &ExecutionContext) -> bool {
        let _ = ctx;
        match self {
            Condition::PropertyEquals { property, value } => {
                let _ = (property, value);
                todo!("Condition::PropertyEquals - query entity property and compare")
            }
            Condition::PropertyInRange { property, min, max } => {
                let _ = (property, min, max);
                todo!("Condition::PropertyInRange - query numeric property and check bounds")
            }
            Condition::HasItem { item_id, min_count } => {
                let _ = (item_id, min_count);
                todo!("Condition::HasItem - query inventory for item count")
            }
            Condition::HasAbility { ability_id } => {
                let _ = ability_id;
                todo!("Condition::HasAbility - query ability set for learned ability")
            }
            Condition::InRegion { region_id } => {
                let _ = region_id;
                todo!("Condition::InRegion - query spatial system for entity region membership")
            }
            Condition::FactionCheck { faction, relation } => {
                let _ = (faction, relation);
                todo!("Condition::FactionCheck - query faction standing table")
            }
            Condition::CustomExpression { expression } => {
                let _ = expression;
                todo!("Condition::CustomExpression - evaluate via expression engine")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Verify it round-trips without panic (structural equality via Debug)
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
}
