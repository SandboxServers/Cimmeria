//! Base side of `gmGiveTrainingPoints`: one guarded `UPDATE`, then the cell.
//!
//! The cell has already GM-gated the caller, refused a non-positive amount and
//! resolved `player_id`. The base adds the points in a single statement
//! ([`persist_training_points_grant`]), refreshes the session's point cache,
//! and sends `BaseToCellMsg::TrainingPointsGranted`; the cell mirrors the
//! points and sends the client counter.

use cimmeria_entity::known_names;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::super::gm_feedback::send_gm_feedback_to_client;
use super::super::super::super::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;

/// One `CellToBaseMsg::GrantTrainingPoints`, as the base handles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainingPointsGrant {
    pub entity_id: u32,
    pub player_id: i32,
    /// Points to add. The cell refuses `<= 0`; the base re-checks.
    pub amount: i32,
    /// The GM who receives the feedback line.
    pub gm_feedback_to: Option<u32>,
}

/// Add `amount` to `player_id`'s training points and return the new total.
///
/// `Ok(None)` means the row was held back and **nothing** changed: no such
/// player, or the sum would pass `i32::MAX` (`sgw_player.training_points` is
/// `integer`, so an unguarded add would raise a Postgres overflow error). The
/// guard is written as `training_points <= i32::MAX - amount` so that the
/// comparison itself cannot overflow for a positive `amount`.
///
/// Why add in SQL instead of writing the cached value: the cache can be
/// stale, and an absolute write from a stale cache is how a grant gets lost.
pub(super) async fn persist_training_points_grant(
    pool: &PgPool,
    player_id: i32,
    amount: i32,
) -> sqlx::Result<Option<i32>> {
    sqlx::query_scalar::<_, i32>(
        "UPDATE sgw_player \
            SET training_points = training_points + $1 \
          WHERE player_id = $2 \
            AND training_points <= 2147483647 - $1 \
        RETURNING training_points",
    )
    .bind(amount)
    .bind(player_id)
    .fetch_optional(pool)
    .await
}

/// Persist a GM training-point grant, tell the cell, and report to the GM.
#[tracing::instrument(
    name = "progression.grant_training_points",
    level = "info",
    skip_all,
    fields(
        entity_id = grant.entity_id,
        player_id = grant.player_id,
        amount = grant.amount,
    )
)]
pub async fn handle_grant_training_points(
    grant: TrainingPointsGrant,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<tokio::sync::mpsc::Sender<BaseToCellMsg>>,
) {
    let TrainingPointsGrant {
        entity_id,
        player_id,
        amount,
        gm_feedback_to,
    } = grant;
    // Every refusal still answers the GM: a console command that prints
    // nothing reads as a dropped packet.
    let refuse = |reason: &'static str| async move {
        if let Some(gm_id) = gm_feedback_to {
            send_gm_feedback_to_client(
                gm_id,
                &format!("gmGiveTrainingPoints: refused — {reason}"),
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
    };

    // The cell refuses this first; a corrupted message must not debit.
    if amount <= 0 {
        let player_label = known_names::player_name(player_id);
        tracing::warn!(
            entity_id,
            entity_name = player_label,
            player_id,
            player_name = player_label,
            amount,
            "GrantTrainingPoints: non-positive amount — rejecting"
        );
        refuse("amount must be positive").await;
        return;
    }
    let Some(pool) = db_pool else {
        let player_label = known_names::player_name(player_id);
        tracing::warn!(
            entity_id,
            entity_name = player_label,
            player_id,
            player_name = player_label,
            amount,
            "GrantTrainingPoints: no DB pool, dropping grant"
        );
        refuse("no database").await;
        return;
    };
    let Some(addr) = entity_to_addr.lock().unwrap().get(&entity_id).copied() else {
        let player_label = known_names::player_name(player_id);
        tracing::warn!(
            entity_id,
            entity_name = player_label,
            player_id,
            player_name = player_label,
            "GrantTrainingPoints: no address for entity"
        );
        return;
    };

    // The session must still be playing the character the cell resolved. A
    // reused entity id would otherwise credit one character and refresh
    // another's cache.
    let active = match connected.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
    .get(&addr)
    .and_then(|s| s.active_player_id);
    if active != Some(player_id) {
        let player_label = known_names::player_name(player_id);
        tracing::warn!(
            entity_id,
            entity_name = player_label,
            player_id,
            player_name = player_label,
            active_player_id = ?active,
            active_player_name = known_names::player_name(active),
            "GrantTrainingPoints: session is not playing the resolved character — rejecting"
        );
        refuse("character is no longer active").await;
        return;
    }

    let total = match persist_training_points_grant(pool, player_id, amount).await {
        Ok(Some(total)) => total,
        Ok(None) => {
            let player_label = known_names::player_name(player_id);
            tracing::warn!(
                entity_id,
                entity_name = player_label,
                player_id,
                player_name = player_label,
                amount,
                "GrantTrainingPoints: UPDATE matched 0 rows (player missing, or the \
                 total would pass i32::MAX) — nothing granted"
            );
            refuse("the total would pass 2147483647").await;
            return;
        }
        Err(e) => {
            let player_label = known_names::player_name(player_id);
            tracing::error!(
                entity_id,
                entity_name = player_label,
                player_id,
                player_name = player_label,
                amount,
                "GrantTrainingPoints: UPDATE failed: {e}"
            );
            refuse("database error").await;
            return;
        }
    };

    // `handle_grant_xp` writes `training_points` as an absolute value from
    // this cache, so a stale cache would erase the grant on the next level-up.
    {
        let mut map = match connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(state) = map
            .get_mut(&addr)
            .filter(|s| s.active_player_id == Some(player_id))
        {
            state.player_training_points = Some(total.max(0) as u32);
        }
    }

    let player_label = known_names::player_name(player_id);
    tracing::info!(
        entity_id,
        entity_name = player_label,
        player_id,
        player_name = player_label,
        amount,
        training_points = total,
        "GrantTrainingPoints: persisted"
    );

    match cell_tx {
        Some(tx) => {
            if let Err(e) = tx
                .send(BaseToCellMsg::TrainingPointsGranted {
                    entity_id,
                    training_points: total,
                })
                .await
            {
                tracing::error!(
                    entity_id,
                    entity_name = known_names::player_name(player_id),
                    training_points = total,
                    error = %e,
                    "GrantTrainingPoints: base→cell send failed; counter and trainer gate stale until relog"
                );
            }
        }
        None => tracing::warn!(
            entity_id,
            entity_name = known_names::player_name(player_id),
            training_points = total,
            "GrantTrainingPoints: no cell channel; counter and trainer gate stale until relog"
        ),
    }

    if let Some(gm_id) = gm_feedback_to {
        send_gm_feedback_to_client(
            gm_id,
            &format!("gmGiveTrainingPoints: +{amount} training points (total {total})"),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }
}
