//! NT-40: a replayed `client.native` row carries the names of its IDs, and
//! an entity ID is named after whoever held the slot when the client wrote
//! the row, not when the server received it. Each assertion reads the
//! re-emitted event's own fields, which is what SigNoz sees.

use std::time::{Duration, SystemTime};

use serde_json::json;
use tokio::sync::mpsc;

use cimmeria_names::{NameBook, Table};
use cimmeria_services::cell::messages::EntityLabelsRequest;

use super::dto::ClientNativeEvent;
use super::entity_labels::EntityLabelLink;
use super::replay::replay_ndjson_named;
use super::replay_native::replay_client_native_named;
use super::replay_tests::{capture, capture_async, claims};

/// IDs no seed row uses.
const ITEM: i64 = 990_401;
const ABILITY: i64 = 990_402;
const ENTITY: u32 = 4123;

fn book() -> NameBook {
    let mut b = NameBook::empty();
    b.insert(Table::Items, ITEM, "NT40 Naquadah Shard");
    b.insert(Table::Abilities, ABILITY, "NT40 Staff Blast");
    b
}

fn row(target: &str, level: &str, fields: serde_json::Value) -> ClientNativeEvent {
    ClientNativeEvent {
        ts_ms: 1_700_000_000_000,
        seq: 1,
        target: target.into(),
        level: level.into(),
        fields: fields.as_object().cloned().unwrap_or_default(),
    }
}

fn jaffa(id: u32) -> Option<&'static str> {
    (id == ENTITY).then_some("Jaffa Guard")
}

/// The packet's fixture: one row with an entity ID, an item ID and an
/// address comes back with all three names, next to the IDs.
#[test]
fn a_fixture_row_carries_entity_item_and_address_names() {
    let e = row(
        "client.ability.recv",
        "debug",
        json!({
            "entity_id": ENTITY,
            "item_type_id": ITEM,
            "address": "0x01576f90",
            "ability_id": ABILITY,
            "method_index": 14,
            "type_id": 4,
            "msg_id": 0x8e,
        }),
    );
    let book = book();
    let rows = capture(|| replay_client_native_named(&claims(Some("lab")), e, &book, jaffa));
    let f = &rows[0].fields;
    assert_eq!(f["entity_id"], ENTITY.to_string());
    assert_eq!(f["entity_name"], "Jaffa Guard");
    assert_eq!(f["item_type_id"], ITEM.to_string());
    assert_eq!(f["item_name"], "NT40 Naquadah Shard");
    assert_eq!(f["address"], "0x01576f90");
    assert_eq!(f["address_name"], "Mercury::Channel::send");
    assert_eq!(f["ability_name"], "NT40 Staff Blast");
    assert_eq!(f["class_id"], "4");
    assert_eq!(f["class_name"], "SGWMob");
    assert_eq!(f["method_index"], "14");
    assert_eq!(f["method_name"], "onEffectResults");
    assert_eq!(f["msg_name"], "entityMethod");
}

/// An address the table doesn't list keeps its value and gets no name: no
/// `""`, no guess from the nearest symbol.
#[test]
fn an_unknown_address_passes_through_unnamed() {
    let e = row(
        "client.os.exception",
        "error",
        json!({ "address": "0x01576f91", "code": "0xc0000005" }),
    );
    let rows = capture(|| replay_client_native_named(&claims(None), e, &book(), |_| None));
    let f = &rows[0].fields;
    assert_eq!(f["address"], "0x01576f91");
    assert!(!f.contains_key("address_name"), "{f:?}");
}

/// A status row (no game IDs) still names its address: the hook install
/// rows are where most addresses come from.
#[test]
fn a_status_row_names_its_address() {
    let e = row(
        "client.hooks.inline.installed",
        "info",
        json!({ "hook": "channel_send", "address": "0x01576F90" }),
    );
    let rows = capture(|| replay_client_native_named(&claims(None), e, &book(), |_| None));
    assert_eq!(rows[0].fields["address_name"], "Mercury::Channel::send");
}

/// An entity the cell can't name, an ability the book doesn't know: the
/// IDs stay, the names are left out.
#[test]
fn unresolved_ids_keep_their_values_without_names() {
    let e = row(
        "client.ability.recv",
        "debug",
        json!({ "target_id": 77, "ability_id": 12_345_678 }),
    );
    let rows = capture(|| replay_client_native_named(&claims(None), e, &book(), jaffa));
    let f = &rows[0].fields;
    assert_eq!(f["target_id"], "77");
    assert!(!f.contains_key("target_name"));
    assert_eq!(f["ability_id"], "12345678");
    assert!(!f.contains_key("ability_name"));
}

/// The client sent `client.net.out`'s message, so its id is read from the
/// server's interface; every other row's from the client's.
#[test]
fn a_msg_id_is_named_by_its_direction() {
    let out = row("client.net.out", "debug", json!({ "msg_id": 0x80 }));
    let inbound = row(
        "client.mercury.entity_method",
        "debug",
        json!({ "msg_id": 0x80 }),
    );
    let rows = capture(|| {
        replay_client_native_named(&claims(None), out, &book(), |_| None);
        replay_client_native_named(&claims(None), inbound, &book(), |_| None);
    });
    assert_eq!(rows[0].fields["msg_name"], "cellMethod");
    assert_eq!(rows[1].fields["msg_name"], "entityMethod");
}

