//! Shutdown flush (TG-DB-02, D-TG15): save every in-world player's position
//! and dirty bandolier ammo before the cell loop exits.
//!
//! Called from the `shutdown` arm of the message loop, before
//! `shutdown_report`. Builds one `ShutdownFlushRequest` for every player with
//! a `player_id`, hands it to the base (which writes the players in parallel
//! and answers with those saved) and waits until the deadline plus a short
//! grace for the answer. Players the answer leaves out (a failed write, the
//! deadline, a base that never answered) are returned unsaved, and
//! `shutdown_report` warns once for each.
//!
//! Rows: INFO `shutdown_flush_started` (`players_in_world`), INFO
//! `shutdown_flush_finished` (`saved`, `missed`, `elapsed_ms`).

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use tokio::sync::{mpsc, oneshot};

use crate::cell::messages::{
    CellToBaseMsg, ShutdownAmmoSlot, ShutdownFlushRequest, ShutdownPlayerSave,
};
use crate::cell::space_manager::SpaceManager;

/// How long past the base's deadline the cell keeps waiting for the answer:
/// the base aborts its writes at the deadline and answers at once, so this
/// only covers a base that is stuck on an earlier message.
const REPLY_GRACE: Duration = Duration::from_secs(1);

/// Save every in-world player through the base, waiting at most `deadline`
/// (plus [`REPLY_GRACE`]). Returns the entity IDs of the players saved; their
/// ammo dirty markers are cleared.
pub(super) async fn flush_players_on_shutdown(
    tx: &mpsc::Sender<CellToBaseMsg>,
    space_mgr: &mut SpaceManager,
    deadline: Duration,
) -> HashSet<u32> {
    let started = Instant::now();
    let mut saves = Vec::new();
    // player_id -> entity_id, to map the answer back.
    let mut entity_of: HashMap<i32, u32> = HashMap::new();
    for eid in space_mgr.all_player_entity_ids() {
        let Some(e) = space_mgr.get_entity(eid) else {
            continue;
        };
        let Some(player_id) = e.player_id else {
            continue;
        };
        entity_of.insert(player_id, eid);
        // A player in no space has no world to save; it is reported missed.
        let Some(world_name) = space_mgr.get_entity_world_name(eid) else {
            continue;
        };
        let ammo = e
            .bandolier_ammo_dirty
            .iter()
            .filter_map(|slot_id| {
                e.bandolier_items.get(slot_id).map(|item| ShutdownAmmoSlot {
                    slot_id: *slot_id,
                    expected_instance_id: item.instance_id,
                    current_ammo: item.current_ammo,
                    cur_ammo_type: item.cur_ammo_type,
                })
            })
            .collect();
        saves.push(ShutdownPlayerSave {
            player_id,
            world_name,
            position: [e.position.x, e.position.y, e.position.z],
            ammo,
        });
    }

    let players_in_world = entity_of.len();
    tracing::info!(
        event = "shutdown_flush_started",
        players_in_world,
        deadline_ms = deadline.as_millis() as u64,
        "shutdown flush started"
    );

    let saved_ids = if saves.is_empty() {
        Vec::new()
    } else {
        request_saves(tx, saves, deadline).await
    };

    let mut saved = HashSet::with_capacity(saved_ids.len());
    for player_id in saved_ids {
        if let Some(&eid) = entity_of.get(&player_id) {
            if saved.insert(eid) {
                if let Some(e) = space_mgr.get_entity_mut(eid) {
                    e.bandolier_ammo_dirty.clear();
                }
            }
        }
    }

    tracing::info!(
        event = "shutdown_flush_finished",
        players_in_world,
        saved = saved.len(),
        missed = players_in_world - saved.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        "shutdown flush finished"
    );
    saved
}

