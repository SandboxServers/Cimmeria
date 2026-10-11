//! Unreadable saved missions lock instead of loading as none (TG-DB-12,
//! owner decision D-TG18).
//!
//! When the `sgw_mission` read fails at world entry, falling back to "no
//! saved missions" lets a re-accept overwrite completed progress (the #411
//! shape). Instead:
//!
//! 1. [`load_saved_missions_with_retry`] retries the read with a short
//!    backoff ([`READ_BACKOFF_MS`]).
//! 2. If it still fails, world entry [`lock`]s the player here and tells the
//!    cell, which refuses every accept and advance until the state arrives.
//! 3. [`retry_until_readable`] keeps reading in the background every
//!    [`RECOVERY_INTERVAL`] and hands the missions over on the first success.
//!
//! The lock here is the base-side backstop: [`refuse_persist_if_locked`]
//! keeps `handle_mission_update` from writing any `sgw_mission` row for a
//! locked player, so nothing the cell builds from its empty tracker reaches
//! the database even if a cell gate is missed.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use cimmeria_entity::known_names;
use sqlx::PgPool;

use super::try_query_saved_missions;
use crate::cell::messages::SavedMission;

/// Waits before each retry of the world-entry read: four attempts in all,
/// about 1.4 s at worst, so a connection blip doesn't lock anyone.
pub const READ_BACKOFF_MS: [u64; 3] = [200, 400, 800];

/// How often the background read runs while a player's missions are locked.
pub const RECOVERY_INTERVAL: Duration = Duration::from_secs(30);

/// The feedback line a player sees when a mission change is refused.
pub const UNAVAILABLE_LINE: &str = "Mission data unavailable, try again shortly.";

/// Locked players: `player_id` → the entity that owns the lock, so a stale
/// background task from an earlier world entry can't unlock a newer one.
static LOCKED: LazyLock<Mutex<HashMap<i32, u32>>> = LazyLock::new(Default::default);

/// What the world-entry read produced.
#[derive(Debug)]
pub enum SavedMissionsLoad {
    /// The read succeeded (possibly on a retry).
    Loaded(Vec<SavedMission>),
    /// Every attempt failed: lock the player's missions.
    Unreadable,
}

/// Read the saved missions for world entry, retrying with
/// [`READ_BACKOFF_MS`]. Never falls back to "none".
pub async fn load_saved_missions_with_retry(
    db_pool: &Option<Arc<PgPool>>,
    player_id: i32,
) -> SavedMissionsLoad {
    let player_name = known_names::player_name(player_id);
    let mut attempt: u32 = 1;
    loop {
        match try_query_saved_missions(db_pool, player_id).await {
            Ok(missions) => {
                if attempt > 1 {
                    tracing::info!(
                        event = "saved_missions_read_retried",
                        player_id,
                        player_name,
                        attempts = attempt,
                        count = missions.len(),
                        "Saved missions read succeeded after a retry"
                    );
                }
                return SavedMissionsLoad::Loaded(missions);
            }
            Err(e) => {
                let Some(&wait_ms) = READ_BACKOFF_MS.get(attempt as usize - 1) else {
                    tracing::error!(
                        event = "player_load_degraded",
                        reason = "missions_query_failed",
                        player_id,
                        player_name,
                        attempts = attempt,
                        error = %e,
                        "Saved missions unreadable at world entry; the player's missions are locked \
                         (no accept, advance or save) until a background read succeeds"
                    );
                    return SavedMissionsLoad::Unreadable;
                };
                tracing::warn!(
                    event = "saved_missions_read_retry",
                    reason = "query_failed",
                    player_id,
                    player_name,
                    attempt,
                    wait_ms,
                    error = %e,
                    "Saved missions read failed; retrying"
                );
                tokio::time::sleep(Duration::from_millis(wait_ms)).await;
                attempt += 1;
            }
        }
    }
}

