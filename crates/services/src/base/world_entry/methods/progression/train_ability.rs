//! Base side of a trainer purchase: the atomic debit and spend increment.
//!
//! The cell has already passed `ability_tree::evaluate_train` and sends the
//! node's cost with the request. The base does exactly one `UPDATE`
//! ([`persist_purchase`]) and reports the new counters back to the cell with
//! `BaseToCellMsg::AbilityGranted`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;

use super::super::super::super::ConnectedClientState;

/// One `CellToBaseMsg::TrainAbility`, as the base handles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrainRequest {
    pub entity_id: u32,
    pub player_id: i32,
    pub ability_id: i32,
    /// The node's `skill_point_cost`.
    pub cost: i32,
    /// The node's branch, for logs.
    pub tree_index: i32,
}

/// The row after a purchase: `RETURNING training_points, tree_points_spent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct PurchaseResult {
    pub training_points: i32,
    pub tree_points_spent: i32,
}

/// Buy `ability_id` for `player_id` in one statement.
///
/// Appends the ability to `abilities` and `trained_abilities`, debits `cost`
/// from `training_points` and adds it to `tree_points_spent`, but only when
/// `training_points >= cost` and the ability is not already known. `Ok(None)`
/// means the guard held the row back (no such player, too few points, or a
/// replayed purchase) and **nothing** changed; the four fields move together
/// or not at all.
///
/// Why one statement: a double-click or a replayed packet sends two
/// `TrainAbility`s. The second one's `NOT (abilities @> ...)` sees the first
/// one's append under Postgres row locking, so it matches 0 rows instead of
/// debiting twice. A read-then-write pair would leave that window open.
pub async fn persist_purchase(
    pool: &PgPool,
    player_id: i32,
    ability_id: i32,
    cost: i32,
) -> sqlx::Result<Option<PurchaseResult>> {
    sqlx::query_as::<_, PurchaseResult>(
        "UPDATE sgw_player \
            SET abilities = abilities || $1::integer, \
                trained_abilities = trained_abilities || $1::integer, \
                training_points = training_points - $3, \
                tree_points_spent = tree_points_spent + $3 \
          WHERE player_id = $2 \
            AND training_points >= $3 \
            AND NOT (abilities @> ARRAY[$1::integer]) \
        RETURNING training_points, tree_points_spent",
    )
    .bind(ability_id)
    .bind(player_id)
    .bind(cost)
    .fetch_optional(pool)
    .await
}

/// Persist a trainer purchase and tell the cell.
#[tracing::instrument(
    name = "progression.train_ability",
    level = "info",
    skip_all,
    fields(
        entity_id = request.entity_id,
        player_id = request.player_id,
        ability_id = request.ability_id,
        cost = request.cost,
        tree_index = request.tree_index,
    )
)]
pub async fn handle_train_ability(
    request: TrainRequest,
    db_pool: &Option<Arc<PgPool>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<tokio::sync::mpsc::Sender<crate::cell::messages::BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let TrainRequest {
        entity_id,
        player_id,
        ability_id,
        cost,
        tree_index,
    } = request;
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::warn!(entity_id, player_id, ability_id, "TrainAbility: no DB pool");
            return;
        }
    };

    // A negative cost would credit points and shrink the spend. The schema
    // forbids it (`skill_point_cost >= 0`), so this only fires on a
    // corrupted message; refuse rather than let the UPDATE run.
    if cost < 0 {
        tracing::warn!(
            target: "abilities",
            event = "train_negative_cost",
            entity_id,
            player_id,
            ability_id,
            cost,
            "TrainAbility: negative cost — rejecting"
        );
        return;
    }

    let addr = match entity_to_addr.lock().unwrap().get(&entity_id).copied() {
        Some(a) => a,
        None => {
            tracing::warn!(entity_id, "TrainAbility: no address for entity");
            return;
        }
    };

    // The session must still be playing the character the cell validated.
    // An entity id reused between the cell's send and this handler would
    // otherwise write another character's point cache and hand its cell
    // entity an ability it never bought.
    //
    // Then the fast path: the UPDATE's `training_points >= cost` guard is
    // the authority, but a stale-cache pre-check spares a DB round-trip on
    // the common "out of points" case.
    {
        let map = match connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let session = map.get(&addr);
        let active = session.and_then(|s| s.active_player_id);
        if active != Some(player_id) {
            tracing::warn!(
                target: "abilities",
                event = "train_player_mismatch",
                entity_id,
                player_id,
                ability_id,
                active_player_id = ?active,
                "TrainAbility: session is not playing the validated character — rejecting"
            );
            return;
        }
        let tp_in_memory = session.and_then(|s| s.player_training_points).unwrap_or(0);
        if (tp_in_memory as i64) < cost as i64 {
            tracing::info!(
                entity_id,
                player_id,
                ability_id,
                cost,
                training_points = tp_in_memory,
                "TrainAbility: rejected — not enough training points (in-memory)"
            );
            return;
        }
    }

    let result = match persist_purchase(pool, player_id, ability_id, cost).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            tracing::info!(
                entity_id,
                player_id,
                ability_id,
                cost,
                "TrainAbility: UPDATE matched 0 rows (player missing, too few points, \
                 or already known) — nothing debited"
            );
            return;
        }
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                ability_id,
                "TrainAbility: UPDATE failed: {e}"
            );
            return;
        }
    };

    // Sync in-memory training_points so the next train attempt and the next
    // level-up start from the post-debit value without a DB read.
    {
        let mut map = match connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(state) = map
            .get_mut(&addr)
            .filter(|s| s.active_player_id == Some(player_id))
        {
            state.player_training_points = Some(result.training_points.max(0) as u32);
        }
    }

    tracing::info!(
        target: "abilities",
        event = "train_persisted",
        entity_id,
        player_id,
        ability_id,
        tree_index,
        cost,
        training_points = result.training_points,
        tree_points_spent = result.tree_points_spent,
        "TrainAbility: persisted + debited"
    );

    // Notify the cell so it mirrors the purchase and refreshes the client.
    // If the channel is gone, the hotbar and counter stay one purchase
    // behind until relog — log loudly so SigNoz surfaces the desync.
    if let Some(tx) = cell_tx {
        if let Err(e) = tx
            .send(crate::cell::messages::BaseToCellMsg::AbilityGranted {
                entity_id,
                ability_id,
                training_points: result.training_points,
                tree_points_spent: result.tree_points_spent,
            })
            .await
        {
            tracing::error!(
                entity_id, ability_id, error = %e,
                "TrainAbility: base→cell AbilityGranted send failed; hotbar will desync until relog"
            );
        }
    }
}
