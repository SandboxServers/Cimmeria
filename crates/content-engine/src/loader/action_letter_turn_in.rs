//! Strict DB-row conversion for Frost's Letter's atomic Marsh hand-in.
//!
//! The action uses `target_id = 1360`, no `target_key`, zero delay and:
//!
//! ```json
//! {"expected_step_id":4038,"item_id":3730,"quantity":1,"dialog_id":4576,"dialog_set_id":120101,"slot":10}
//! ```
//!
//! A malformed row refuses its entire chain. Dropping just this action
//! could leave a success dialog or mission completion behind.

use crate::actions::Action;

use super::DbActionRow;

/// The `action_type` this module owns.
pub(super) const ACTION_TYPE: &str = "complete_mission_with_item";

/// Convert the one approved mission/item hand-in tuple.
pub(super) fn convert_letter_turn_in(row: &DbActionRow) -> Result<Action, String> {
    if row.target_id != Some(1360) || row.target_key.is_some() {
        return Err("target_id must be 1360 and target_key must be empty".into());
    }
    if row.delay_ms != 0 {
        return Err("delay_ms must be zero".into());
    }
    let Some(params) = row.params.as_object() else {
        return Err("params must be an object with the six fixed fields".into());
    };
    const FIELDS: [(&str, i32); 6] = [
        ("expected_step_id", 4038),
        ("item_id", 3730),
        ("quantity", 1),
        ("dialog_id", 4576),
        ("dialog_set_id", 120101),
        ("slot", 10),
    ];
    if params.len() != FIELDS.len() {
        return Err("params must contain exactly the six fixed fields".into());
    }
    for (name, expected) in FIELDS {
        let actual = params
            .get(name)
            .and_then(|value| value.as_i64())
            .and_then(|value| i32::try_from(value).ok());
        if actual != Some(expected) {
            return Err(format!("`{name}` must be {expected}"));
        }
    }
    Ok(Action::CompleteMissionWithItem {
        mission_id: 1360,
        expected_step_id: 4038,
        item_id: 3730,
        quantity: 1,
        dialog_id: 4576,
        dialog_set_id: 120101,
        slot: 10,
    })
}