/// Lock `player_id`'s missions for the session whose entity is `entity_id`.
pub fn lock(player_id: i32, entity_id: u32) {
    if let Ok(mut locked) = LOCKED.lock() {
        locked.insert(player_id, entity_id);
    }
}

/// Unlock `player_id` if `entity_id` still owns the lock. Returns whether
/// the lock was removed.
pub fn unlock(player_id: i32, entity_id: u32) -> bool {
    let Ok(mut locked) = LOCKED.lock() else {
        return false;
    };
    if locked.get(&player_id) == Some(&entity_id) {
        locked.remove(&player_id);
        return true;
    }
    false
}

/// Drop any lock for `player_id`: a later world entry read the missions.
pub fn clear(player_id: i32) {
    if let Ok(mut locked) = LOCKED.lock() {
        locked.remove(&player_id);
    }
}

/// Whether `player_id`'s missions are locked.
pub fn is_locked(player_id: i32) -> bool {
    LOCKED.lock().is_ok_and(|l| l.contains_key(&player_id))
}

/// The `handle_mission_update` backstop: refuse, with a WARN, a mission row
/// write for a locked player. Returns `true` when the write must not run.
pub fn refuse_persist_if_locked(player_id: i32, mission_id: i32, status: i8) -> bool {
    if !is_locked(player_id) {
        return false;
    }
    tracing::warn!(
        event = "mission_state_unknown",
        reason = "saved_missions_unreadable",
        site = "persist",
        player_id,
        player_name = known_names::player_name(player_id),
        mission_id,
        mission_name = cimmeria_names::book().mission(mission_id),
        status,
        "Mission row write refused: the saved missions are unreadable, so this state was built \
         without them and would overwrite saved progress"
    );
    true
}