/// Send the request and wait for the answer. Any failure (the base's channel
/// closed, the answer dropped, the wait timed out) saves nobody.
async fn request_saves(
    tx: &mpsc::Sender<CellToBaseMsg>,
    saves: Vec<ShutdownPlayerSave>,
    deadline: Duration,
) -> Vec<i32> {
    let base_deadline = tokio::time::Instant::now() + deadline;
    let (reply, answer) = oneshot::channel();
    let req = ShutdownFlushRequest {
        saves,
        deadline: base_deadline,
        reply,
    };
    let exchange = async {
        tx.send(CellToBaseMsg::ShutdownFlush(req))
            .await
            .map_err(|_| "base_channel_closed")?;
        answer.await.map_err(|_| "reply_dropped")
    };
    match tokio::time::timeout_at(base_deadline + REPLY_GRACE, exchange).await {
        Ok(Ok(ids)) => ids,
        Ok(Err(reason)) => {
            tracing::warn!(
                event = "shutdown_flush_failed",
                reason,
                "shutdown flush: the base did not take the request; nobody was saved"
            );
            Vec::new()
        }
        Err(_) => {
            tracing::warn!(
                event = "shutdown_flush_failed",
                reason = "deadline_reached",
                deadline_ms = deadline.as_millis() as u64,
                "shutdown flush: no answer from the base by the deadline; nobody counted saved"
            );
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use tracing::Level;

    fn mgr() -> SpaceManager {
        let mut mgr = SpaceManager::new(1);
        mgr.parse_spaces_xml(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" Instanced="false" MinX="-800" MaxX="800" MinY="-800" MaxY="800" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_startup_spaces(
            r#"<?xml version="1.0"?><Spaces><Space WorldName="Castle" /></Spaces>"#,
        )
        .unwrap();
        mgr.create_entity(1, "Castle", [10.0, 20.0, 30.0], [0.0; 3])
            .unwrap();
        let e = mgr.get_entity_mut(1).unwrap();
        e.player_id = Some(72);
        e.account_id = Some(6);
        mgr.connect_entity(1);
        mgr.spawn_npc(2, "Castle", [1.0, 0.0, 0.0], [0.0; 3])
            .unwrap();
        mgr
    }

    /// The base answers with the player: it is saved, the NPC is not part of
    /// the request, and both rows carry the counts.
    #[tokio::test]
    async fn answered_flush_marks_the_player_saved() {
        let mut mgr = mgr();
        let (tx, mut rx) = mpsc::channel(4);
        let base = tokio::spawn(async move {
            let Some(CellToBaseMsg::ShutdownFlush(req)) = rx.recv().await else {
                panic!("expected a ShutdownFlush");
            };
            assert_eq!(req.saves.len(), 1, "players only: {:?}", req.saves);
            assert_eq!(req.saves[0].player_id, 72);
            assert_eq!(req.saves[0].world_name, "Castle");
            assert_eq!(req.saves[0].position, [10.0, 20.0, 30.0]);
            req.reply.send(vec![72]).unwrap();
        });
        let capture = LogCapture::install();
        let saved = flush_players_on_shutdown(&tx, &mut mgr, Duration::from_secs(5)).await;
        base.await.unwrap();
        assert_eq!(saved, HashSet::from([1]));
        assert!(capture.all().iter().any(|c| c.level == Level::INFO
            && c.has_field("event", "shutdown_flush_started")
            && c.has_field("players_in_world", "1")));
        let finished = capture
            .all()
            .into_iter()
            .find(|c| c.has_field("event", "shutdown_flush_finished"))
            .expect("a finished row");
        assert!(finished.has_field("saved", "1"), "{finished:?}");
        assert!(finished.has_field("missed", "0"), "{finished:?}");
    }

    /// **Deadline guard, cell side.** A base that takes the request and never
    /// answers (a stuck save) is cut at the deadline: the flush returns with
    /// nobody saved, and the shutdown report then warns for the player.
    #[tokio::test]
    async fn unanswered_flush_is_cut_at_the_deadline_and_reported() {
        let mut mgr = mgr();
        let (tx, mut rx) = mpsc::channel(4);
        // Hold the request (and so its reply sender) without answering.
        let holder = tokio::spawn(async move {
            let held = rx.recv().await;
            tokio::time::sleep(Duration::from_secs(30)).await;
            drop(held);
        });
        let capture = LogCapture::install();
        let started = Instant::now();
        let saved = flush_players_on_shutdown(&tx, &mut mgr, Duration::from_millis(100)).await;
        assert!(
            started.elapsed() < REPLY_GRACE + Duration::from_secs(2),
            "the flush must give up at its deadline, took {:?}",
            started.elapsed()
        );
        assert!(saved.is_empty());
        holder.abort();

        super::super::shutdown_report::report_unsaved_on_shutdown(&mgr, &saved);
        let warns: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| {
                c.level == Level::WARN && c.has_field("event", "player_state_not_persisted")
            })
            .collect();
        assert_eq!(warns.len(), 1, "{warns:#?}");
        assert!(warns[0].has_field("player_id", "72"));
        assert!(
            capture
                .all()
                .iter()
                .any(|c| c.has_field("event", "shutdown_flush_finished")
                    && c.has_field("missed", "1"))
        );
    }
}
