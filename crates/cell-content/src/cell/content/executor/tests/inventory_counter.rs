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
        action_delays: Vec::new(),
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
            vault,
        } => {
            // Player 1 has no vault session, so the vault is not searched.
            assert_eq!(vault.reason(), Some("no_vault_session"));
            assert_eq!(entity_id, 1);
            assert_eq!(player_id, 42);
            assert_eq!(type_id, 19);
            assert_eq!(count, 1);
        }
        other => panic!("expected RemoveInventoryItemByType, got {:?}", other),
    }
}

/// BV-03: with a vault window open (a GM `.bank` session here), a
/// by-instance removal (the item the player just used) carries the live,
/// open verdict, while a by-type removal (a turn-in) still never reaches
/// into the vault. Fails if the executor stops taking the verdict for the
/// instance, or starts passing it to the by-type search.
#[tokio::test]
async fn remove_item_takes_the_vault_verdict_only_by_instance() {
    use cimmeria_entity::cell_entity::{VaultScope, VaultSession};
    use cimmeria_wire::cell::vault::VaultAccess;

    let mut mgr = make_space_mgr();
    mgr.create_entity(1, "Agnos", [0.0, 0.0, 0.0], [0.0; 3])
        .unwrap();
    let space_id = mgr.get_entity_space_id(1).unwrap();
    mgr.get_entity_mut(1).unwrap().vault_session = Some(VaultSession {
        scope: VaultScope::Personal,
        banker_id: None,
        space_id,
        opened_at: std::time::Instant::now(),
        expansion_offer: None,
    });
    let (tx, mut rx) = mpsc::channel(8);
    let engine = ChainEngine::new();
    let remove = |params: std::collections::HashMap<String, serde_json::Value>| ResolvedActions {
        action_delays: Vec::new(),
        params,
        actions: vec![(
            1034,
            Action::RemoveItem {
                item_id: 19,
                count: 1,
            },
        )],
    };

    execute_actions(remove(Default::default()), 1, 42, &tx, &mut mgr, &engine).await;
    match rx.try_recv().expect("by-type removal") {
        CellToBaseMsg::RemoveInventoryItemByType { vault, .. } => {
            assert_eq!(
                vault,
                VaultAccess::NO_SESSION,
                "a turn-in never searches the vault"
            )
        }
        other => panic!("expected RemoveInventoryItemByType, got {other:?}"),
    }

    let by_instance = std::collections::HashMap::from([(
        "instance_id".to_string(),
        serde_json::Value::from(5001),
    )]);
    execute_actions(remove(by_instance), 1, 42, &tx, &mut mgr, &engine).await;
    match rx.try_recv().expect("by-instance removal") {
        CellToBaseMsg::RemoveInventoryItem { item_id, vault, .. } => {
            assert_eq!(item_id, 5001);
            assert!(vault.opens_personal_vault(), "{vault:?}");
        }
        other => panic!("expected RemoveInventoryItem, got {other:?}"),
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
        action_delays: Vec::new(),
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
        action_delays: Vec::new(),
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
        action_delays: Vec::new(),
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