/// Keep reading the saved missions every `interval` until a read succeeds,
/// then unlock and hand them to `deliver` (the send to the cell).
///
/// Stops without delivering when `still_live` turns false (the session or
/// its entity is gone); the lock is dropped then too, if this entity still
/// owns it. The unlock comes before `deliver`, so the first save the cell
/// makes after it unlocks is never refused here.
pub async fn retry_until_readable<D, Fut>(
    db_pool: Option<Arc<PgPool>>,
    player_id: i32,
    entity_id: u32,
    interval: Duration,
    mut still_live: impl FnMut() -> bool,
    deliver: D,
) where
    D: FnOnce(Vec<SavedMission>) -> Fut,
    Fut: Future<Output = bool>,
{
    let player_name = known_names::player_name(player_id);
    let mut attempts: u32 = 0;
    loop {
        tokio::time::sleep(interval).await;
        if !still_live() {
            unlock(player_id, entity_id);
            tracing::debug!(
                event = "mission_state_retry_stopped",
                reason = "session_gone",
                player_id,
                player_name,
                entity_id,
                entity_name = player_name,
                attempts,
                "Saved missions background read stopped: the session ended first"
            );
            return;
        }
        attempts += 1;
        match try_query_saved_missions(&db_pool, player_id).await {
            Ok(missions) => {
                unlock(player_id, entity_id);
                let count = missions.len();
                let delivered = deliver(missions).await;
                tracing::info!(
                    event = "saved_missions_read_recovered",
                    player_id,
                    player_name,
                    entity_id,
                    entity_name = player_name,
                    attempts,
                    count,
                    delivered,
                    "Saved missions read succeeded in the background; handing them to the cell"
                );
                return;
            }
            Err(e) => {
                tracing::warn!(
                    event = "saved_missions_read_retry",
                    reason = "query_failed",
                    player_id,
                    player_name,
                    entity_id,
                    entity_name = player_name,
                    attempts,
                    error = %e,
                    "Saved missions still unreadable; the player's missions stay locked"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::LogCapture;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tracing::Level;

    // Sentinels: each test owns its player id, so the shared lock table
    // never sees two tests on one key.
    const P_RECOVER: i32 = 0x7000_D900;
    const P_GONE: i32 = 0x7000_D901;
    const P_STALE: i32 = 0x7000_D902;
    const P_PERSIST: i32 = 0x7000_D903;
    const P_RETRY: i32 = 0x7000_D904;

    fn unreachable_pool() -> Arc<PgPool> {
        Arc::new(
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_millis(50))
                .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
                .expect("connect_lazy must succeed for any well-formed URL"),
        )
    }

    /// The recovery unlock: a read that succeeds (no pool reads as none)
    /// drops the lock before handing the missions over. Without the
    /// `unlock` the cell's first save after recovery would be refused.
    #[tokio::test]
    async fn background_read_success_unlocks_then_delivers() {
        lock(P_RECOVER, 7);
        let unlocked_at_delivery = Arc::new(AtomicBool::new(false));
        let seen = unlocked_at_delivery.clone();
        retry_until_readable(
            None,
            P_RECOVER,
            7,
            Duration::from_millis(1),
            || true,
            |missions| async move {
                assert!(missions.is_empty());
                seen.store(!is_locked(P_RECOVER), Ordering::SeqCst);
                true
            },
        )
        .await;
        assert!(!is_locked(P_RECOVER), "recovery must unlock");
        assert!(
            unlocked_at_delivery.load(Ordering::SeqCst),
            "the unlock must come before the hand-over to the cell"
        );
    }

    #[tokio::test]
    async fn background_read_stops_and_unlocks_when_the_session_ends() {
        lock(P_GONE, 8);
        let delivered = Arc::new(AtomicBool::new(false));
        let d = delivered.clone();
        retry_until_readable(
            Some(unreachable_pool()),
            P_GONE,
            8,
            Duration::from_millis(1),
            || false,
            |_| async move {
                d.store(true, Ordering::SeqCst);
                true
            },
        )
        .await;
        assert!(!delivered.load(Ordering::SeqCst));
        assert!(!is_locked(P_GONE));
    }

    #[test]
    fn a_stale_entity_cannot_unlock_a_newer_world_entry() {
        lock(P_STALE, 1);
        lock(P_STALE, 2);
        assert!(!unlock(P_STALE, 1), "entity 1's task is stale");
        assert!(is_locked(P_STALE));
        assert!(unlock(P_STALE, 2));
        assert!(!is_locked(P_STALE));
    }

    #[test]
    fn persist_is_refused_with_a_warn_while_locked() {
        let capture = LogCapture::install();
        assert!(!refuse_persist_if_locked(P_PERSIST, 622, 1));
        lock(P_PERSIST, 3);
        assert!(refuse_persist_if_locked(P_PERSIST, 622, 1));
        clear(P_PERSIST);
        let row = capture
            .find_event(Level::WARN, "", "saved_missions_unreadable")
            .expect("the refusal WARN");
        assert!(row.has_field("event", "mission_state_unknown"));
        assert!(row.has_field("site", "persist"));
        assert!(row.fields.contains_key("mission_id"));
    }

    /// An unreachable database is retried with the backoff and then
    /// reported as unreadable, never as "no missions".
    #[tokio::test]
    async fn an_unreadable_read_retries_then_reports_unreadable() {
        let capture = LogCapture::install();
        let started = std::time::Instant::now();
        let load = load_saved_missions_with_retry(&Some(unreachable_pool()), P_RETRY).await;
        assert!(matches!(load, SavedMissionsLoad::Unreadable));
        assert!(
            started.elapsed() >= Duration::from_millis(READ_BACKOFF_MS.iter().sum()),
            "every backoff wait must run"
        );
        let retries = capture
            .all()
            .iter()
            .filter(|c| c.has_field("event", "saved_missions_read_retry"))
            .count();
        assert_eq!(retries, READ_BACKOFF_MS.len());
        let row = capture
            .find_event(Level::ERROR, "", "missions_query_failed")
            .expect("the final ERROR");
        assert!(row.has_field("event", "player_load_degraded"));
    }
}
