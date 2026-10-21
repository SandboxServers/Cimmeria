//! Pure serde + event_type contract tests for `CellOutboxPayload`.
//! No database access — exercises the `kind` tag, `event_type()` strings,
//! roundtrip stability, and the `DrainStats::default()` baseline.

use super::super::*;

#[test]
fn payload_serializes_with_kind_tag() {
    let p = CellOutboxPayload::ItemUsed {
        instance_id: 0,
        type_id: 42,
        target_id: 17,
    };
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
    let p = CellOutboxPayload::ItemUsed {
        instance_id: 0,
        type_id: -7,
        target_id: 0,
    };
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
        CellOutboxPayload::ItemUsed {
            instance_id: 0,
            type_id: 0,
            target_id: 0
        }
        .event_type(),
        "item_used",
    );
}

/// `(position, persisted tag)` for each variant, as an exhaustive match with
/// no wildcard: adding a `CellOutboxPayload` variant fails to compile here
/// until it is given the next position and a tag. Positions are 0..VARIANTS.
fn pinned(p: &CellOutboxPayload) -> (usize, &'static str) {
    match p {
        CellOutboxPayload::ItemUsed { .. } => (0, "item_used"),
        CellOutboxPayload::InventoryItemGranted { .. } => (1, "inventory_item_granted"),
        CellOutboxPayload::InventoryItemRemoved { .. } => (2, "inventory_item_removed"),
    }
}

/// Keep equal to the number of arms in `pinned`.
const VARIANTS: usize = 3;

#[test]
fn persistence_contract_strings_are_stable_for_all_variants() {
    // Each variant has TWO persistence contracts that must agree:
    //   - `event_type()` returns the string written to the
    //     `cell_event_outbox.event_type` VARCHAR column.
    //   - serde's `tag = "kind"` writes the same string into the
    //     payload JSONB body.
    // `row_to_message` matches on (event_type, payload) so both must
    // line up; pin them together here. Changing either string breaks
    // every in-flight row on existing databases.
    let cases = [
        CellOutboxPayload::ItemUsed {
            instance_id: 0,
            type_id: 0,
            target_id: 0,
        },
        CellOutboxPayload::InventoryItemGranted {
            item_id: 0,
            container_id: 0,
            slot_id: 0,
            quantity: 0,
        },
        CellOutboxPayload::InventoryItemRemoved {
            item_id: 0,
            source_container_id: 0,
        },
    ];

    // A new variant breaks `pinned` at compile time; this makes sure it is
    // also added to `cases`, so its tags are actually compared below.
    let mut seen: Vec<usize> = cases.iter().map(|v| pinned(v).0).collect();
    seen.sort_unstable();
    assert_eq!(
        seen,
        (0..VARIANTS).collect::<Vec<_>>(),
        "`cases` must hold exactly one value of every variant"
    );

    for variant in &cases {
        let (_, expected) = pinned(variant);
        assert_eq!(
            variant.event_type(),
            expected,
            "event_type() drift for {variant:?}",
        );
        let json = serde_json::to_value(variant).unwrap();
        assert_eq!(
            json["kind"], expected,
            "JSON `kind` tag drift for {variant:?}",
        );
    }
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
fn drain_stats_default_is_all_zero() {
    // The drainer's "no work" path returns `Default` and the periodic-log
    // gate suppresses on all-zero. Pin the default values so a future
    // refactor doesn't accidentally turn an empty drain into spam.
    let s = DrainStats::default();
    assert_eq!(s.delivered, 0);
    assert_eq!(s.skipped_bad, 0);
    assert_eq!(s.send_failed, 0);
}
