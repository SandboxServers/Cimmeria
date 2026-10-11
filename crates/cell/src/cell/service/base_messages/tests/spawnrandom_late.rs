//! CH-13 review fixes: the `.spawnrandom` batch's edges. Late copies (lab
//! batches despawn them, in-world GMs are told), lines addressed to a GM
//! whose entity id now belongs to someone else, a copy the cell cannot place,
//! and a lab command whose requests the base channel refused.
//!
//! The happy path and the timeout are in `spawnrandom_batch.rs`.

use std::time::Duration;

use super::gm_spawn_ready::spawned_feedback_text;
use super::spawnrandom_batch::{
    lab_exec, make_manager, ready, requests, spawned_ids, GM, GM_PLAYER,
};
use super::*;
use crate::cell::console::spawn_batch::spawn_batch_tick;

/// Every feedback line sent to the GM.
fn gm_lines(rx: &mut mpsc::Receiver<CellToBaseMsg>) -> Vec<String> {
    std::iter::from_fn(|| spawned_feedback_text(rx, GM)).collect()
}

async fn in_world(mgr: &mut SpaceManager, line: &str, tx: &mpsc::Sender<CellToBaseMsg>) {
    crate::cell::console::handle_console_command(GM, line, tx, mgr, &ChainEngine::new()).await;
}

/// The GM logs out and another character gets its entity id.
fn reuse_gm_id(mgr: &mut SpaceManager) {
    mgr.get_entity_mut(GM).unwrap().player_id = Some(GM_PLAYER + 1);
}

/// In-world, a copy that answers after the summary keeps today's behaviour:
/// the NPC stays and the GM gets its own line.
#[tokio::test(start_paused = true)]
async fn in_world_late_copy_stays_and_gets_its_own_line() {
    let (mut mgr, space_id) = make_manager();
    let (tx, mut rx) = mpsc::channel(64);

    in_world(&mut mgr, ".spawnrandom 4242 3 3 2", &tx).await;
    let reqs = requests(&mut rx);
    ready(&mut mgr, &tx, space_id, reqs[0]).await;
    tokio::time::advance(Duration::from_secs(6)).await;
    spawn_batch_tick(&tx, &mut mgr).await;
    let summary = gm_lines(&mut rx);
    assert_eq!(summary.len(), 1, "got {summary:?}");
    assert!(summary[0].ends_with("; 1 timed out"), "got {summary:?}");

    ready(&mut mgr, &tx, space_id, reqs[1]).await;
    assert_eq!(mgr.all_npc_entity_ids().len(), 2, "the late NPC stays");
    let late = gm_lines(&mut rx);
    assert_eq!(late.len(), 1, "got {late:?}");
    assert!(
        late[0].ends_with("(template 4242) after its .spawnrandom report"),
        "got {late:?}"
    );
}

/// The summary goes only to the character that asked. Revert proof: send it
/// to the bare entity id (the packet's `send_gm_feedback(requester, ..)`)
/// and the new holder of the id gets it.
#[tokio::test]
async fn summary_is_dropped_when_the_gm_id_belongs_to_someone_else() {
    let (mut mgr, space_id) = make_manager();
    let (tx, mut rx) = mpsc::channel(64);

    in_world(&mut mgr, ".spawnrandom 4242 3 3 2", &tx).await;
    let reqs = requests(&mut rx);
    ready(&mut mgr, &tx, space_id, reqs[0]).await;
    reuse_gm_id(&mut mgr);
    ready(&mut mgr, &tx, space_id, reqs[1]).await;

    assert_eq!(gm_lines(&mut rx), Vec::<String>::new());
    assert_eq!(
        mgr.all_npc_entity_ids().len(),
        2,
        "the copies still spawned"
    );
}

/// Same for the late line. Revert proof: as above, for `late_copy`.
#[tokio::test(start_paused = true)]
async fn late_line_is_dropped_when_the_gm_id_belongs_to_someone_else() {
    let (mut mgr, space_id) = make_manager();
    let (tx, mut rx) = mpsc::channel(64);

    in_world(&mut mgr, ".spawnrandom 4242 3 3 2", &tx).await;
    let reqs = requests(&mut rx);
    ready(&mut mgr, &tx, space_id, reqs[0]).await;
    tokio::time::advance(Duration::from_secs(6)).await;
    spawn_batch_tick(&tx, &mut mgr).await;
    assert_eq!(gm_lines(&mut rx).len(), 1, "the summary reached the GM");

    reuse_gm_id(&mut mgr);
    ready(&mut mgr, &tx, space_id, reqs[1]).await;
    assert_eq!(gm_lines(&mut rx), Vec::<String>::new());
}

/// A copy the cell cannot place counts as failed, with its reason.
/// Revert proof: drop the batched branch of the placement `Err` arm and the
/// batch waits for the copy, so no summary arrives.
#[tokio::test]
async fn a_copy_the_cell_cannot_place_is_counted_failed() {
    let (mut mgr, space_id) = make_manager();
    let (tx, mut rx) = mpsc::channel(64);

    in_world(&mut mgr, ".spawnrandom 4242 3 3 2", &tx).await;
    let reqs = requests(&mut rx);
    ready(&mut mgr, &tx, space_id, reqs[0]).await;
    ready(&mut mgr, &tx, space_id + 1000, reqs[1]).await;

    assert_eq!(
        gm_lines(&mut rx),
        vec![format!(
            "spawnrandom: spawned 1/2 x template 4242: ids {}; \
             1 failed (could not place it in the space)",
            spawned_ids(&mgr)
        )]
    );
}

/// Through the lab tee, requests the base channel refused fail at once and
/// the reply arrives with them counted. Revert proof: stop counting them
/// (the packet's `break`) and the reply waits for the timeout.
#[tokio::test]
async fn lab_reply_counts_requests_the_base_channel_refused() {
    let (mut mgr, _space_id) = make_manager();
    let (tx, rx) = mpsc::channel(64);
    drop(rx);

    let mut reply = lab_exec(&mut mgr, ".spawnrandom 4242 3 3 3", &tx).await;

    let lines = reply
        .try_recv()
        .expect("the reply does not wait for copies the base never got")
        .expect("a GM command runs");
    assert_eq!(
        lines,
        vec![
            "spawnrandom: requested 3 x template 4242".to_string(),
            "spawnrandom: spawned 0/3 x template 4242: ids none; \
             3 failed (cell→base channel error)"
                .to_string(),
        ]
    );
    assert_eq!(mgr.all_npc_entity_ids().len(), 0);
}
