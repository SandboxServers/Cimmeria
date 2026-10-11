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
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};
// tokio's clock, so the deadline tests run on paused time.
use tokio::time::Instant;

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
        let world_name = space_mgr.get_entity_world_name(eid);
        let Some(e) = space_mgr.get_entity_mut(eid) else {
            continue;
        };
        let Some(player_id) = e.player_id else {
            continue;
        };
        entity_of.insert(player_id, eid);
        // A player in no space has no world to save; it is reported missed.
        let Some(world_name) = world_name else {
            continue;
        };
        let mut ammo = Vec::with_capacity(e.bandolier_ammo_dirty.len());
        let mut dropped = Vec::new();
        for &slot_id in &e.bandolier_ammo_dirty {
            match e.bandolier_items.get(&slot_id) {
                Some(item) => ammo.push(ShutdownAmmoSlot {
                    slot_id,
                    expected_instance_id: item.instance_id,
                    current_ammo: item.current_ammo,
                    cur_ammo_type: item.cur_ammo_type,
                }),
                None => dropped.push(slot_id),
            }
        }
        if !dropped.is_empty() {
            let id = e.identity();
            for slot_id in dropped {
                // Dirty but the slot holds no item (swapped out since the
                // ammo changed): nothing to save. Same row as TG-ITM-11's
                // logout flush.
                tracing::warn!(
                    event = "bandolier_slot_dropped",
                    reason = "no_item",
                    account_id = id.account_id,
                    account_name = id.account_name,
                    player_id = id.player_id,
                    player_name = id.player_name,
                    slot_id, // nt:id-only bandolier slot index, not a named object
                    "shutdown flush: dirty bandolier slot has no item; nothing to save"
                );
                e.bandolier_ammo_dirty.remove(&slot_id);
            }
        }
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
    let base_deadline = Instant::now() + deadline;
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

    /// Assert the flush missed player 72: one `shutdown_flush_failed` with
    /// `reason`, `missed = 1`, and the report's one WARN for the player.
    fn assert_missed_with(
        capture: &cimmeria_test_support::LogCaptureGuard,
        mgr: &SpaceManager,
        saved: &HashSet<u32>,
        reason: &str,
    ) {
        assert!(saved.is_empty(), "nobody saved: {saved:?}");
        let all = capture.all();
        assert!(
            all.iter().any(|c| c.level == Level::WARN
                && c.has_field("event", "shutdown_flush_failed")
                && c.has_field("reason", reason)),
            "a shutdown_flush_failed with reason {reason}: {all:#?}"
        );
        assert!(
            all.iter()
                .any(|c| c.has_field("event", "shutdown_flush_finished")
                    && c.has_field("missed", "1"))
        );
        super::super::shutdown_report::report_unsaved_on_shutdown(mgr, saved);
        let warns: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| {
                c.level == Level::WARN && c.has_field("event", "player_state_not_persisted")
            })
            .collect();
        assert_eq!(warns.len(), 1, "{warns:#?}");
        assert!(warns[0].has_field("player_id", "72"));
    }

    /// **Deadline guard, cell side.** A base that takes the request and never
    /// answers (a stuck save) is cut at the deadline plus [`REPLY_GRACE`]:
    /// the flush returns with nobody saved, and the shutdown report then
    /// warns for the player. Paused time: the clock only moves by `advance`,
    /// so "returned" here means "returned at that instant", not "eventually".
    #[tokio::test(start_paused = true)]
    async fn unanswered_flush_is_cut_at_the_deadline_and_reported() {
        let mut mgr = mgr();
        let (tx, mut rx) = mpsc::channel(4);
        // Hold the request (and so its reply sender) without answering.
        let holder = tokio::spawn(async move {
            let held = rx.recv().await;
            std::future::pending::<()>().await;
            drop(held);
        });
        let capture = LogCapture::install();
        let deadline = Duration::from_secs(10);
        let started = Instant::now();
        let saved = {
            let flush = flush_players_on_shutdown(&tx, &mut mgr, deadline);
            tokio::pin!(flush);
            // `timeout(ZERO, ..)` polls the flush once and reports whether it
            // finished, without letting the paused clock move.
            let poll_once = Duration::ZERO;
            assert!(
                tokio::time::timeout(poll_once, &mut flush).await.is_err(),
                "the request is sent and the flush waits"
            );
            // Just short of deadline + grace: still waiting.
            tokio::time::advance(deadline + REPLY_GRACE - Duration::from_millis(1)).await;
            assert!(
                tokio::time::timeout(poll_once, &mut flush).await.is_err(),
                "the flush must wait for the base until deadline + grace"
            );
            // At deadline + grace: it gives up.
            tokio::time::advance(Duration::from_millis(1)).await;
            tokio::time::timeout(poll_once, &mut flush)
                .await
                .expect("the flush must give up at deadline + grace")
        };
        assert_eq!(started.elapsed(), deadline + REPLY_GRACE);
        holder.abort();
        assert_missed_with(&capture, &mgr, &saved, "deadline_reached");
    }

    /// The base's channel is closed (its task already ended): the send
    /// fails at once, every player is missed, and no time passes.
    #[tokio::test(start_paused = true)]
    async fn closed_base_channel_misses_every_player_at_once() {
        let mut mgr = mgr();
        let (tx, rx) = mpsc::channel(4);
        drop(rx);
        let capture = LogCapture::install();
        let started = Instant::now();
        let saved = flush_players_on_shutdown(&tx, &mut mgr, Duration::from_secs(10)).await;
        assert_eq!(
            started.elapsed(),
            Duration::ZERO,
            "no wait on a closed channel"
        );
        assert_missed_with(&capture, &mgr, &saved, "base_channel_closed");
    }

    /// The base takes the request and drops the reply sender without
    /// answering: every player is missed, and the flush returns at once
    /// instead of waiting out the deadline.
    #[tokio::test(start_paused = true)]
    async fn dropped_reply_misses_every_player_at_once() {
        let mut mgr = mgr();
        let (tx, mut rx) = mpsc::channel(4);
        let base = tokio::spawn(async move {
            drop(rx.recv().await);
        });
        let capture = LogCapture::install();
        let started = Instant::now();
        let saved = flush_players_on_shutdown(&tx, &mut mgr, Duration::from_secs(10)).await;
        assert_eq!(
            started.elapsed(),
            Duration::ZERO,
            "no wait on a dropped reply"
        );
        base.await.unwrap();
        assert_missed_with(&capture, &mgr, &saved, "reply_dropped");
    }

    /// A dirty ammo slot with no bandolier item is left out of the request
    /// with a WARN `bandolier_slot_dropped`, and its marker is cleared even
    /// though the player was not saved; a slot that has an item stays dirty
    /// until a save lands.
    #[tokio::test]
    async fn dirty_slot_without_item_is_dropped_with_a_warn() {
        let mut mgr = mgr();
        mgr.get_entity_mut(1)
            .unwrap()
            .bandolier_ammo_dirty
            .insert(3);
        let (tx, mut rx) = mpsc::channel(4);
        let base = tokio::spawn(async move {
            let Some(CellToBaseMsg::ShutdownFlush(req)) = rx.recv().await else {
                panic!("expected a ShutdownFlush");
            };
            assert!(req.saves[0].ammo.is_empty(), "{:?}", req.saves[0].ammo);
            // Not saved: the marker must be cleared by the drop, not the save.
            req.reply.send(Vec::new()).unwrap();
        });
        let capture = LogCapture::install();
        let saved = flush_players_on_shutdown(&tx, &mut mgr, Duration::from_secs(5)).await;
        base.await.unwrap();
        assert!(saved.is_empty());
        assert!(
            mgr.get_entity(1).unwrap().bandolier_ammo_dirty.is_empty(),
            "the dropped slot's dirty marker is cleared"
        );
        let drops: Vec<_> = capture
            .all()
            .into_iter()
            .filter(|c| c.has_field("event", "bandolier_slot_dropped"))
            .collect();
        assert_eq!(drops.len(), 1, "{drops:#?}");
        assert_eq!(drops[0].level, Level::WARN);
        assert!(drops[0].has_field("reason", "no_item"), "{:?}", drops[0]);
        assert!(drops[0].has_field("slot_id", "3"), "{:?}", drops[0]);
        assert!(drops[0].has_field("player_id", "72"), "{:?}", drops[0]);
        assert!(drops[0].has_field("account_id", "6"), "{:?}", drops[0]);
    }
}
