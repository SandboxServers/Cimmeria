//! Using a crafting item: a Blueprint item teaches its blueprint(s), a
//! Racial Paradigm Guide raises its paradigm by one, to at most 10.
//!
//! The client sends the ordinary `useItem(item_id, target_id)`. The item-use
//! path recognises the item by its `resources.crafting_item_effects` rows
//! and hands it here instead of firing the content engine's `OnItemUse`, so
//! no content chain can consume the same item a second time.
//!
//! One transaction ([`transaction::use_item_in_db`]) locks the item and the
//! player's crafting state, decides the use ([`rule::decide`]), consumes one
//! of the item and saves the new crafting state, all or nothing. After the
//! commit the client gets `onUpdateKnownCrafts` (139, the full list) or
//! `onUpdateRacialParadigmLevel` (138); the caller then syncs the inventory.
//! A use that would change nothing (every blueprint known, a paradigm at
//! 10), an item that is gone or not in a carried bag, is refused with a
//! text line and consumes nothing. `target_id` is never read: the item's
//! effect comes from the seed and always applies to the user.

pub mod rule;
pub mod transaction;

pub use rule::{Applied, BlueprintChange, ItemEffects, MAX_RACIAL_PARADIGM_LEVEL};
pub use transaction::{Committed, UseFailure, CARRIED_CONTAINERS};

use super::feedback::{reject, CraftReject};
use super::request::CraftCtx;
use super::sync::{push_known_crafts, push_paradigm};
use super::telemetry::{account_id_of, record_request, sql_error_class, Outcome};
use crate::base::outbox::CellOutboxPayload;
use transaction::use_item_in_db;

/// The client method name: the `verb` field and metric label.
pub const VERB: &str = "useItem";

/// The text subject for this path's `Unavailable` rejection.
const ACTION: &str = "Using that item";

/// What the caller must do for the inventory after a committed use.
#[derive(Debug)]
pub struct ConsumedItem {
    /// The inventory instance used.
    pub item_id: i32,
    /// The bag it was used from.
    pub container_id: i32,
    /// The instance is gone (its last one was used): the client needs
    /// `onRemoveItem` as well as the inventory update.
    pub removed_all: bool,
    /// The cell notification for a removed instance, enqueued with the
    /// commit; dispatch it now.
    pub outbox: Option<(i64, CellOutboxPayload)>,
}

/// Use crafting item instance `item_id` for `player_id`. Events:
/// `request`, then `blueprint_learned` or `paradigm_raised` on success,
/// `rejected` (from [`reject`]) on a refusal, `persist_failed` or
/// `lookup_failed` (WARN) on a failure. `Some` only when an item was
/// consumed.
#[tracing::instrument(name = "crafting.request", level = "info", skip_all, fields(verb = VERB))]
pub async fn handle_crafting_item_use(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    ctx: &CraftCtx<'_>,
) -> Option<ConsumedItem> {
    let client = ctx.client();
    let account_id = account_id_of(entity_id, ctx.connected, ctx.entity_to_addr);
    tracing::info!(
        target: "crafting",
        event = "request",
        verb = VERB,
        account_id,
        player_id,
        entity_id,
        item_id,
        "crafting item used"
    );
    let unavailable = CraftReject::Unavailable { action: ACTION };
    let Some(pool) = ctx.db_pool else {
        tracing::warn!(
            target: "crafting",
            event = "persist_failed",
            verb = VERB,
            phase = "no_pool",
            account_id,
            player_id,
            entity_id,
            item_id,
            "crafting item use: no database pool"
        );
        reject(VERB, entity_id, player_id, &unavailable, client).await;
        return None;
    };

    let committed = match use_item_in_db(pool, entity_id, player_id, item_id).await {
        Ok(Ok(committed)) => committed,
        Ok(Err(why)) => {
            reject(VERB, entity_id, player_id, &why, client).await;
            return None;
        }
        Err(failure) => {
            warn_failure(&failure, account_id, player_id, entity_id, item_id);
            reject(VERB, entity_id, player_id, &unavailable, client).await;
            return None;
        }
    };

    let Committed {
        applied,
        type_id,
        container_id,
        qty_before,
        qty_after,
        blueprint_ids,
        outbox,
    } = committed;
    let consumed = format!("{item_id}:{type_id}:{qty_before}→{qty_after}");
    record_request(VERB, Outcome::Accepted);
    match applied {
        Applied::Learned {
            blueprints,
            known_before,
            known_after,
        } => {
            tracing::info!(
                target: "crafting",
                event = "blueprint_learned",
                verb = VERB,
                account_id,
                player_id,
                entity_id,
                item_id,
                type_id,
                blueprints = %Applied::blueprint_field(&blueprints),
                known_before,
                known_after,
                consumed = %consumed,
                "blueprint item used"
            );
            push_known_crafts(entity_id, player_id, &blueprint_ids, client).await;
        }
        Applied::Raised {
            paradigm_id,
            level_before,
            level_after,
        } => {
            tracing::info!(
                target: "crafting",
                event = "paradigm_raised",
                verb = VERB,
                account_id,
                player_id,
                entity_id,
                item_id,
                type_id,
                paradigm_id,
                level_before,
                level_after,
                consumed = %consumed,
                "racial paradigm guide used"
            );
            push_paradigm(entity_id, player_id, paradigm_id, level_after, client).await;
        }
    }
    Some(ConsumedItem {
        item_id,
        container_id,
        removed_all: qty_after == 0,
        outbox,
    })
}

fn warn_failure(
    failure: &UseFailure,
    account_id: Option<u32>,
    player_id: i32,
    entity_id: u32,
    item_id: i32,
) {
    // A missing or malformed effect row is a lookup miss; everything else
    // is a rolled-back write.
    let event = if failure.phase == "item_effects" && failure.error.is_none() {
        "lookup_failed"
    } else {
        "persist_failed"
    };
    match &failure.error {
        Some(e) => tracing::warn!(
            target: "crafting",
            event,
            verb = VERB,
            phase = failure.phase,
            reason = failure.reason,
            account_id,
            player_id,
            entity_id,
            item_id,
            error_class = sql_error_class(e),
            error = %e,
            "crafting item use: transaction failed, rolled back; nothing was used"
        ),
        None => tracing::warn!(
            target: "crafting",
            event,
            verb = VERB,
            phase = failure.phase,
            reason = failure.reason,
            rows_affected = failure.rows_affected,
            expected = failure.expected,
            account_id,
            player_id,
            entity_id,
            item_id,
            "crafting item use: refused by the data, rolled back; nothing was used"
        ),
    }
}

#[cfg(test)]
mod tests;
