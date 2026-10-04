//! Base side of the GM bulk ability commands (ability-mechanics AB-N2):
//! `gmGiveAllAbilities` (154) and `gmResetAbilities` (153).
//!
//! The cell has GM-gated the call and resolved the GM's own `player_id`.
//! The base writes `sgw_player` in one locked `UPDATE`
//! ([`persist_bulk`]) and answers the cell with
//! `BaseToCellMsg::GmAbilitiesChanged`, which the cell mirrors with one
//! `onKnownAbilitiesUpdate` burst and the GM's result line. A refusal here
//! (no database, a recycled entity, no row, no starters, a database error)
//! sends the GM the reason instead.
//!
//! - **Give all** appends every id the row lacks, in the order the cell
//!   sent them (tree order). It is not a trainer purchase: `trained_abilities`,
//!   `training_points` and `tree_points_spent` are untouched, so a respec
//!   keeps the grants, as with `.giveability`.
//! - **Reset** sets `abilities` to the archetype's character-creation
//!   starters (`resources.char_creation_abilities` for every `char_creation`
//!   row of the character's archetype, the set `createCharacter` granted),
//!   refunds `tree_points_spent` into `training_points` and clears
//!   `trained_abilities`, as the trainer respec does. Unlike the respec it
//!   needs no trainer, charges nothing, and also removes quest and GM
//!   grants: it is the clean slate a test run starts from.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use super::super::super::super::gm_feedback::send_gm_feedback_to_client;
use super::super::super::super::ConnectedClientState;
use crate::cell::messages::{BaseToCellMsg, GmAbilitiesChanged, GmAbilityBulk, GmAbilityChange};

/// What [`persist_bulk`] wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BulkWrite {
    /// `abilities` before and after.
    pub before: Vec<i32>,
    pub after: Vec<i32>,
    /// `training_points` after.
    pub training_points: i32,
}

/// Why [`persist_bulk`] wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BulkRefusal {
    /// No `sgw_player` row.
    PlayerRowMissing,
    /// Reset: the archetype has no starter abilities in the seed, and an
    /// empty set would leave the character with no attack.
    NoStarters,
}

/// The archetype's character-creation starters, ascending. `archetype` is
/// the `sgw_player.archetype` ordinal, the `EArchetype` enum position, as
/// `player_load` reads the ability tree. Takes any executor so the reset
/// reads it on its own transaction's connection.
pub(super) async fn starter_abilities<'e, E>(executor: E, archetype: i32) -> sqlx::Result<Vec<i32>>
where
    E: sqlx::PgExecutor<'e>,
{
    sqlx::query_scalar(
        "SELECT DISTINCT ca.ability_id \
           FROM resources.char_creation_abilities ca \
           JOIN resources.char_creation cc USING (char_def_id) \
          WHERE cc.archetype = (enum_range(NULL::resources.\"EArchetype\"))[$1 + 1] \
          ORDER BY 1",
    )
    .bind(archetype)
    .fetch_all(executor)
    .await
}

/// Apply one bulk change to `player_id`'s row under a row lock.
pub(super) async fn persist_bulk(
    pool: &PgPool,
    player_id: i32,
    change: GmAbilityChange,
    ability_ids: &[i32],
) -> sqlx::Result<Result<BulkWrite, BulkRefusal>> {
    let mut txn = pool.begin().await?;
    let row: Option<(Vec<i32>, i32)> = sqlx::query_as(
        "SELECT abilities, archetype FROM sgw_player WHERE player_id = $1 FOR UPDATE",
    )
    .bind(player_id)
    .fetch_optional(&mut *txn)
    .await?;
    let Some((before, archetype)) = row else {
        return Ok(Err(BulkRefusal::PlayerRowMissing));
    };
    let (after, training_points): (Vec<i32>, i32) = match change {
        GmAbilityChange::GrantAll => {
            let mut after = before.clone();
            for &id in ability_ids {
                if !after.contains(&id) {
                    after.push(id);
                }
            }
            sqlx::query_as(
                "UPDATE sgw_player SET abilities = $2 WHERE player_id = $1 \
                 RETURNING abilities, training_points",
            )
            .bind(player_id)
            .bind(&after)
            .fetch_one(&mut *txn)
            .await?
        }
        GmAbilityChange::Reset => {
            // On the transaction's connection: a second pool connection
            // while this one holds the row lock can starve the pool when
            // several resets run at once.
            let starters = starter_abilities(&mut *txn, archetype).await?;
            if starters.is_empty() {
                return Ok(Err(BulkRefusal::NoStarters));
            }
            sqlx::query_as(
                "UPDATE sgw_player \
                    SET abilities = $2, \
                        training_points = training_points + tree_points_spent, \
                        tree_points_spent = 0, \
                        trained_abilities = '{}' \
                  WHERE player_id = $1 \
                 RETURNING abilities, training_points",
            )
            .bind(player_id)
            .bind(&starters)
            .fetch_one(&mut *txn)
            .await?
        }
    };
    txn.commit().await?;
    Ok(Ok(BulkWrite {
        before,
        after,
        training_points,
    }))
}

