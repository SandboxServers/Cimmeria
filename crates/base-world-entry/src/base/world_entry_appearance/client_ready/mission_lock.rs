//! World entry's saved-missions read, with the TG-DB-12 lock (D-TG18).
//!
//! The read retries with a short backoff. If it still fails the player
//! enters the world with missions locked: the base lock refuses mission row
//! writes, `MissionStateUnknown` (sent before `InitPlayerState`) locks the
//! cell's tracker, and a background task re-reads every
//! [`state_lock::RECOVERY_INTERVAL`] until it succeeds, then sends
//! `SavedMissionsRecovered`. See `missions::state_lock` in
//! `cimmeria-base-methods` and `cell::missions::state_lock` on the cell.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::cell::messages::{BaseToCellMsg, SavedMission};

use crate::base::world_entry::methods::missions::state_lock::{self, SavedMissionsLoad};

/// Read `player_id`'s saved missions for world entry. Returns the missions
/// for `InitPlayerState`, empty when they are locked, and on a lock tells
/// the cell (before `InitPlayerState`, which the caller sends next) and
/// starts the background read.
pub(super) async fn load_or_lock(
    db_pool: &Option<Arc<sqlx::PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    player_id: i32,
    entity_id: u32,
) -> Vec<SavedMission> {
    match state_lock::load_saved_missions_with_retry(db_pool, player_id).await {
        SavedMissionsLoad::Loaded(missions) => {
            // A lock left by an earlier world entry is stale now.
            state_lock::clear(player_id);
            missions
        }
        SavedMissionsLoad::Unreadable => {
            // No cell channel (a dev setup): there is no cell tracker to
            // lock and no background read to ever unlock, so a base lock
            // here would refuse this player's mission writes until restart.
            let Some(tx) = cell_tx else {
                tracing::debug!(
                    event = "mission_state_lock_skipped",
                    reason = "no_cell_channel",
                    player_id,
                    player_name = cimmeria_entity::known_names::player_name(player_id),
                    entity_id,
                    entity_name = cimmeria_entity::known_names::player_name(player_id),
                    "Saved missions unreadable but no cell channel; not locking"
                );
                return Vec::new();
            };
            state_lock::lock(player_id, entity_id);
            if let Err(e) = tx
                .send(BaseToCellMsg::MissionStateUnknown { entity_id })
                .await
            {
                // The base lock still refuses every mission row write.
                tracing::warn!(
                    event = "mission_state_unknown",
                    reason = "cell_channel_closed",
                    player_id,
                    player_name = cimmeria_entity::known_names::player_name(player_id),
                    entity_id,
                    entity_name = cimmeria_entity::known_names::player_name(player_id),
                    error = %e,
                    "MissionStateUnknown not delivered to the cell; the base still refuses \
                     mission row writes"
                );
            }
            spawn_recovery(
                db_pool.clone(),
                tx.clone(),
                entity_to_addr.clone(),
                player_id,
                entity_id,
            );
            Vec::new()
        }
    }
}

/// Re-read in the background until the read succeeds or the entity's
/// session ends, then hand the missions to the cell.
fn spawn_recovery(
    db_pool: Option<Arc<sqlx::PgPool>>,
    cell_tx: mpsc::Sender<BaseToCellMsg>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    player_id: i32,
    entity_id: u32,
) {
    tokio::spawn(async move {
        state_lock::retry_until_readable(
            db_pool,
            player_id,
            entity_id,
            state_lock::RECOVERY_INTERVAL,
            || {
                entity_to_addr
                    .lock()
                    .is_ok_and(|m| m.contains_key(&entity_id))
            },
            |saved_missions| async move {
                cell_tx
                    .send(BaseToCellMsg::SavedMissionsRecovered {
                        entity_id,
                        saved_missions,
                    })
                    .await
                    .is_ok()
            },
        )
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// With no cell channel nothing could ever unlock the player, so the
    /// base must not lock (the lock would refuse their writes until restart).
    #[tokio::test]
    async fn no_cell_channel_does_not_lock() {
        const PLAYER: i32 = 0x7000_D9B0;
        let pool = Arc::new(
            sqlx::postgres::PgPoolOptions::new()
                .max_connections(1)
                .acquire_timeout(Duration::from_millis(50))
                .connect_lazy("postgres://nobody:nobody@127.0.0.1:1/none")
                .expect("connect_lazy must succeed for any well-formed URL"),
        );
        let missions = load_or_lock(
            &Some(pool),
            &None,
            &Arc::new(Mutex::new(HashMap::new())),
            PLAYER,
            9,
        )
        .await;
        assert!(missions.is_empty());
        assert!(!state_lock::is_locked(PLAYER));
    }
}
