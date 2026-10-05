//! The telemetry ingest's label channel in the real cell loop (NT-40): its
//! questions are answered, but never ahead of a queued gameplay message.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::{mpsc, oneshot, Notify};

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::messages::{
    BaseToCellMsg, CellToBaseMsg, EntityLabelQuery, EntityLabelsRequest, LabQuery, LabQueryResult,
};
use crate::cell::space_manager::SpaceManager;

const SLOT: u32 = 100;

fn manager_with_daniel() -> (SpaceManager, u32) {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Agnos" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(SLOT, "Agnos", [0.0; 3], [0.0; 3])
        .unwrap();
    let e = mgr.get_entity_mut(SLOT).unwrap();
    e.character_name = Some("Daniel".into());
    e.created_at = SystemTime::now() - Duration::from_secs(10);
    let space = mgr.get_entity_space_id(SLOT).unwrap();
    (mgr, space)
}

/// Gameplay messages queued with a label question are all handled before
/// it. The last of thirty queued gameplay messages destroys the entity the
/// question is about, and the question is for a moment after that, so the
/// answer tells the order apart: `None` if the destroy ran first, Daniel if
/// the question overtook it. Without the gameplay-first guard the loop
/// picks among ready arms at random and the question overtakes one of the
/// thirty in all but a vanishing fraction of runs.
#[tokio::test]
async fn a_label_question_waits_for_queued_gameplay_messages() {
    let (mgr, space_id) = manager_with_daniel();
    let (base_tx, mut base_rx) = mpsc::channel::<BaseToCellMsg>(256);
    let (labels_tx, labels_rx) = mpsc::channel::<EntityLabelsRequest>(4);
    let (cell_tx, _cell_rx) = mpsc::channel::<CellToBaseMsg>(256);

    for _ in 0..29 {
        let (reply_tx, _reply_rx) = oneshot::channel::<LabQueryResult>();
        base_tx
            .try_send(BaseToCellMsg::LabQuery {
                query: LabQuery::EntityGet { entity_id: SLOT },
                reply_tx,
            })
            .unwrap();
    }
    base_tx
        .try_send(BaseToCellMsg::DestroyEntity { entity_id: SLOT })
        .unwrap();
    let (reply_tx, label_rx) = oneshot::channel();
    labels_tx
        .try_send(EntityLabelsRequest {
            queries: vec![EntityLabelQuery {
                space_id,
                entity_id: SLOT,
                at: SystemTime::now() + Duration::from_secs(60),
            }],
            reply_tx,
        })
        .unwrap();

    let shutdown = Arc::new(Notify::new());
    let stop = shutdown.clone();
    let cell = tokio::spawn(async move {
        let mut labels_rx = Some(labels_rx);
        super::super::message_loop::run_cell_loop(
            &mut base_rx,
            &mut labels_rx,
            &cell_tx,
            mgr,
            ChainEngine::new(),
            None,
            Vec::new(),
            stop,
        )
        .await;
    });

    let labels = tokio::time::timeout(Duration::from_secs(5), label_rx)
        .await
        .expect("the loop answers label questions")
        .unwrap();
    assert_eq!(
        labels,
        [None],
        "the label question was answered before the queued destroy ran"
    );
    shutdown.notify_one();
    cell.await.unwrap();
}
