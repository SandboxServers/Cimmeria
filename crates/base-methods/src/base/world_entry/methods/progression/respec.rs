//! Base side of a trainer respec (`resetMyAbilities`, AT-08, D-AT03).
//!
//! The cell has already checked that the player stands at a pinned trainer
//! in range. The base does exactly one guarded `UPDATE`
//! ([`persist_respec`]) and answers the cell with
//! `BaseToCellMsg::AbilitiesReset`, which carries a [`RespecOutcome`].
//!
//! **No saved hotbar exists server-side.** The owner asked (2026-09-26) for
//! refunded ids to be stripped from the saved hotbar in the same
//! transaction. The client keeps its action bar in a per-character Lua
//! saved variable (`GActionProfiles`, a `<CharacterVariable>` in
//! `ActionButtons.toc`) under `My Games/.../SGWGame/<account>/<char>/`, and
//! no server method, property or table carries it. The only server-held
//! ability list the client's bar draws from is `sgw_player.abilities`,
//! which this `UPDATE` strips. See the AT-08 worknote.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;

use super::super::super::super::ConnectedClientState;
use crate::ability_tree::RespecOutcome;

/// One `CellToBaseMsg::ResetAbilities`, as the base handles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RespecRequest {
    pub entity_id: u32,
    pub player_id: i32,
    /// Naquadah to charge (`ability_tree::RESPEC_COST_NAQUADAH`).
    pub cost: i32,
}

/// Respec `player_id` for `cost` naquadah in one statement.
///
/// In one `UPDATE`, guarded by `naquadah >= cost` and "something was
/// trainer-bought":
/// - `abilities` loses every id in `trained_abilities` (order kept), so
///   starter and quest grants survive;
/// - `training_points += tree_points_spent`, the exact refund, because only
///   trainer purchases count as spend (D-AT03);
/// - `tree_points_spent = 0`, `trained_abilities = '{}'`;
/// - `naquadah -= cost`.
///
/// The locked self-join supplies the pre-update `trained_abilities` for
/// `RETURNING` (Postgres 17 has no `OLD` in `RETURNING`).
///
/// When the guard holds the row back, a read-only `SELECT` picks the
/// outcome: [`RespecOutcome::NothingToReset`] (a replay lands here) or
/// [`RespecOutcome::NotEnoughNaquadah`]. Nothing changes in either case.
/// `Ok(None)` means the player row does not exist.
///
/// Why one statement: a double-click or a replayed packet sends two
/// requests. The second one's guard sees the first one's reset under row
/// locking, finds nothing trainer-bought, and matches 0 rows, so the
/// charge happens once.
pub(super) async fn persist_respec(
    pool: &PgPool,
    player_id: i32,
    cost: i32,
) -> sqlx::Result<Option<RespecOutcome>> {
    let reset: Option<(Vec<i32>, i32, i32)> = sqlx::query_as(
        "UPDATE sgw_player AS p \
            SET abilities = ARRAY(\
                    SELECT a FROM unnest(p.abilities) WITH ORDINALITY AS u(a, n) \
                     WHERE NOT (a = ANY(p.trained_abilities)) \
                     ORDER BY n), \
                training_points = p.training_points + p.tree_points_spent, \
                tree_points_spent = 0, \
                trained_abilities = '{}', \
                naquadah = p.naquadah - $2 \
           FROM (SELECT player_id, trained_abilities FROM sgw_player \
                  WHERE player_id = $1 FOR UPDATE) AS old \
          WHERE p.player_id = old.player_id \
            AND p.naquadah >= $2 \
            AND (p.tree_points_spent > 0 OR cardinality(p.trained_abilities) > 0) \
        RETURNING old.trained_abilities, p.training_points, p.naquadah",
    )
    .bind(player_id)
    .bind(cost)
    .fetch_optional(pool)
    .await?;

    if let Some((refunded, training_points, naquadah)) = reset {
        return Ok(Some(RespecOutcome::Reset {
            refunded,
            training_points,
            naquadah,
        }));
    }

    // The guard held the row back. Only the feedback depends on this read,
    // so it needs no lock: nothing is written either way.
    let row: Option<(i32, bool)> = sqlx::query_as(
        "SELECT naquadah, (tree_points_spent > 0 OR cardinality(trained_abilities) > 0) \
           FROM sgw_player WHERE player_id = $1",
    )
    .bind(player_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(naquadah, anything_trained)| {
        if anything_trained && naquadah < cost {
            RespecOutcome::NotEnoughNaquadah { naquadah }
        } else {
            RespecOutcome::NothingToReset
        }
    }))
}

