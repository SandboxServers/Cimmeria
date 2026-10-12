//! CH-13: the real cell loop closes a `.spawnrandom` batch nobody answers.
//! The lab console's reply is parked on the batch, so without the 1 Hz
//! `spawn_batch_tick` call in `message_loop.rs` a lost base reply would
//! leave `server_console_exec` waiting forever.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, Notify};

use cimmeria_content_engine::chain::ChainEngine;

use crate::cell::messages::{BaseToCellMsg, CellToBaseMsg};
use crate::cell::space_manager::SpaceManager;

const GM: u32 = 1;

fn manager_with_gm() -> SpaceManager {
    let mut mgr = SpaceManager::new(1);
    mgr.parse_spaces_xml(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_startup_spaces(
        r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
    )
    .unwrap();
    mgr.create_entity(GM, "Castle", [0.0; 3], [0.0; 3]).unwrap();
    let e = mgr.get_entity_mut(GM).unwrap();
    e.is_player = true;
    e.access_level = 2;
    mgr
}

/// The base never answers the copies; the loop's tick still closes the batch
/// and sends the parked reply, every copy timed out. Revert proof: remove
/// the `spawn_batch_tick` call from the loop's 1 Hz branch and the reply
/// never comes.
#[tokio::test(start_paused = true)]
async fn the_loop_closes_an_unanswered_batch_and_sends_the_lab_reply() {
    let (base_tx, mut base_rx) = mpsc::channel::<BaseToCellMsg>(16);
    // Kept open and never read: the requests reach "the base", which then
    // says nothing. A closed channel would fail the copies at once instead.
    let (cell_tx, _cell_rx) = mpsc::channel::<CellToBaseMsg>(256);

    let (reply_tx, reply_rx) = oneshot::channel();
    base_tx
        .try_send(BaseToCellMsg::LabConsoleExec {
            entity_id: GM,
            line: ".spawnrandom 4242 3 3 2".to_string(),
            reply_tx,
        })
        .unwrap();

    let shutdown = Arc::new(Notify::new());
    let stop = shutdown.clone();
    let cell = tokio::spawn(async move {
        super::super::message_loop::run_cell_loop(
            &mut base_rx,
            &mut None,
            &cell_tx,
            manager_with_gm(),
            ChainEngine::new(),
            None,
            Vec::new(),
            stop,
        )
        .await;
    });

    // Paused clock: the runtime jumps to each timer, so this is quick.
    let lines = tokio::time::timeout(Duration::from_secs(30), reply_rx)
        .await
        .expect("the loop's tick closes the batch and sends the reply")
        .unwrap()
        .expect("a GM command runs");
    assert_eq!(
        lines,
        vec![
            "spawnrandom: requested 2 x template 4242".to_string(),
            "spawnrandom: spawned 0/2 x template 4242: ids none; 2 timed out".to_string(),
        ]
    );
    cell.abort();
    let _ = shutdown;
}
