//! `start_minigame` row -> `on_failure_chains` (MG-02, D-MG5). No database.

use super::super::action::convert_action;
use super::super::*;
use crate::actions::Action;

fn failure_chains(params: serde_json::Value) -> Vec<i64> {
    let row = DbActionRow {
        chain_id: 9001,
        action_type: "start_minigame".to_string(),
        target_id: None,
        target_key: Some("Livewire".to_string()),
        params,
        delay_ms: 0,
        sort_order: 1,
    };
    match convert_action(&row) {
        Some(Action::StartMinigame {
            on_failure_chains, ..
        }) => on_failure_chains,
        other => panic!("start_minigame must convert to StartMinigame, got {other:?}"),
    }
}

/// `on_failure_chains` parses like `on_victory_chains`.
/// Revert: drop the loader's `on_failure_chains` read and this yields `[]`.
#[test]
fn start_minigame_parses_on_failure_chains() {
    assert_eq!(
        failure_chains(serde_json::json!({ "on_failure_chains": [7] })),
        vec![7]
    );
}

/// An omitted key is an empty list, not a rejection: every seeded row today
/// omits it.
#[test]
fn start_minigame_without_on_failure_chains_has_none() {
    assert!(failure_chains(serde_json::json!({ "on_victory_chains": [1017] })).is_empty());
}
