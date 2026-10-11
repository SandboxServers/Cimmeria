//! CH-13 (D-CH9): `.spawnrandom` reports the ids it spawned, through the lab
//! console tee (`server_console_exec`) and in-world chat alike.
//!
//! Drives the real cell side end to end: `LabConsoleExec` (or the in-world
//! console) runs the command, the test plays the base by answering each
//! `GmSpawnNpc` with `GmSpawnNpcReady` / `GmSpawnNpcFailed` through
//! `handle_base_message`, and the 1 Hz batch tick closes a batch whose
//! copies did not all answer.

use std::time::Duration;

use tokio::sync::oneshot;
use tokio::sync::oneshot::error::TryRecvError;

use super::gm_spawn_ready::{gm_record, spawned_feedback_text};
use super::*;
use crate::cell::console::spawn_batch::spawn_batch_tick;
use crate::cell::messages::LabConsoleResult;

const GM: u32 = 1;
const TEMPLATE: i32 = 4242;

fn make_manager() -> (SpaceManager, u32) {
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
    if let Some(e) = mgr.get_entity_mut(GM) {
        e.is_player = true;
        e.access_level = 2;
    }
    mgr.connect_entity(GM);
    let space_id = mgr.get_entity(GM).unwrap().space_id.0 as u32;
    (mgr, space_id)
}

/// Run `line` through the lab console tee; returns the (possibly still
/// pending) reply.
async fn lab_exec(
    mgr: &mut SpaceManager,
    line: &str,
    tx: &mpsc::Sender<CellToBaseMsg>,
) -> oneshot::Receiver<LabConsoleResult> {
    let (reply_tx, reply_rx) = oneshot::channel();
    handle_base_message(
        BaseToCellMsg::LabConsoleExec {
            entity_id: GM,
            line: line.to_string(),
            reply_tx,
        },
        tx,
        mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
    reply_rx
}

/// The `GmSpawnNpc` requests the command sent to the base: (batch, position).
fn requests(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<(Option<u32>, [f32; 3])> {
    std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|m| match m {
            CellToBaseMsg::GmSpawnNpc {
                batch, position, ..
            } => Some((batch, position)),
            _ => None,
        })
        .collect()
}

/// Play the base: this copy resolved.
async fn ready(
    mgr: &mut SpaceManager,
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_id: u32,
    (batch, pos): (Option<u32>, [f32; 3]),
) {
    handle_base_message(
        BaseToCellMsg::GmSpawnNpcReady {
            record: Box::new(gm_record(TEMPLATE, pos)),
            space_id,
            requester_entity_id: GM,
            batch,
        },
        tx,
        mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
}

/// Play the base: this copy's template was not found.
async fn not_found(mgr: &mut SpaceManager, tx: &mpsc::Sender<CellToBaseMsg>, batch: u32) {
    handle_base_message(
        BaseToCellMsg::GmSpawnNpcFailed {
            requester_entity_id: GM,
            batch,
            template_id: TEMPLATE,
            reason: format!("template {TEMPLATE} not found"),
        },
        tx,
        mgr,
        &ChainEngine::new(),
        &[],
    )
    .await;
}

fn spawned_ids(mgr: &SpaceManager) -> String {
    let mut ids = mgr.all_npc_entity_ids();
    ids.sort_unstable();
    ids.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

/// Through the lab tee, the reply waits for the base's answers and then
/// lists every spawned id. Revert proof: answer the tee at once (the old
/// behaviour) and the reply carries only the "requested" line.
#[tokio::test]
async fn lab_spawnrandom_reply_lists_every_spawned_id() {
    let (mut mgr, space_id) = make_manager();
    let (tx, mut rx) = mpsc::channel(64);

    let mut reply = lab_exec(&mut mgr, ".spawnrandom 4242 3 3 3", &tx).await;
    let reqs = requests(&mut rx);
    assert_eq!(reqs.len(), 3, "one request per copy reached the base");
    assert!(
        matches!(reply.try_recv(), Err(TryRecvError::Empty)),
        "the reply must wait for the spawned ids"
    );

    for req in reqs {
        ready(&mut mgr, &tx, space_id, req).await;
    }

    let lines = reply.await.unwrap().expect("a GM command runs");
    assert_eq!(
        lines,
        vec![
            "spawnrandom: requested 3 x template 4242".to_string(),
            format!(
                "spawnrandom: spawned 3/3 x template 4242: ids {}",
                spawned_ids(&mgr)
            ),
        ]
    );
    assert_eq!(mgr.all_npc_entity_ids().len(), 3);
    assert_eq!(
        spawned_feedback_text(&mut rx, GM),
        None,
        "a batched copy sends no per-copy chat line"
    );
}

/// A copy that failed and one that never answered are reported as such once
/// the batch times out; the reply still arrives, with the ids that did spawn.
/// A copy that answers after the report gets its own chat line.
#[tokio::test(start_paused = true)]
async fn lab_spawnrandom_reports_partial_delivery_after_the_timeout() {
    let (mut mgr, space_id) = make_manager();
    let (tx, mut rx) = mpsc::channel(64);

    let mut reply = lab_exec(&mut mgr, ".spawnrandom 4242 3 3 3", &tx).await;
    let reqs = requests(&mut rx);
    let batch = reqs[0].0.expect("batched");
    ready(&mut mgr, &tx, space_id, reqs[0]).await;
    not_found(&mut mgr, &tx, batch).await;

    spawn_batch_tick(&tx, &mut mgr).await;
    assert!(
        matches!(reply.try_recv(), Err(TryRecvError::Empty)),
        "nothing is reported before the timeout"
    );

    tokio::time::advance(Duration::from_secs(6)).await;
    spawn_batch_tick(&tx, &mut mgr).await;

    let lines = reply.await.unwrap().expect("a GM command runs");
    assert_eq!(
        lines.last().map(String::as_str),
        Some(
            format!(
                "spawnrandom: spawned 1/3 x template 4242: ids {}; \
                 1 failed (template 4242 not found); 1 timed out",
                spawned_ids(&mgr)
            )
            .as_str()
        )
    );

    ready(&mut mgr, &tx, space_id, reqs[2]).await;
    let late = spawned_feedback_text(&mut rx, GM).expect("a late copy is reported");
    assert!(
        late.ends_with("after its .spawnrandom report"),
        "got {late}"
    );
}

/// In-world, the summary is the GM's chat line.
#[tokio::test]
async fn in_world_spawnrandom_sends_the_summary_as_feedback() {
    let (mut mgr, space_id) = make_manager();
    let (tx, mut rx) = mpsc::channel(64);

    crate::cell::console::handle_console_command(
        GM,
        ".spawnrandom 4242 3 3 2",
        &tx,
        &mut mgr,
        &ChainEngine::new(),
    )
    .await;
    let reqs = requests(&mut rx);
    for req in reqs {
        ready(&mut mgr, &tx, space_id, req).await;
    }

    assert_eq!(
        spawned_feedback_text(&mut rx, GM),
        Some(format!(
            "spawnrandom: spawned 2/2 x template 4242: ids {}",
            spawned_ids(&mgr)
        ))
    );
}
