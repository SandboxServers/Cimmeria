//! `convert_condition` — DB row → `Condition` enum variant — and the
//! associated `parse_comparison_op` / `parse_mission_status` /
//! `parse_step_status` helpers.

use crate::conditions::{ComparisonOp, Condition, MissionStatusValue, StepStatusValue};

use super::DbConditionRow;

/// Convert a DB condition row to a Condition enum variant.
pub(super) fn convert_condition(row: &DbConditionRow) -> Option<Condition> {
    let op = parse_comparison_op(&row.operator)?;
    match row.condition_type.as_str() {
        "mission_status" => {
            let mission_id = row.target_id?;
            let status = parse_mission_status(row.value.as_deref()?)?;
            Some(Condition::MissionStatus {
                mission_id,
                operator: op,
                expected_status: status,
            })
        }
        "step_status" => {
            let mission_id = row.target_id?;
            let step_id = row.target_key.as_deref()?.parse().ok()?;
            let status = parse_step_status(row.value.as_deref()?)?;
            Some(Condition::StepStatus {
                mission_id,
                step_id,
                operator: op,
                expected_status: status,
            })
        }
        "archetype" => {
            let archetype_id = row.value.as_deref()?.parse().ok()?;
            Some(Condition::Archetype {
                operator: op,
                archetype_id,
            })
        }
        "objective_status" => {
            let mission_id = row.target_id?;
            let objective_id = row.target_key.as_deref()?.parse().ok()?;
            let expected = row.value.as_deref()?.to_string();
            Some(Condition::ObjectiveStatus {
                mission_id,
                objective_id,
                operator: op,
                expected_status: expected,
            })
        }
        "counter" => {
            let counter_name = row.target_key.as_deref()?.to_string();
            let value = row.value.as_deref()?.parse().ok()?;
            Some(Condition::Counter {
                counter_name,
                operator: op,
                value,
            })
        }
        "stat_below_max" => {
            // target_id carries the stat id (no operator/value/key — the
            // condition is structural: cur < max). Extra columns are
            // ignored so chain authors don't accidentally encode an
            // operator that the evaluator can't honor.
            let stat_id = row.target_id?;
            Some(Condition::StatBelowMax { stat_id })
        }
        _ => None,
    }
}

pub(super) fn parse_comparison_op(s: &str) -> Option<ComparisonOp> {
    match s {
        "eq" => Some(ComparisonOp::Eq),
        "neq" => Some(ComparisonOp::Neq),
        "gte" => Some(ComparisonOp::Gte),
        "lte" => Some(ComparisonOp::Lte),
        "gt" => Some(ComparisonOp::Gt),
        "lt" => Some(ComparisonOp::Lt),
        _ => None,
    }
}

pub(super) fn parse_mission_status(s: &str) -> Option<MissionStatusValue> {
    match s {
        "not_active" => Some(MissionStatusValue::NotActive),
        "active" => Some(MissionStatusValue::Active),
        "completed" => Some(MissionStatusValue::Completed),
        _ => None,
    }
}

pub(super) fn parse_step_status(s: &str) -> Option<StepStatusValue> {
    match s {
        "not_active" => Some(StepStatusValue::NotActive),
        "active" => Some(StepStatusValue::Active),
        "completed" => Some(StepStatusValue::Completed),
        _ => None,
    }
}
