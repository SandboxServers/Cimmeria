//! `CellToBaseMsg::ShutdownFlush` (TG-DB-02, D-TG15): the last save of every
//! in-world player when the server stops.
//!
//! The cell sends one request with each player's world, position and dirty
//! bandolier ammo. Each player is saved in a task of its own, so the writes
//! run in parallel (bounded by the pool's connection limit), through the same
//! writers a logout uses (`position::persist_position`,
//! `bandolier::bandolier_ammo_update`). Whatever is still running at the
//! request's deadline is aborted. The reply names the players whose every
//! write landed; the cell reports the others as `player_state_not_persisted`.
//!
//! The dispatch loop awaits this inline, so every cell message queued ahead
//! of the request (a logout's `PersistPosition`, an ammo update) has been
//! handled before the first shutdown write starts.

use std::future::Future;
use std::sync::Arc;

use sqlx::PgPool;
use tokio::task::JoinSet;

use crate::cell::messages::{ShutdownFlushRequest, ShutdownPlayerSave};

use super::{bandolier, position};

/// Run the shutdown flush and answer the cell.
pub(super) async fn handle_shutdown_flush(
    req: ShutdownFlushRequest,
    db_pool: &Option<Arc<PgPool>>,
) {
    let ShutdownFlushRequest {
        saves,
        deadline,
        reply,
    } = req;
    let pool = db_pool.clone();
    let saved = run_flush(saves, deadline, move |save| {
        let pool = pool.clone();
        async move { save_player(save, pool).await }
    })
    .await;
    if reply.send(saved).is_err() {
        // The cell stopped waiting (its own deadline passed first). It has
        // already reported every player as not persisted.
        tracing::debug!(
            event = "shutdown_flush_reply_dropped",
            reason = "cell_stopped_waiting",
            "shutdown flush: the cell stopped waiting before the answer"
        );
    }
}

/// Save every player with `save`, in parallel, until `deadline`. Returns the
/// `player_id`s whose save returned `true`; a save still running at the
/// deadline is aborted and counts as missed.
pub(super) async fn run_flush<F, Fut>(
    saves: Vec<ShutdownPlayerSave>,
    deadline: tokio::time::Instant,
    save: F,
) -> Vec<i32>
where
    F: Fn(ShutdownPlayerSave) -> Fut,
    Fut: Future<Output = bool> + Send + 'static,
{
    let mut set = JoinSet::new();
    for s in saves {
        let player_id = s.player_id;
        let fut = save(s);
        set.spawn(async move { (player_id, fut.await) });
    }
    let mut saved = Vec::with_capacity(set.len());
    loop {
        match tokio::time::timeout_at(deadline, set.join_next()).await {
            Ok(Some(Ok((player_id, true)))) => saved.push(player_id),
            // A failed write logged its own WARN with the reason.
            Ok(Some(Ok((_, false)))) => {}
            Ok(Some(Err(e))) => tracing::warn!(
                event = "shutdown_flush_task_failed",
                reason = "task_panicked",
                error = %e,
                "shutdown flush: a player's save task failed"
            ),
            Ok(None) => break,
            Err(_) => {
                tracing::debug!(
                    event = "shutdown_flush_deadline_reached",
                    abandoned = set.len(),
                    "shutdown flush: deadline reached, abandoning the saves still running"
                );
                set.abort_all();
                break;
            }
        }
    }
    saved
}

/// One player's writes: the position, then each dirty ammo slot. Every
/// write is attempted; the player counts as saved only if all of them land.
async fn save_player(save: ShutdownPlayerSave, pool: Option<Arc<PgPool>>) -> bool {
    let mut ok =
        position::persist_position(save.player_id, None, &save.world_name, save.position, &pool)
            .await;
    for slot in &save.ammo {
        ok &= bandolier::bandolier_ammo_update(
            save.player_id,
            slot.slot_id,
            slot.expected_instance_id,
            slot.current_ammo,
            slot.cur_ammo_type,
            &pool,
        )
        .await;
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::Instant;

    fn save(player_id: i32) -> ShutdownPlayerSave {
        ShutdownPlayerSave {
            player_id,
            world_name: "Castle".to_string(),
            position: [1.0, 2.0, 3.0],
            ammo: Vec::new(),
        }
    }

    /// **Deadline guard.** A save that never finishes is cut at the deadline
    /// and left out of the answer, while the saves that finished are in it.
    /// Without the deadline the call never returns (the test times out);
    /// without the abort, the stuck save would hold the shutdown open.
    /// Paused time: the runtime jumps the clock to the next timer when idle,
    /// so the elapsed time is exact, not a wall-clock bound.
    #[tokio::test(start_paused = true)]
    async fn stuck_save_is_cut_at_the_deadline_and_missed() {
        let deadline = Instant::now() + Duration::from_secs(10);
        let started = Instant::now();
        let saved = tokio::time::timeout(
            Duration::from_secs(60),
            run_flush(vec![save(1), save(2), save(3)], deadline, |s| async move {
                match s.player_id {
                    // Stuck: a write that never comes back.
                    2 => std::future::pending().await,
                    // A write that failed.
                    3 => false,
                    _ => true,
                }
            }),
        )
        .await
        .expect("run_flush must return at its deadline");
        assert_eq!(saved, vec![1], "only the finished, successful save counts");
        assert_eq!(
            started.elapsed(),
            Duration::from_secs(10),
            "the flush stops exactly at the deadline"
        );
    }

    /// Saves run in parallel: ten saves of 150 ms each all finish 150 ms in,
    /// inside a 1 s deadline. Run one after another they would need 1.5 s
    /// and miss. Paused time makes the elapsed time exact.
    #[tokio::test(start_paused = true)]
    async fn saves_run_in_parallel() {
        let started = Instant::now();
        let deadline = started + Duration::from_secs(1);
        let saves = (1..=10).map(save).collect();
        let mut saved = run_flush(saves, deadline, |_| async {
            tokio::time::sleep(Duration::from_millis(150)).await;
            true
        })
        .await;
        saved.sort_unstable();
        assert_eq!(saved, (1..=10).collect::<Vec<_>>());
        assert_eq!(started.elapsed(), Duration::from_millis(150));
    }
}
