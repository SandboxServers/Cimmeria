//! The crafting consume-and-grant transaction.
//!
//! An induction's work is one database transaction: lock the player row,
//! lock and re-check every item instance the request named, consume the
//! inputs by design from the main and crafting bags, grant the products,
//! apply expertise, commit. Any refusal or error rolls the whole
//! transaction back, so nothing is ever half-applied. After the commit the
//! client is told exactly what changed (`onRemoveItem` for drained stacks,
//! `onUpdateItem` for the rest, `onUpdateDiscipline` per discipline); after
//! a refusal it gets the feedback line and a full inventory resync, which
//! restores the slots the crafting pages empty on confirm.
//!
//! The request names one instance per component type (the last stack the
//! client found), so a requirement that spans several stacks can only be
//! met by consuming by design; the named instances are checked, not
//! trusted.
//!
//! - [`consume`]: the player lock, the named-instance check and consumption
//!   by design.
//! - [`grant`]: placing products (stack merge, else free slots).
//! - [`learn`]: adding the blueprints a plan teaches.
//! - [`client_sync`]: the post-commit client updates and the resync.
//! - [`applied`]: what a commit changed, with before and after quantities
//!   (the `completed` event's fields).
//! - [`failure`]: why a transaction did not commit, and `persist_failed`.

use sqlx::{PgPool, Postgres, Transaction};

use super::feedback::{reject_at_completion, CraftReject};
use super::session::InductionEnv;
use super::telemetry::JobIds;
use crate::base::outbox::{self, CellOutboxPayload};

mod applied;
mod client_sync;
mod consume;
mod failure;
mod grant;
mod knowledge;
mod learn;
mod plan;

#[cfg(test)]
mod tests;

pub use applied::{BlueprintsLearned, ConsumedStack, CraftApplied, ExpertiseChange, GrantedStack};
pub use client_sync::resync_inventory;
pub use failure::CraftTxError;
pub use plan::{CraftTransaction, NamedItem, RequiredKnowledge};

use failure::{at, expect_rows, log_persist_failed};

/// The main bag and the crafting bag: the only bags crafting reads inputs
/// from.
pub const CRAFTING_INPUT_BAGS: [i32; 2] = [
    cimmeria_cell_catalog::item_placement::INV_CRAFTING,
    cimmeria_cell_catalog::item_placement::INV_MAIN,
];

/// Run `plan` as one transaction and commit it. Returns what changed and
/// the outbox rows to dispatch to the cell. Nothing is sent to the client
/// and nothing is logged for a failure; [`apply_craft_transaction`] does
/// both.
pub async fn run_craft_transaction(
    pool: &PgPool,
    ids: &JobIds,
    plan: &CraftTransaction,
) -> Result<(CraftApplied, Vec<(i64, CellOutboxPayload)>), CraftTxError> {
    plan.check_shape()?;
    let mut tx = pool.begin().await.map_err(at("begin"))?;
    match apply_in_tx(&mut tx, ids, plan).await {
        Ok(done) => {
            tx.commit().await.map_err(at("commit"))?;
            Ok(done)
        }
        Err(e) => {
            // Defensible silent rollback: the originating error is
            // returned and logged by the caller.
            let _ = tx.rollback().await;
            Err(e)
        }
    }
}

