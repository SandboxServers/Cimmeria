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
    // build doesn't know. Drainer must skip it (logged via warn), not
    // panic / mis-dispatch.
    let row = OutboxRow {
        id: 100,
        entity_id: 1,
        event_type: "future_event_v2".to_string(),
        payload: sqlx::types::Json(CellOutboxPayload::ItemUsed {
            type_id: 0,
            target_id: 0,
        }),
    };
    assert!(row_to_message(&row).is_none());
}
