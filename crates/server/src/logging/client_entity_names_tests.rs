//! NT-40 end to end: a client row uploaded after its entity left and the
//! slot was reused is named after the entity that held the slot when the
//! client wrote the row. The ingest is the real admin-api path
//! (`replay_ndjson_named`), the names come from a real `SpaceManager` and
//! its departed-entity ring, and the cell's side of the round trip is the
//! same `answer_entity_labels` call the cell loop makes for a request on
//! its label channel.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use tokio::sync::mpsc;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use cimmeria_admin_api::routes::dev_session::TokenClaims;
use cimmeria_admin_api::routes::telemetry::{replay_ndjson_named, EntityLabelLink};
use cimmeria_services::cell::messages::EntityLabelsRequest;
use cimmeria_services::cell::space_manager::SpaceManager;

const SLOT: u32 = 100;

#[derive(Clone, Default)]
struct Rows(Arc<Mutex<Vec<BTreeMap<String, String>>>>);

struct Text<'a>(&'a mut BTreeMap<String, String>);

impl Visit for Text<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0
            .insert(field.name().to_string(), format!("{value:?}"));
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_string(), value.to_string());
    }
}

impl<S: Subscriber> Layer<S> for Rows {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        if event.metadata().target() != "client.native" {
            return;
        }
        let mut fields = BTreeMap::new();
        event.record(&mut Text(&mut fields));
        self.0.lock().unwrap().push(fields);
    }
}

fn occupy(mgr: &mut SpaceManager, name: &str, created: SystemTime) {
    mgr.create_entity(SLOT, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(SLOT).unwrap();
    e.character_name = Some(name.to_string());
    e.created_at = created;
}

/// The cell loop's side of the label channel, over `mgr`: the same
/// `answer_entity_labels` call the loop makes.
fn serve_cell(mgr: SpaceManager) -> EntityLabelLink {
    let (tx, mut rx) = mpsc::channel::<EntityLabelsRequest>(4);
    tokio::spawn(async move {
        while let Some(req) = rx.recv().await {
            mgr.answer_entity_labels(req);
        }
    });
    EntityLabelLink::new(tx)
}

fn line(ts: i64, target: &str, fields: String) -> String {
    format!(
        r#"{{"type":"client_native","ts_ms":{ts},"seq":1,"target":"{target}","level":"info","fields":{fields}}}"#
    )
}

async fn replay(
    claims: &TokenClaims,
    ndjson: &str,
    recv: SystemTime,
    cell: &EntityLabelLink,
) -> Vec<BTreeMap<String, String>> {
    let rows = Rows::default();
    let sub = tracing_subscriber::registry().with(rows.clone());
    {
        let _guard = tracing::subscriber::set_default(sub);
        replay_ndjson_named(claims, ndjson, recv, Some(cell))
            .await
            .unwrap();
    }
    let out = rows.0.lock().unwrap().clone();
    out
}

#[tokio::test]
async fn a_row_uploaded_after_its_slot_was_reused_names_the_original_occupant() {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="0" MaxX="100" MinY="0" MaxY="100" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();

    // Daniel held the slot for the last two minutes, left, and Vala took it.
    occupy(
        &mut mgr,
        "Daniel",
        SystemTime::now() - Duration::from_secs(120),
    );
    let space_id = mgr.get_entity_space_id(SLOT).unwrap();
    mgr.destroy_entity(SLOT);
    occupy(&mut mgr, "Vala", SystemTime::now());
    assert_eq!(mgr.entity_label(SLOT), Some("Vala"));
    let cell = serve_cell(mgr);
    // Row times are whole milliseconds; let the clock leave the millisecond
    // of the hand-over so "after the upload" is after it.
    tokio::time::sleep(Duration::from_millis(5)).await;

    let claims = TokenClaims {
        iss: "cimmeria-server".into(),
        sub: "install-nt40".into(),
        sid: "session-nt40-recycled-slot".into(),
        iat: 0,
        exp: i64::MAX,
        scope: vec!["telemetry.write".into()],
        kind: Some("lab".into()),
    };

    // The chunk the client uploads now, written over the last minute while
    // Daniel held the slot. Its clock is years off the server's; its newest
    // row was written just before the upload.
    let client_now: i64 = 1_700_000_000_000;
    let late = [
        line(
            client_now - 61_000,
            "client.entity.enter",
            format!(r#"{{"entity_id":{SLOT},"space_id":{space_id}}}"#),
        ),
        line(
            client_now - 60_000,
            "client.ability.recv",
            format!(r#"{{"source_id":{SLOT},"method_index":14}}"#),
        ),
        line(client_now, "client.engine.tick", "{}".to_string()),
    ]
    .join("\n");
    let rows = replay(&claims, &late, SystemTime::now(), &cell).await;
    assert_eq!(rows.len(), 3);
    assert_eq!(
        rows[1].get("source_name").map(String::as_str),
        Some("Daniel"),
        "the row was written while Daniel held the slot; naming it from the \
         receive time would pin Daniel's row on Vala: {:?}",
        rows[1]
    );
    assert_eq!(rows[1]["source_id"], SLOT.to_string());
    assert_eq!(rows[1]["names_source"], "client_claimed");

    // The next chunk, written after the hand-over, names Vala.
    let fresh = line(
        client_now + 1_000,
        "client.ability.recv",
        format!(r#"{{"target_id":{SLOT},"method_index":14}}"#),
    );
    let rows = replay(
        &claims,
        &fresh,
        SystemTime::now() + Duration::from_secs(1),
        &cell,
    )
    .await;
    assert_eq!(
        rows[0].get("target_name").map(String::as_str),
        Some("Vala"),
        "written after the hand-over"
    );
}