async fn apply_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    ids: &JobIds,
    plan: &CraftTransaction,
) -> Result<(CraftApplied, Vec<(i64, CellOutboxPayload)>), CraftTxError> {
    let player_id = ids.player_id;
    // Lock order: advisory locks first (the player-wide move lock, then
    // each bag), then inventory rows. The player row is read, and locked
    // only by a plan that teaches blueprints, after every inventory row.
    // See `grant::lock_containers`, `consume::check_player` and
    // `learn::teach_blueprints`.
    let placements = grant::resolve(tx, &plan.grant).await?;
    grant::lock_containers(tx, player_id, &placements).await?;
    consume::check_player(tx, player_id).await?;
    let mut named = consume::check_named_items(tx, player_id, &plan.named_items).await?;

    let mut applied = CraftApplied::default();
    for &(item_id, quantity) in &plan.consume_named {
        consume::consume_instance(tx, ids, &mut named, item_id, quantity, &mut applied).await?;
    }
    for &(design_id, quantity) in &plan.consume {
        consume::consume_design(tx, ids, design_id, quantity, &mut applied).await?;
    }
    for placement in placements {
        grant::place(tx, ids, placement, &mut applied).await?;
    }
    if !plan.learn_blueprints.is_empty() {
        learn::teach_blueprints(tx, ids, &plan.learn_blueprints, &mut applied).await?;
    }
    // After learning, so a plan that does both takes the player row
    // FOR UPDATE once instead of upgrading a share lock.
    if let Some(required) = plan.required_knowledge {
        knowledge::check_knowledge(tx, player_id, required).await?;
    }
    for &(discipline_id, delta) in &plan.expertise {
        let before: Option<i32> = sqlx::query_scalar(
            "SELECT expertise FROM sgw_player_discipline_expertise WHERE player_id = $1 AND discipline_id = $2 FOR UPDATE",
        )
        .bind(player_id)
        .bind(discipline_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(at("expertise"))?;
        // Only disciplines the player knows gain expertise.
        let Some(before) = before else { continue };
        let after = (before + delta).clamp(0, 100);
        let done = sqlx::query(
            "UPDATE sgw_player_discipline_expertise SET expertise = $3 WHERE player_id = $1 AND discipline_id = $2",
        )
        .bind(player_id)
        .bind(discipline_id)
        .bind(after)
        .execute(&mut **tx)
        .await
        .map_err(at("expertise"))?;
        expect_rows(ids, "expertise", done, 1)?;
        applied.expertise.push(ExpertiseChange {
            discipline_id,
            before,
            after,
        });
    }

    let mut pending = Vec::new();
    for d in applied.drained() {
        let payload = CellOutboxPayload::InventoryItemRemoved {
            item_id: d.item_id,
            source_container_id: d.container_id,
        };
        let id = outbox::enqueue_in_tx(tx, ids.entity_id, &payload)
            .await
            .map_err(at("outbox"))?;
        pending.push((id, payload));
    }
    for g in &applied.granted {
        let payload = CellOutboxPayload::InventoryItemGranted {
            item_id: g.design_id,
            container_id: g.container_id,
            slot_id: g.slot_id,
            quantity: g.quantity(),
        };
        let id = outbox::enqueue_in_tx(tx, ids.entity_id, &payload)
            .await
            .map_err(at("outbox"))?;
        pending.push((id, payload));
    }
    Ok((applied, pending))
}

/// Run `plan` and bring the client up to date. On success the client gets
/// the item and discipline updates and the cell its inventory events. On a
/// refusal or a database error nothing is applied, and the player gets
/// the feedback line and a full inventory resync. Every rollback that is
/// not a game-rule refusal logs `persist_failed` (WARN) with its `phase`.
pub async fn apply_craft_transaction(
    env: &InductionEnv,
    ids: &JobIds,
    plan: &CraftTransaction,
) -> Result<CraftApplied, CraftReject> {
    let Some(pool) = env.db_pool.as_ref() else {
        log_persist_failed(
            ids,
            &CraftTxError::Invalid {
                phase: "begin",
                reason: "no_database",
            },
        );
        send_reject(env, ids, &CraftReject::InductionFailed).await;
        return Err(CraftReject::InductionFailed);
    };
    match run_craft_transaction(pool, ids, plan).await {
        Ok((applied, pending)) => {
            client_sync::send_applied(env, pool, ids, &applied).await;
            if let Some(cell_tx) = &env.cell_tx {
                for (outbox_id, payload) in pending {
                    outbox::try_dispatch_now(
                        pool.as_ref(),
                        cell_tx,
                        outbox_id,
                        ids.entity_id,
                        payload,
                    )
                    .await;
                }
            }
            Ok(applied)
        }
        Err(err) => {
            log_persist_failed(ids, &err);
            let why = match err {
                CraftTxError::Rejected(why) => why,
                _ => CraftReject::InductionFailed,
            };
            send_reject(env, ids, &why).await;
            resync_inventory(env, pool, ids).await;
            Err(why)
        }
    }
}

async fn send_reject(env: &InductionEnv, ids: &JobIds, why: &CraftReject) {
    reject_at_completion(ids, why, env.client()).await;
}
