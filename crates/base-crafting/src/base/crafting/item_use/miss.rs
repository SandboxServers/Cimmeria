//! Crafting uses whose instance the item-use path cannot find for the
//! player: another character's item, or one this player already used.
//!
//! The item-use path looks the instance up by owner, so a miss carries no
//! type. [`is_crafting_miss`] decides whether the miss was a crafting item,
//! so it gets the crafting refusal ("That item is no longer in your
//! inventory.") and its telemetry instead of the silent ordinary-item WARN:
//!
//! - an instance this process consumed through a crafting use (a replayed
//!   press after the commit; the row is gone, so only this record knows);
//! - an existing instance of a crafting item owned by someone else.

use cimmeria_entity::known_names;
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};

use sqlx::PgPool;

/// How many consumed instances are remembered, across all players. A replay
/// arrives within moments of the use it repeats, so a short history is
/// enough; instance ids come from a sequence and are never reused, so an
/// entry can never match a different item.
const REMEMBERED: usize = 1024;

fn consumed() -> &'static Mutex<VecDeque<(i32, i32)>> {
    static CONSUMED: OnceLock<Mutex<VecDeque<(i32, i32)>>> = OnceLock::new();
    CONSUMED.get_or_init(|| Mutex::new(VecDeque::with_capacity(REMEMBERED)))
}

/// Record that `player_id` used up instance `item_id` through a crafting use.
pub fn remember_consumed(player_id: i32, item_id: i32) {
    let Ok(mut consumed) = consumed().lock() else {
        return;
    };
    if consumed.len() == REMEMBERED {
        consumed.pop_front();
    }
    consumed.push_back((player_id, item_id));
}

fn was_consumed(player_id: i32, item_id: i32) -> bool {
    consumed()
        .lock()
        .map(|c| c.contains(&(player_id, item_id)))
        .unwrap_or(false)
}

/// Whether a use of `item_id` that found no instance owned by `player_id`
/// was a use of a crafting item. A failed read answers `false`, so the miss
/// keeps the ordinary-item behavior. `account_id` and `entity_id` only
/// label that WARN.
pub async fn is_crafting_miss(
    pool: &PgPool,
    account_id: Option<u32>,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
) -> bool {
    if was_consumed(player_id, item_id) {
        return true;
    }
    let found: Result<bool, sqlx::Error> = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM sgw_inventory inv \
           JOIN resources.crafting_item_effects e ON e.item_id = inv.type_id \
          WHERE inv.item_id = $1)",
    )
    .bind(item_id)
    .fetch_one(pool)
    .await;
    match found {
        Ok(found) => found,
        Err(e) => {
            tracing::warn!(
                target: "crafting",
                event = "lookup_failed",
                verb = super::VERB,
                phase = "item_miss",
                account_id,
                account_name = known_names::account_name(account_id),
                player_id,
                player_name = known_names::player_name(player_id),
                entity_id,
                entity_name = known_names::player_name(player_id),
                item_id, // nt:id-only instance id, type unread yet
                error = %e,
                "crafting item miss lookup failed; treated as an ordinary item"
            );
            false
        }
    }
}