/// Without the row's entity type, a method index is named only where every
/// in-world type agrees: 27-31 differ between SGWPlayer and SGWMob.
#[test]
fn a_method_index_without_a_type_is_named_only_where_types_agree() {
    let agreed = row(
        "client.ability.recv",
        "debug",
        json!({ "method_index": 14 }),
    );
    let split = row(
        "client.ability.recv",
        "debug",
        json!({ "method_index": 27 }),
    );
    let rows = capture(|| {
        replay_client_native_named(&claims(None), agreed, &book(), |_| None);
        replay_client_native_named(&claims(None), split, &book(), |_| None);
    });
    assert_eq!(rows[0].fields["method_name"], "onEffectResults");
    assert!(!rows[1].fields.contains_key("method_name"));
}

/// A stand-in cell with one slot that changed hands at `handover`: Daniel
/// before, Vala from then on. The real lookup is the cell's
/// (`entity_labels_at`, tested in `cimmeria-cell-world`); this one checks
/// the time the ingest asks about.
fn fake_cell(space_id: u32, handover: SystemTime) -> EntityLabelLink {
    let (tx, mut rx) = mpsc::channel::<EntityLabelsRequest>(4);
    tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            let labels = req
                .queries
                .iter()
                .map(|q| {
                    (q.space_id == space_id && q.entity_id == ENTITY)
                        .then_some(if q.at < handover { "Daniel" } else { "Vala" })
                })
                .collect();
            let _ = req.reply_tx.send(labels);
        }
    });
    EntityLabelLink::new(tx)
}

fn line(ts: i64, target: &str, fields: serde_json::Value) -> String {
    json!({
        "type": "client_native", "ts_ms": ts, "seq": 1,
        "target": target, "level": "info", "fields": fields,
    })
    .to_string()
}

/// A chunk written a minute before its upload, about a slot that changed
/// hands 30 s before the upload, names the occupant at the time it was
/// written. Naming by receive time would name the new occupant, and the
/// client's clock here is years off the server's, so its raw time would
/// name no one. The next chunk, written after the hand-over, names the new
/// occupant.
#[tokio::test]
async fn a_late_chunk_names_the_slot_holder_when_it_was_written() {
    const SPACE: u32 = 65536;
    let recv = SystemTime::now();
    let link = fake_cell(SPACE, recv - Duration::from_secs(30));
    // The client's clock runs years behind the server's.
    let client_now: i64 = 1_700_000_000_000;
    let late = [
        line(
            client_now - 90_000,
            "client.entity.create",
            json!({ "entity_id": ENTITY, "space_id": SPACE, "type_id": 2 }),
        ),
        line(
            client_now - 60_000,
            "client.ability.recv",
            json!({ "source_id": ENTITY, "method_index": 14 }),
        ),
        // The newest row, written just before the upload.
        line(client_now, "client.engine.tick", json!({})),
    ]
    .join("\n");
    let fresh = line(
        client_now + 1_000,
        "client.ability.recv",
        json!({ "target_id": ENTITY, "method_index": 14 }),
    );
    // A session id of its own: the clock offset is kept per session.
    let mut claims = claims(None);
    claims.sid = "nt40-late-chunk".into();

    let (counts, rows) =
        capture_async(replay_ndjson_named(&claims, &late, recv, Some(&link))).await;
    assert_eq!(counts.unwrap().accepted, 3);
    assert_eq!(rows[0].fields["entity_name"], "Daniel");
    assert_eq!(
        rows[1].fields["source_name"], "Daniel",
        "written 60 s before the upload, 30 s before the slot changed hands"
    );
    assert_eq!(rows[1].fields["names_source"], "client_claimed");

    let next = recv + Duration::from_secs(1);
    let (_, rows) = capture_async(replay_ndjson_named(&claims, &fresh, next, Some(&link))).await;
    assert_eq!(
        rows[0].fields["target_name"], "Vala",
        "written after the hand-over"
    );
}

/// The shape follows the DLL target, not the row's fields: a governor
/// rollup that also carries a game key keeps its rollup attributes.
#[test]
fn a_status_target_keeps_its_status_keys_whatever_else_it_carries() {
    let mut e = row(
        "client.telemetry.rollup",
        "info",
        json!({ "rollup_target": "client.lua.pcall", "count": 7, "type_id": 2, "msg_id": 1 }),
    );
    e.seq = 2;
    let rows = capture(|| replay_client_native_named(&claims(None), e, &book(), |_| None));
    let f = &rows[0].fields;
    assert_eq!(f["rollup_count"], "7");
    assert_eq!(f["rollup_target"], "client.lua.pcall");
    assert!(!f.contains_key("class_id"), "a status row has no ID pairs");
    assert_eq!(f["names_source"], "client_claimed");
}