/// The character `entity_id`'s session plays now, if any.
fn active_player_of(
    entity_id: u32,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) -> Option<i32> {
    let addr = entity_to_addr.lock().unwrap().get(&entity_id).copied()?;
    match connected.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
    .get(&addr)
    .and_then(|s| s.active_player_id)
}

/// Persist a GM bulk ability change and tell the cell, or tell the GM why
/// not. Every outcome is one event on target `abilities`,
/// `event = "gm_ability_bulk"`, with `decision_outcome` and `persisted`.
#[tracing::instrument(
    name = "progression.gm_ability_bulk",
    level = "info",
    skip_all,
    fields(
        entity_id = msg.entity_id,
        account_id = msg.account_id,
        player_id = msg.player_id,
        cmd = msg.change.command(),
    )
)]
pub async fn handle_gm_ability_bulk(
    msg: GmAbilityBulk,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<tokio::sync::mpsc::Sender<BaseToCellMsg>>,
) {
    let GmAbilityBulk {
        entity_id,
        player_id,
        account_id,
        change,
        ability_ids,
    } = msg;
    let cmd = change.command();
    let refuse = |reason: &'static str, text: String| async move {
        tracing::warn!(
            target: "abilities",
            event = "gm_ability_bulk",
            decision_outcome = "refused",
            reason,
            persisted = false,
            entity_id,
            account_id,
            player_id,
            cmd,
            "GM bulk ability change refused"
        );
        // Only while the GM's entity still plays the GM's character: an
        // entity id is recycled on relog.
        if active_player_of(entity_id, connected, entity_to_addr) == Some(player_id) {
            send_gm_feedback_to_client(entity_id, &text, transport, connected, entity_to_addr)
                .await;
        }
    };

    let Some(pool) = db_pool else {
        refuse("no_database", format!("{cmd}: refused, no database")).await;
        return;
    };
    if active_player_of(entity_id, connected, entity_to_addr) != Some(player_id) {
        refuse("session_mismatch", String::new()).await;
        return;
    }
    let write = match persist_bulk(pool, player_id, change, &ability_ids).await {
        Ok(Ok(w)) => w,
        Ok(Err(BulkRefusal::PlayerRowMissing)) => {
            let text = format!("{cmd}: your character has no saved record; nothing changed");
            refuse("player_row_missing", text).await;
            return;
        }
        Ok(Err(BulkRefusal::NoStarters)) => {
            let text = format!(
                "{cmd}: no starter abilities are seeded for your archetype; nothing changed"
            );
            refuse("no_starters", text).await;
            return;
        }
        Err(e) => {
            tracing::error!(
                target: "abilities",
                event = "gm_ability_bulk",
                decision_outcome = "refused",
                reason = "db_error",
                persisted = false,
                entity_id,
                account_id,
                player_id,
                cmd,
                error = %e,
                "GM bulk ability change: database error"
            );
            let text = format!("{cmd}: database error; nothing changed");
            send_gm_feedback_to_client(entity_id, &text, transport, connected, entity_to_addr)
                .await;
            return;
        }
    };

    let added: Vec<i32> = write
        .after
        .iter()
        .copied()
        .filter(|id| !write.before.contains(id))
        .collect();
    let removed: Vec<i32> = write
        .before
        .iter()
        .copied()
        .filter(|id| !write.after.contains(id))
        .collect();
    tracing::info!(
        target: "abilities",
        event = "gm_ability_bulk",
        decision_outcome = "persisted",
        persisted = true,
        entity_id,
        account_id,
        player_id,
        cmd,
        added = added.len(),
        removed = removed.len(),
        training_points = write.training_points,
        "GM bulk ability change persisted"
    );

    let reply = BaseToCellMsg::GmAbilitiesChanged(GmAbilitiesChanged {
        entity_id,
        player_id,
        change,
        added,
        removed,
        training_points: write.training_points,
    });
    let sent = match cell_tx {
        Some(tx) => tx.send(reply).await.is_ok(),
        None => false,
    };
    if !sent {
        tracing::error!(
            target: "abilities",
            event = "gm_ability_bulk",
            decision_outcome = "mirror_send_failed",
            entity_id,
            account_id,
            player_id,
            cmd,
            "GM bulk ability change: no cell channel; the change shows after relog"
        );
        let text = format!("{cmd}: saved; it shows after you relog");
        send_gm_feedback_to_client(entity_id, &text, transport, connected, entity_to_addr).await;
    }
}
