//! `row_to_message` decoder tests — pure logic, no DB. Covers the happy
//! path for each variant, an unknown event_type rejection, and the
//! defensive (event_type, payload) mismatch rejection.

use super::super::*;

#[test]
fn row_to_message_builds_item_used() {
    let row = OutboxRow {
        id: 99,
        entity_id: 1234,
        event_type: "item_used".to_string(),
        payload: sqlx::types::Json(CellOutboxPayload::ItemUsed {
            instance_id: 0,
            type_id: 19,
            target_id: 5,
        }),
        attempts: 0,
    };
    let msg = row_to_message(&row).expect("known event_type should produce message");
    match msg {
        BaseToCellMsg::ItemUsed {
            entity_id,
            instance_id: _,
            type_id,
            target_id,
        } => {
            assert_eq!(entity_id, 1234);
            assert_eq!(type_id, 19);
            assert_eq!(target_id, 5);
        }
        _ => panic!("expected ItemUsed"),
    }
}

#[test]
fn row_to_message_returns_none_for_unknown_event_type() {
    // Forward-compat: a future base might enqueue an event_type this
    // build doesn't know. Drainer must skip it (and rate-limit logging
    // off `attempts`), not panic / mis-dispatch.
    let row = OutboxRow {
        id: 100,
        entity_id: 1,
        event_type: "future_event_v2".to_string(),
        payload: sqlx::types::Json(CellOutboxPayload::ItemUsed {
            instance_id: 0,
            type_id: 0,
            target_id: 0,
        }),
        attempts: 0,
    };
    assert!(row_to_message(&row).is_none());
}

#[test]
fn row_to_message_builds_inventory_item_granted() {
    let row = OutboxRow {
        id: 200,
        entity_id: 42,
        event_type: "inventory_item_granted".to_string(),
        payload: sqlx::types::Json(CellOutboxPayload::InventoryItemGranted {
            item_id: 100_007,
            container_id: 1,
            slot_id: 2,
            quantity: 1,
        }),
        attempts: 0,
    };
    match row_to_message(&row).expect("known event_type") {
        BaseToCellMsg::InventoryItemGranted {
            entity_id,
            item_id,
            container_id,
            slot_id,
            quantity,
        } => {
            assert_eq!(entity_id, 42);
            assert_eq!(item_id, 100_007);
            assert_eq!(container_id, 1);
            assert_eq!(slot_id, 2);
            assert_eq!(quantity, 1);
        }
        _ => panic!("expected InventoryItemGranted"),
    }
}

#[test]
fn row_to_message_builds_inventory_item_removed() {
    let row = OutboxRow {
        id: 201,
        entity_id: 42,
        event_type: "inventory_item_removed".to_string(),
        payload: sqlx::types::Json(CellOutboxPayload::InventoryItemRemoved {
            item_id: 100_008,
            source_container_id: 3,
        }),
        attempts: 0,
    };
    match row_to_message(&row).expect("known event_type") {
        BaseToCellMsg::InventoryItemRemoved {
            entity_id,
            item_id,
            source_container_id,
        } => {
            assert_eq!(entity_id, 42);
            assert_eq!(item_id, 100_008);
            assert_eq!(source_container_id, 3);
        }
        _ => panic!("expected InventoryItemRemoved"),
    }
}

#[test]
fn row_to_message_returns_none_on_event_type_payload_mismatch() {
    // Defensive: an event_type that says "granted" but a payload that
    // shape-matches a different variant (legacy data, hand-edited row,
    // future variant rename half-rolled) must not silently dispatch the
    // wrong message — drainer should skip and log.
    let row = OutboxRow {
        id: 202,
        entity_id: 1,
        event_type: "inventory_item_granted".to_string(),
        payload: sqlx::types::Json(CellOutboxPayload::ItemUsed {
            instance_id: 0,
            type_id: 0,
            target_id: 0,
        }),
        attempts: 0,
    };
    assert!(row_to_message(&row).is_none());
}
