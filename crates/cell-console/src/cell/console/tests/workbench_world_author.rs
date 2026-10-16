//! Dispatch guards behind the reviewed patrol and entity authoring forms.

use cimmeria_content_engine::chain::ChainEngine;
use tokio::sync::mpsc;

use super::{decode_feedback, setup};
use crate::cell::console::handle_console_command;

#[tokio::test]
async fn malformed_path_id_refuses_before_buffer_or_sql_queue() {
    let (mut mgr, gm, _) = setup();
    let (tx, mut rx) = mpsc::channel(16);
    handle_console_command(gm, ".path_add invalid", &tx, &mut mgr, &ChainEngine::new()).await;
    assert!(mgr.patrol_authoring.is_empty());
    assert!(mgr.authoring_changes.get(&gm).is_none_or(Vec::is_empty));
    let feedback: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|msg| decode_feedback(&msg))
        .collect();
    assert!(
        feedback
            .iter()
            .any(|line| line.contains("invalid") || line.contains("integer")),
        "{feedback:?}"
    );
}

#[tokio::test]
async fn missing_selection_refuses_entity_edit_and_patrol_assignment() {
    let (mut mgr, gm, npc) = setup();
    mgr.get_entity_mut(gm).unwrap().current_target_id = None;
    let old_tag = mgr.get_entity(npc).unwrap().tag.clone();
    let old_ai = mgr.get_entity(npc).unwrap().ai_state();
    let (tx, mut rx) = mpsc::channel(32);
    for command in [".tag new-tag", ".path_assign 7"] {
        handle_console_command(gm, command, &tx, &mut mgr, &ChainEngine::new()).await;
    }
    assert_eq!(mgr.get_entity(npc).unwrap().tag, old_tag);
    assert_eq!(mgr.get_entity(npc).unwrap().ai_state(), old_ai);
    assert!(mgr.authoring_changes.get(&gm).is_none_or(Vec::is_empty));
    let feedback: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|msg| decode_feedback(&msg))
        .collect();
    assert_eq!(
        feedback
            .iter()
            .filter(|line| line.contains("target is required"))
            .count(),
        2,
        "{feedback:?}"
    );
}
