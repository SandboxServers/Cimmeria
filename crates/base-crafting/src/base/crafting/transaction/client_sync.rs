//! Post-commit client updates for the crafting transaction, and the full
//! inventory resync a refusal sends.
//!
//! `onUpdateItem` is upsert-only: a stack that no longer exists must be
//! removed with `onRemoveItem`, or the client keeps showing it.

use cimmeria_entity::known_names;
use std::sync::Arc;

use cimmeria_entity::crafting::serialize_on_update_discipline;
use cimmeria_entity::inventory::InvItem;
use cimmeria_wire::crafting::known_crafts_args;
use sqlx::PgPool;

use super::CraftApplied;
use crate::base::crafting::session::InductionEnv;
use crate::base::crafting::telemetry::{send_to_player, JobIds};
use crate::mercury::method_idx;

/// The inventory row select of the world-entry load and the generic
/// inventory resync, with an optional `item_id` filter (`$2`, NULL = all).
/// Container 18 (Black Market escrow) is server-held and never sent.
const INVENTORY_ITEMS_SELECT: &str = r#"
SELECT inv.item_id, inv.type_id, inv.stack_size, inv.slot_id, inv.container_id,
       inv.bound, inv.durability, inv.charges,
       COALESCE((
           SELECT array_agg(array_position(enum_range(NULL::resources."EAmmoType"), ammo) - 1 ORDER BY ord)
           FROM unnest(ri.ammo_types) WITH ORDINALITY AS ammo_values(ammo, ord)
       ), ARRAY[]::integer[]) AS ammo_type_ids,
       CASE WHEN ri.default_ammo_type IS NULL THEN 0
            ELSE array_position(enum_range(NULL::resources."EAmmoType"), ri.default_ammo_type) - 1
       END AS cur_ammo_type_id
FROM sgw_inventory inv
LEFT JOIN resources.items ri ON ri.item_id = inv.type_id
WHERE inv.character_id = $1 AND inv.container_id <> 18
  AND ($2::int4[] IS NULL OR inv.item_id = ANY($2))
ORDER BY inv.container_id, inv.slot_id
"#;

#[derive(sqlx::FromRow)]
struct InventoryRow {
    item_id: i32,
    type_id: i32,
    stack_size: i32,
    slot_id: i32,
    container_id: i32,
    bound: bool,
    durability: i32,
    charges: i32,
    ammo_type_ids: Vec<i32>,
    cur_ammo_type_id: i32,
}

/// Tell the client what a committed transaction changed: `onRemoveItem`
/// for the drained stacks, one `onUpdateItem` for the shrunk and granted
/// stacks, `onUpdateDiscipline` per changed discipline, and
/// `onUpdateKnownCrafts` (the whole list) when blueprints were taught.
///
/// If any item notification failed (a send, or the item read), the client
/// gets one recovery pass after the rest: `onRemoveItem` for the drained
/// stacks again, then a full `onUpdateItem`. The removal is repeated
/// because `onUpdateItem` only adds and updates: a full list alone would
/// leave a drained stack on screen.
pub(super) async fn send_applied(
    env: &InductionEnv,
    pool: &Arc<PgPool>,
    ids: &JobIds,
    applied: &CraftApplied,
) {
    let drained: Vec<i32> = applied.drained().iter().map(|d| d.item_id).collect();
    let mut items_ok = true;
    if !drained.is_empty() {
        items_ok &= send_to_player(
            env,
            ids,
            method_idx::ON_REMOVE_ITEM,
            &remove_item_args(&drained),
            "remove_item",
        )
        .await;
    }
    let updated = applied.updated_item_ids();
    if !updated.is_empty() {
        items_ok &= send_items(env, pool, ids, Some(&updated)).await;
    }
    if !items_ok {
        if !drained.is_empty() {
            send_to_player(
                env,
                ids,
                method_idx::ON_REMOVE_ITEM,
                &remove_item_args(&drained),
                "resync_remove",
            )
            .await;
        }
        resync_inventory(env, pool, ids).await;
    }
    for e in &applied.expertise {
        let args = serialize_on_update_discipline(e.discipline_id, e.after);
        send_to_player(
            env,
            ids,
            method_idx::ON_UPDATE_DISCIPLINE,
            &args,
            "update_discipline",
        )
        .await;
    }
    if let Some(learned) = &applied.blueprints {
        send_to_player(
            env,
            ids,
            method_idx::ON_UPDATE_KNOWN_CRAFTS,
            &known_crafts_args(&learned.blueprint_ids),
            "known_crafts",
        )
        .await;
    }
}

/// Send every item the player holds in one `onUpdateItem`, so the client's
/// bags match the database again after a refused craft. A failure is
/// logged by the read or the send.
pub async fn resync_inventory(env: &InductionEnv, pool: &Arc<PgPool>, ids: &JobIds) {
    send_items(env, pool, ids, None).await;
}

/// `onRemoveItem(ARRAY<INT32> ItemIdList)` arguments.
pub(super) fn remove_item_args(item_ids: &[i32]) -> Vec<u8> {
    let mut args = Vec::with_capacity(4 + 4 * item_ids.len());
    args.extend_from_slice(&(item_ids.len() as u32).to_le_bytes());
    for id in item_ids {
        args.extend_from_slice(&id.to_le_bytes());
    }
    args
}

/// Read the player's items (all, or the `filter` ids) and send them in one
/// `onUpdateItem`. `false` when the read or the send failed (logged as
/// `client_sync_failed`). Nothing is sent after a failed read, because an
/// empty or partial list would be read as the whole inventory.
async fn send_items(
    env: &InductionEnv,
    pool: &Arc<PgPool>,
    ids: &JobIds,
    filter: Option<&[i32]>,
) -> bool {
    let rows: Vec<InventoryRow> = match sqlx::query_as(INVENTORY_ITEMS_SELECT)
        .bind(ids.player_id)
        .bind(filter)
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(
                target: "crafting",
                event = "client_sync_failed",
                job_id = ids.job_id, // nt:id-only induction job counter, unnamed
                account_id = ids.account_id,
                account_name = known_names::account_name(ids.account_id),
                player_id = ids.player_id,
                player_name = known_names::player_name(ids.player_id),
                gm_entity_id = ids.gm_entity_id,
                gm_entity_name = ids.gm_name,
                entity_id = ids.entity_id,
                entity_name = known_names::player_name(ids.player_id),
                what = if filter.is_some() { "update_item" } else { "resync" },
                reason = "inventory_read_failed",
                error_class = crate::base::crafting::telemetry::sql_error_class(&e),
                error = %e,
                "crafting inventory read failed -- the client keeps stale bags until the next update"
            );
            return false;
        }
    };
    let mut args = Vec::with_capacity(4 + rows.len() * 48);
    args.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    for row in &rows {
        InvItem {
            id: row.item_id,
            dbid: row.type_id,
            stack_size: row.stack_size,
            // The wire slot is 1-based.
            slot_id: row.slot_id + 1,
            container_id: row.container_id,
            is_bound: row.bound,
            durability: row.durability,
            ammo_types: row.ammo_type_ids.clone(),
            cur_ammo_type: row.cur_ammo_type_id,
            charges: row.charges,
        }
        .serialize(&mut args);
    }
    let what = if filter.is_some() {
        "update_item"
    } else {
        "resync"
    };
    send_to_player(env, ids, method_idx::ON_UPDATE_ITEM, &args, what).await
}
