//! Outbox tests that don't need a live database.
//!
//! The DB-touching paths (`enqueue`, `enqueue_in_tx`, `mark_delivered`,
//! `record_failure`, `drain_undelivered`) are tested via the integration
//! suite gated on `sqlx::test` infra (issue #79). This file covers the
//! pure logic: payload serialization shape and row→message conversion.

use super::*;

#[test]
fn payload_serializes_with_kind_tag() {
    let p = CellOutboxPayload::ItemUsed { type_id: 42, target_id: 17 };
    let json = serde_json::to_value(&p).unwrap();
    // kind tag matters: it's how a future variant gets disambiguated on
    // deserialize, and an accidental rename would silently break in-flight
    // rows on production databases.
    assert_eq!(json["kind"], "item_used");
    assert_eq!(json["type_id"], 42);
    assert_eq!(json["target_id"], 17);
}

#[test]
fn payload_roundtrips_through_json() {
    let p = CellOutboxPayload::ItemUsed { type_id: -7, target_id: 0 };
    let json = serde_json::to_string(&p).unwrap();
    let back: CellOutboxPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(back, p);
}

#[test]
fn event_type_string_is_stable() {
    // This string is persisted in the `event_type` column. Changing it
    // breaks any in-flight rows on existing databases — the test exists
    // to make that breakage loud at refactor time.
    assert_eq!(
        CellOutboxPayload::ItemUsed { type_id: 0, target_id: 0 }.event_type(),
        "item_used",
    );
}

#[test]
fn row_to_message_builds_item_used() {
    let row = OutboxRow {
        id: 99,
        entity_id: 1234,
        event_type: "item_used".to_string(),
        payload: sqlx::types::Json(CellOutboxPayload::ItemUsed {
            type_id: 19,
            target_id: 5,
        }),
        attempts: 0,
    };
    let msg = row_to_message(&row).expect("known event_type should produce message");
    match msg {
        BaseToCellMsg::ItemUsed { entity_id, type_id, target_id } => {
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
            type_id: 0,
            target_id: 0,
        }),
        attempts: 0,
    };
    assert!(row_to_message(&row).is_none());
}

#[test]
fn payload_kind_tags_are_stable_for_all_variants() {
    // The kind tag is the persistence contract — changing one breaks
    // every in-flight row on existing databases. Pin them so a future
    // rename surfaces here, not in production.
    assert_eq!(
        CellOutboxPayload::ItemUsed { type_id: 0, target_id: 0 }.event_type(),
        "item_used",
    );
    assert_eq!(
        CellOutboxPayload::InventoryItemGranted {
            item_id: 0, container_id: 0, slot_id: 0, quantity: 0,
        }.event_type(),
        "inventory_item_granted",
    );
    assert_eq!(
        CellOutboxPayload::InventoryItemRemoved {
            item_id: 0, source_container_id: 0,
        }.event_type(),
        "inventory_item_removed",
    );
}

#[test]
fn inventory_item_granted_roundtrips() {
    let p = CellOutboxPayload::InventoryItemGranted {
        item_id: 100_001,
        container_id: 1,
        slot_id: 5,
        quantity: 3,
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: CellOutboxPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(back, p);
}

#[test]
fn inventory_item_removed_roundtrips() {
    let p = CellOutboxPayload::InventoryItemRemoved {
        item_id: 100_002,
        source_container_id: 3,
    };
    let json = serde_json::to_string(&p).unwrap();
    let back: CellOutboxPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(back, p);
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
            entity_id, item_id, container_id, slot_id, quantity,
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
            entity_id, item_id, source_container_id,
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
            type_id: 0, target_id: 0,
        }),
        attempts: 0,
    };
    assert!(row_to_message(&row).is_none());
}

#[test]
fn drain_stats_default_is_all_zero() {
    // The drainer's "no work" path returns `Default` and the periodic-log
    // gate suppresses on all-zero. Pin the default values so a future
    // refactor doesn't accidentally turn an empty drain into spam.
    let s = DrainStats::default();
    assert_eq!(s.delivered, 0);
    assert_eq!(s.skipped_bad, 0);
    assert_eq!(s.send_failed, 0);
}