/// Persist a respec and tell the cell what happened.
#[tracing::instrument(
    name = "progression.reset_abilities",
    level = "info",
    skip_all,
    fields(
        entity_id = request.entity_id,
        player_id = request.player_id,
        cost = request.cost,
    )
)]
pub async fn handle_reset_abilities(
    request: RespecRequest,
    db_pool: &Option<Arc<PgPool>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    cell_tx: &Option<tokio::sync::mpsc::Sender<crate::cell::messages::BaseToCellMsg>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let RespecRequest {
        entity_id,
        player_id,
        cost,
    } = request;
    let Some(pool) = db_pool else {
        tracing::warn!(entity_id, player_id, "ResetAbilities: no DB pool");
        return;
    };

    // A negative cost would pay the player for a respec. Only a corrupted
    // message carries one; refuse before the UPDATE.
    if cost < 0 {
        tracing::warn!(
            target: "abilities",
            event = "respec_negative_cost",
            entity_id,
            player_id,
            cost,
            "ResetAbilities: negative cost — rejecting"
        );
        return;
    }

    let Some(addr) = entity_to_addr.lock().unwrap().get(&entity_id).copied() else {
        tracing::warn!(entity_id, "ResetAbilities: no address for entity");
        return;
    };

    // The session must still be playing the character the cell validated;
    // a reused entity id would otherwise reset another character.
    {
        let map = match connected.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let active = map.get(&addr).and_then(|s| s.active_player_id);
        if active != Some(player_id) {
            tracing::warn!(
                target: "abilities",
                event = "respec_player_mismatch",
                entity_id,
                player_id,
                active_player_id = ?active,
                "ResetAbilities: session is not playing the validated character — rejecting"
            );
            return;
        }
    }

    let outcome = match persist_respec(pool, player_id, cost).await {
        Ok(Some(o)) => o,
        Ok(None) => {
            tracing::warn!(
                target: "abilities",
                event = "respec_player_missing",
                entity_id,
                player_id,
                "ResetAbilities: no sgw_player row — nothing reset"
            );
            return;
        }
        Err(e) => {
            tracing::error!(entity_id, player_id, "ResetAbilities: UPDATE failed: {e}");
            return;
        }
    };

    match &outcome {
        RespecOutcome::Reset {
            refunded,
            training_points,
            naquadah,
        } => {
            // The next purchase and level-up start from the refunded value.
            let mut map = match connected.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            if let Some(state) = map
                .get_mut(&addr)
                .filter(|s| s.active_player_id == Some(player_id))
            {
                state.player_training_points = Some((*training_points).max(0) as u32);
            }
            tracing::info!(
                target: "abilities",
                event = "respec_persisted",
                entity_id,
                player_id,
                refunded = ?refunded,
                training_points,
                naquadah,
                cost,
                "ResetAbilities: trainer abilities removed, points refunded, naquadah charged"
            );
        }
        RespecOutcome::NothingToReset => tracing::info!(
            target: "abilities",
            event = "respec_rejected",
            reason = "nothing_trained",
            entity_id,
            player_id,
            "ResetAbilities: nothing trainer-bought — no change, no charge"
        ),
        RespecOutcome::NotEnoughNaquadah { naquadah } => tracing::info!(
            target: "abilities",
            event = "respec_rejected",
            reason = "not_enough_naquadah",
            entity_id,
            player_id,
            naquadah,
            cost,
            "ResetAbilities: too little naquadah — no change"
        ),
    }

    if let Some(tx) = cell_tx {
        if let Err(e) = tx
            .send(crate::cell::messages::BaseToCellMsg::AbilitiesReset { entity_id, outcome })
            .await
        {
            tracing::error!(
                entity_id, player_id, error = %e,
                "ResetAbilities: base→cell AbilitiesReset send failed; the cell keeps the \
                 pre-respec abilities and points until relog"
            );
        }
    }
}
