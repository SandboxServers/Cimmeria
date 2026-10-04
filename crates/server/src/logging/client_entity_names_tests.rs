//! NT-40 end to end: a client row uploaded after its entity left and the
//! slot was reused is named after the entity that held the slot when the
//! client wrote the row. The ingest is the real admin-api path
//! (`replay_ndjson_named`), the names come from a real `SpaceManager` and
//! its departed-entity ring, and the cell's side of the round trip is the
//! same `entity_labels_at` call the cell loop makes for
//! `BaseToCellMsg::EntityLabelsAt`.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use tokio::sync::mpsc;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer, SubscriberExt};

use cimmeria_admin_api::routes::dev_session::TokenClaims;
use cimmeria_admin_api::routes::telemetry::replay_ndjson_named;
use cimmeria_services::cell::messages::BaseToCellMsg;
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

/// The cell loop's side of `EntityLabelsAt`, over `mgr`.
fn serve_cell(mgr: SpaceManager) -> mpsc::Sender<BaseToCellMsg> {
    let (tx, mut rx) = mpsc::channel::<BaseToCellMsg>(4);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let BaseToCellMsg::EntityLabelsAt { queries, reply_tx } = msg {
                let _ = reply_tx.send(mgr.entity_labels_at(&queries));
            }
        }
    });
    tx
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
    // of the hand-over so "just before the upload" is after it.
    tokio::time::sleep(Duration::from_millis(5)).await;

    // The chunk the client uploads now. Its clock is years off the
    // server's; its newest row was written just before the upload, and the
    // row about the slot a minute earlier, while Daniel held it.
    let client_now: i64 = 1_700_000_000_000;
    let line = |ts: i64, target: &str, fields: String| {
        format!(
            r#"{{"type":"client_native","ts_ms":{ts},"seq":1,"target":"{target}","level":"info","fields":{fields}}}"#
        )
    };
    let ndjson = [
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
        line(
            client_now,
            "client.ability.recv",
            format!(r#"{{"target_id":{SLOT},"method_index":14}}"#),
        ),
    ]
    .join("\n");
    let claims = TokenClaims {
        iss: "cimmeria-server".into(),
        sub: "install-nt40".into(),
        sid: "session-nt40-recycled-slot".into(),
        iat: 0,
        exp: i64::MAX,
        scope: vec!["telemetry.write".into()],
        kind: Some("lab".into()),
    };

    let rows = Rows::default();
    let sub = tracing_subscriber::registry().with(rows.clone());
    let counts = {
        let _guard = tracing::subscriber::set_default(sub);
        replay_ndjson_named(&claims, &ndjson, SystemTime::now(), Some(&cell))
            .await
            .unwrap()
    };
    assert_eq!(counts.accepted, 3);
    let rows = rows.0.lock().unwrap().clone();
    assert_eq!(
        rows[1].get("source_name").map(String::as_str),
        Some("Daniel"),
        "the row was written while Daniel held the slot; naming it from the \
         receive time would pin Daniel's row on Vala: {:?}",
        rows[1]
    );
    assert_eq!(rows[1]["source_id"], SLOT.to_string());
    assert_eq!(
        rows[2].get("target_name").map(String::as_str),
        Some("Vala"),
        "the newest row was written after the hand-over"
    );
}
