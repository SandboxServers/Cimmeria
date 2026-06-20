//! `Action::RemoveItem` + counter actions (`IncrementCounter` /
//! `ResetCounter`) executor coverage.

use super::*;

/// Regression for #95: `Action::RemoveItem` must route through the new
/// `RemoveInventoryItemByType` cell→base RPC, not the silently-ignored
/// stub it used to be. Locks in the chain-driven removal path that
/// chain 1034 (FindAmbernol consume) depends on.
#[tokio::test]
async fn remove_item_action_emits_remove_inventory_by_type() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1034,
            Action::RemoveItem {
                item_id: 19,
                count: 1,
            },
        )],
    };

    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let msg = rx.try_recv().expect("expected RemoveInventoryItemByType");
    match msg {
        CellToBaseMsg::RemoveInventoryItemByType {
            entity_id,
            player_id,
            type_id,
            count,
        } => {
            assert_eq!(entity_id, 1);
            assert_eq!(player_id, 42);
            assert_eq!(type_id, 19);
            assert_eq!(count, 1);
        }
        other => panic!("expected RemoveInventoryItemByType, got {:?}", other),
    }
}

/// `Action::IncrementCounter` mutates `entity.counters`. Previously
/// a stub that only logged; now load-bearing for kill-counter
/// missions like Mess Hall (counter `messhall_kills`) and Hallway05
/// (`hallway05_kills`). Pin the new-key initialization path:
/// missing entry → 0, then add `amount`.
#[tokio::test]
async fn increment_counter_initializes_and_adds_amount() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1085,
            Action::IncrementCounter {
                counter_name: "messhall_kills".to_string(),
                amount: 1,
            },
        )],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    let entity = mgr.get_entity(1).expect("entity must still exist");
    assert_eq!(
        entity.counters.get("messhall_kills"),
        Some(&1),
        "new counter must initialize at 0 and add `amount` (1)",
    );
}

/// `Action::IncrementCounter` on an existing counter adds to the
/// stored value rather than overwriting. The Mess Hall mission
/// design depends on this: each guard kill increments the same
/// counter; the second kill must read the first's stored value
/// for the completion chain's `gte (target - 1)` condition to
/// fire on the right kill.
#[tokio::test]
async fn increment_counter_adds_to_existing_value() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(1)
        .unwrap()
        .counters
        .insert("messhall_kills".to_string(), 1);

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1086,
            Action::IncrementCounter {
                counter_name: "messhall_kills".to_string(),
                amount: 1,
            },
        )],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    assert_eq!(
        mgr.get_entity(1).unwrap().counters.get("messhall_kills"),
        Some(&2),
        "second increment must add to the stored value, not overwrite",
    );
}

/// `Action::ResetCounter` removes the entry entirely. Subsequent
/// `Condition::Counter` reads see the missing-key default of 0.
/// Used by the Mess Hall completion chain (1087) so a re-accept
/// of mission 681 (e.g., the same player respawning into a fresh
/// instance) starts the counter clean.
#[tokio::test]
async fn reset_counter_clears_entry() {
    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    mgr.get_entity_mut(1)
        .unwrap()
        .counters
        .insert("messhall_kills".to_string(), 2);

    let (tx, _rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let resolved = ResolvedActions {
        params: std::collections::HashMap::new(),
        actions: vec![(
            1087,
            Action::ResetCounter {
                counter_name: "messhall_kills".to_string(),
            },
        )],
    };
    execute_actions(resolved, 1, 42, &tx, &mut mgr, &engine).await;

    assert!(
        !mgr.get_entity(1)
            .unwrap()
            .counters
            .contains_key("messhall_kills"),
        "reset must remove the entry — leaving a 0 entry would surface \
         via populate_counters_context as `counter_messhall_kills = 0` \
         rather than the missing-key default, masking a re-acceptance",
    );
}
