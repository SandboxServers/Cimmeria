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
