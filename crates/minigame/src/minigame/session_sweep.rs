//! Pending-session TTL sweep: eviction, expiry receipts and the report to Base.

use std::time::Duration;

use cimmeria_wire::cell::messages::CellToBaseMsg;
use tokio::sync::mpsc;
use tokio::time::Instant;

use super::{MinigameSession, SessionRegistry, SessionRegistryInner};

impl SessionRegistry {
    /// Drop every registered-but-never-connected session older than `ttl`.
    /// Returns the evicted entity ids.
    pub async fn expire_pending(&self, ttl: Duration) -> Vec<u32> {
        let mut inner = self.inner.lock().await;
        inner.expire_receipts(Instant::now());
        expire_pending_locked(&mut inner, ttl)
            .into_iter()
            .map(|session| session.entity_id)
            .collect()
    }

    /// [`Self::expire_pending`], with each evicted session's ticket so the
    /// caller can report it to the cell. Each eviction also mints a one-use
    /// result-3 receipt under the same lock, so the Base redeems it through
    /// the ticket-only path like any other result.
    pub async fn expire_pending_sessions(&self, ttl: Duration) -> Vec<(u32, String)> {
        let mut inner = self.inner.lock().await;
        inner.expire_receipts(Instant::now());
        let evicted = expire_pending_locked(&mut inner, ttl);
        evicted
            .into_iter()
            .map(|session| {
                inner.deposit_expiry_receipt(&session);
                (session.entity_id, session.ticket)
            })
            .collect()
    }

    /// Spawn the background expiry sweep without reporting to the cell. Tests
    /// that only care about registry state use this; the server uses
    /// [`Self::spawn_sweep_reporting`].
    pub fn spawn_sweep(&self, ttl: Duration, interval: Duration) {
        self.spawn_sweep_inner(ttl, interval, None);
    }

    /// Spawn the background expiry sweep and report every evicted session to
    /// the cell as result 3 (D-MG15). The cell took the minigame hold when it
    /// launched, and a session whose SWF never connected has no connection
    /// task to send the result, so without this report the player stays
    /// locked with BSF_PlayingMinigame set.
    ///
    /// Called once from [`crate::minigame::server::run`]; the task lives as
    /// long as the process.
    pub fn spawn_sweep_reporting(
        &self,
        ttl: Duration,
        interval: Duration,
        result_tx: mpsc::Sender<CellToBaseMsg>,
    ) {
        self.spawn_sweep_inner(ttl, interval, Some(result_tx));
    }

    fn spawn_sweep_inner(
        &self,
        ttl: Duration,
        interval: Duration,
        result_tx: Option<mpsc::Sender<CellToBaseMsg>>,
    ) {
        let registry = self.clone();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            // The first `tick()` completes immediately; burn it so the
            // first real sweep happens one interval in, not at startup
            // (where there is nothing to sweep anyway).
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let expired = registry.expire_pending_sessions(ttl).await;
                let Some(result_tx) = &result_tx else {
                    continue;
                };
                for (entity_id, ticket) in expired {
                    report_expired(result_tx, entity_id, ticket).await;
                }
            }
        });
    }
}

/// Expiry pass over an already-locked registry.
///
/// Split out so [`SessionRegistry::register`] can sweep inside the lock it
/// already holds without re-entering `Mutex::lock` (which would deadlock on
/// the non-reentrant `tokio::sync::Mutex`).
///
/// The inline call from `register` discards the evicted list on purpose: that
/// `register` is a new launch for the same entity, so the cell's hold is still
/// wanted and the new board's own result releases it.
pub(super) fn expire_pending_locked(
    inner: &mut SessionRegistryInner,
    ttl: Duration,
) -> Vec<MinigameSession> {
    let now = Instant::now();
    inner.boards.retain(|_, board| !board.is_expired(now));
    let expired: Vec<u32> = inner
        .sessions
        .iter()
        .filter(|(_, s)| !s.connected && now.saturating_duration_since(s.created_at) >= ttl)
        .map(|(id, _)| *id)
        .collect();

    let mut evicted = Vec::with_capacity(expired.len());
    for entity_id in &expired {
        if let Some(session) = inner.sessions.remove(entity_id) {
            inner.boards.remove(entity_id);
            // info!, not warn!: an abandoned launch is a player closing a
            // window, not a server fault. It is logged at all because a
            // "minigame won't start" report correlates directly with
            // whether this line fired for that entity.
            tracing::info!(
                entity_id = *entity_id,
                entity_name = session.player_name.as_deref(),
                game = %session.game_name,
                ttl_secs = ttl.as_secs(),
                "Minigame: expiring session whose client never connected",
            );
            evicted.push(session);
        }
    }
    evicted
}

/// Tell the Base an expired session will never run, so it redeems the
/// result-3 receipt the sweep minted and the cell drops the hold.
async fn report_expired(result_tx: &mpsc::Sender<CellToBaseMsg>, entity_id: u32, ticket: String) {
    if let Err(e) = result_tx
        .send(CellToBaseMsg::MinigameResult { ticket })
        .await
    {
        tracing::error!(
            entity_id, // nt:id-only the session is already evicted, so its player name is gone
            error = %e,
            reason = "result_channel_closed",
            "Minigame: expiry report failed, the player keeps the minigame hold until the cell restarts"
        );
    }
}
