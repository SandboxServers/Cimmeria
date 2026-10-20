//! HA-1360a3a: the mandatory letter hand-in loads as one fixed action or
//! refuses the entire chain, including its other actions and triggers.

use super::super::action_letter_turn_in::convert_letter_turn_in;
use super::super::*;
use crate::chain::ChainEngine;

fn params() -> serde_json::Value {
    serde_json::json!({
        "expected_step_id": 4038,
        "item_id": 3730,
        "quantity": 1,
        "dialog_id": 4576,
        "dialog_set_id": 120101,
        "slot": 10
    })
}

fn letter_row() -> DbActionRow {
    DbActionRow {
        chain_id: 6501,
        action_type: "complete_mission_with_item".into(),
        target_id: Some(1360),
        target_key: None,
        params: params(),
        delay_ms: 0,
        sort_order: 0,
    }
}

fn chain_row(id: i32) -> DbChainRow {
    DbChainRow {
        chain_id: id,
        description: Some("Frost's Letter hand-in".into()),
        scope_type: "mission".into(),
        scope_id: Some(1360),
        enabled: true,
        priority: 0,
    }
}

fn trigger_row() -> DbTriggerRow {
    DbTriggerRow {
        chain_id: 6501,
        event_type: "interact_tag".into(),
        event_key: Some("Harset_ColonelMarsh".into()),
        scope: "player".into(),
        once: false,
        sort_order: 0,
    }
}

fn condition_row() -> DbConditionRow {
    DbConditionRow {
        chain_id: 6501,
        condition_type: "step_status".into(),
        target_id: Some(1360),
        target_key: Some("4038".into()),
        operator: "eq".into(),
        value: Some("active".into()),
        sort_order: 0,
    }
}

fn load_letter(row: DbActionRow) -> Vec<crate::chain::Chain> {
    build_chains_from_rows(
        vec![chain_row(6501)],
        vec![
            trigger_row(),
            DbTriggerRow {
                sort_order: 1,
                ..trigger_row()
            },
        ],
        vec![condition_row()],
        vec![
            DbActionRow {
                action_type: "display_dialog".into(),
                target_id: Some(4576),
                sort_order: -1,
                ..letter_row()
            },
            row,
            DbActionRow {
                action_type: "complete_mission".into(),
                target_id: Some(1360),
                sort_order: 1,
                ..letter_row()
            },
        ],
    )
}

#[test]
fn fixed_tuple_loads_with_zero_delay_and_counts_as_step_advancer() {
    let action = convert_letter_turn_in(&letter_row()).unwrap();
    assert_eq!(
        action,
        Action::CompleteMissionWithItem {
            mission_id: 1360,
            expected_step_id: 4038,
            item_id: 3730,
            quantity: 1,
            dialog_id: 4576,
            dialog_set_id: 120101,
            slot: 10,
        }
    );
    let chains = build_chains_from_rows(
        vec![chain_row(6501)],
        vec![trigger_row()],
        vec![condition_row()],
        vec![letter_row()],
    );
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].actions, vec![action]);
    assert_eq!(chains[0].action_delays, vec![0]);

    let mut engine = ChainEngine::new();
    engine.register_chain(chains.into_iter().next().unwrap());
    assert_eq!(engine.step_anchors(1360, 4038, None).advancers, vec![6501]);
}

#[test]
fn every_malformed_field_refuses_the_whole_multi_trigger_chain() {
    let mut bad_rows = Vec::new();
    for key in [
        "expected_step_id",
        "item_id",
        "quantity",
        "dialog_id",
        "dialog_set_id",
        "slot",
    ] {
        let mut wrong = letter_row();
        wrong.params[key] = serde_json::json!(0);
        bad_rows.push(wrong);
        let mut missing = letter_row();
        missing.params.as_object_mut().unwrap().remove(key);
        bad_rows.push(missing);
        let mut wrong_type = letter_row();
        wrong_type.params[key] = serde_json::json!("wrong");
        bad_rows.push(wrong_type);
    }
    let mut wrong_mission = letter_row();
    wrong_mission.target_id = Some(1361);
    bad_rows.push(wrong_mission);
    let mut missing_mission = letter_row();
    missing_mission.target_id = None;
    bad_rows.push(missing_mission);
    let mut target_key = letter_row();
    target_key.target_key = Some("4038".into());
    bad_rows.push(target_key);
    let mut delayed = letter_row();
    delayed.delay_ms = 1;
    bad_rows.push(delayed);
    let mut negative_delay = letter_row();
    negative_delay.delay_ms = -1;
    bad_rows.push(negative_delay);
    let mut extra_param = letter_row();
    extra_param.params["unexpected"] = serde_json::json!(1);
    bad_rows.push(extra_param);
    let mut non_object = letter_row();
    non_object.params = serde_json::json!([]);
    bad_rows.push(non_object);

    for row in bad_rows {
        assert!(convert_letter_turn_in(&row).is_err(), "{row:?}");
        assert!(
            load_letter(row).is_empty(),
            "a malformed mandatory action must discard all actions and trigger copies"
        );
    }
}

#[test]
fn malformed_letter_does_not_refuse_unrelated_chains() {
    let mut bad = letter_row();
    bad.params["quantity"] = serde_json::json!(2);
    let chains = build_chains_from_rows(
        vec![chain_row(6501), chain_row(6502)],
        vec![trigger_row()],
        vec![condition_row()],
        vec![
            bad,
            DbActionRow {
                chain_id: 6502,
                action_type: "display_dialog".into(),
                target_id: Some(123),
                ..letter_row()
            },
        ],
    );
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].id, 6502);
}
