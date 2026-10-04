//! `BaseToCellMsg::EntityLabelsAt` (NT-40): the telemetry ingest's question
//! "who held this slot at this server time", answered through the real
//! dispatch from the live entity and the departed rings.

use std::time::{Duration, SystemTime};

use super::*;
use tokio::sync::oneshot;

use crate::cell::messages::{EntityLabelQuery, ENTITY_LABEL_QUERY_CAP};

const SLOT: u32 = 100;

fn make_manager() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();
    mgr
}

fn occupy(mgr: &mut SpaceManager, name: &str, created: SystemTime) {
    mgr.create_entity(SLOT, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(SLOT).unwrap();
    e.character_name = Some(name.to_string());
    e.created_at = created;
}

async fn ask(mgr: &mut SpaceManager, queries: Vec<EntityLabelQuery>) -> Vec<Option<&'static str>> {
    let (tx, _rx) = mpsc::channel::<CellToBaseMsg>(8);
    let (reply_tx, reply_rx) = oneshot::channel();
    handle_base_message(
        BaseToCellMsg::EntityLabelsAt { queries, reply_tx },
        &tx,
        mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    reply_rx.await.expect("EntityLabelsAt must always reply")
}

#[tokio::test]
async fn names_each_query_after_whoever_held_the_slot_then() {
    let mut mgr = make_manager();
    let t0 = SystemTime::now() - Duration::from_secs(120);
    occupy(&mut mgr, "Daniel", t0);
    let space_id = mgr.get_entity_space_id(SLOT).unwrap();
    mgr.destroy_entity(SLOT);
    occupy(&mut mgr, "Vala", SystemTime::now());

    let q = |at| EntityLabelQuery {
        space_id,
        entity_id: SLOT,
        at,
    };
    let labels = ask(
        &mut mgr,
        vec![
            q(t0 + Duration::from_secs(60)),
            q(SystemTime::now() + Duration::from_secs(1)),
            q(t0 - Duration::from_secs(1)),
        ],
    )
    .await;
    assert_eq!(
        labels,
        [Some("Daniel"), Some("Vala"), None],
        "a time in Daniel's lifetime names Daniel though Vala holds the slot now; \
         a time before anyone held it names no one"
    );
}

#[tokio::test]
async fn queries_past_the_cap_are_unnamed() {
    let mut mgr = make_manager();
    occupy(
        &mut mgr,
        "Teal'c",
        SystemTime::now() - Duration::from_secs(5),
    );
    let space_id = mgr.get_entity_space_id(SLOT).unwrap();
    let query = EntityLabelQuery {
        space_id,
        entity_id: SLOT,
        at: SystemTime::now(),
    };
    let labels = ask(&mut mgr, vec![query; ENTITY_LABEL_QUERY_CAP + 2]).await;
    assert_eq!(
        labels.len(),
        ENTITY_LABEL_QUERY_CAP + 2,
        "one answer per query"
    );
    assert_eq!(labels[ENTITY_LABEL_QUERY_CAP - 1], Some("Teal'c"));
    assert_eq!(labels[ENTITY_LABEL_QUERY_CAP], None);
}
