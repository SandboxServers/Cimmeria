//! The `handle_grant_item` persistence + client-sync handler.
//!
//! Extracted from `grant/mod.rs` (issue #529). Carries the full grant flow:
//! advisory-lock → stack-merge fast path → fresh-slot reserve/INSERT →
//! bandolier reconcile → outbox enqueue/commit → inventory resync → bandolier
//! cell notification → equipment appearance refresh. Pure code movement; the
//! function body is byte-identical to the original.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;
use tokio::sync::mpsc;

use super::super::vendor::serializers::reserve_free_inventory_slots;
use super::appearance::refresh_player_appearance;
use super::core::send_full_inventory_update;
use crate::base::gm_feedback::send_gm_feedback_to_client;
use crate::base::outbox::{self, CellOutboxPayload};
use crate::base::{helpers, ConnectedClientState};
use crate::cell::messages::BaseToCellMsg;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// Persist an item grant to inventory and sync client appearance.
#[tracing::instrument(
    name = "inventory.grant_item",
    level = "info",
    skip_all,
    fields(entity_id, player_id, item_id, container_id, count)
)]
pub async fn handle_grant_item(
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    container_id: i32,
    count: i32,
    notify_gm: bool,
    db_pool: &Option<Arc<PgPool>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    tracing::debug!(
        entity_id,
        player_id,
        item_id,
        container_id,
        count,
        cell_tx_present = cell_tx.is_some(),
        "handle_grant_item: entered"
    );
    let pool = match db_pool {
        Some(p) => p,
        None => {
            tracing::debug!(player_id, item_id, "GrantItem: no DB pool");
            return;
        }
    };

    let mut db_tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(player_id, item_id, "GrantItem: begin tx failed: {e}");
            return;
        }
    };

    // Acquire the per-(player, container) advisory lock up front so the
    // merge probe and the eventual reserve/INSERT see the same snapshot.
    // `reserve_free_inventory_slots` re-acquires the same lock internally;
    // `pg_advisory_xact_lock` is idempotent within a single transaction,
    // so doing it here too is a no-op safety belt rather than a double
    // wait. Without this, two concurrent grants for the same stackable
    // item could each decide "no existing stack, allocate a new slot"
    // and produce two single-stack rows instead of merging into one.
    if let Err(e) = sqlx::query("SELECT pg_advisory_xact_lock($1, $2)")
        .bind(player_id)
        .bind(container_id)
        .execute(&mut *db_tx)
        .await
    {
        let _ = db_tx.rollback().await;
        tracing::error!(
            player_id,
            item_id,
            container_id,
            "GrantItem: advisory lock failed: {e}"
        );
        return;
    }

    // ── Stack-merge fast path ────────────────────────────────────────
    //
    // Before reserving a fresh slot, look for an existing
    // same-type row in the same container with room for the full
    // count. When the row exists, UPDATE its `stack_size` and
    // short-circuit — no slot reservation, no INSERT, no
    // bandolier_slot reconcile (the merge target is already the
    // active slot if it ever was). This is what makes consumables
    // like the Health Slappack stack to their `max_stack_size`
    // instead of eating one inventory slot per pickup.
    //
    // Gating predicates the WHERE clause enforces:
    //   - `bound = false` — bound items are 1:1 with their owner
    //     (no merging across two bound rows of the same type).
    //   - `ri.max_stack_size > 1` — non-stackable item defs stay
    //     on the one-row-per-pickup path even if a stale identical
    //     row exists.
    //   - `inv.stack_size + $count <= ri.max_stack_size` — refuse
    //     partial merges. A pickup of 3 against a stack at 8/10
    //     skips merge and allocates a new slot rather than
    //     splitting (8 stays, 2 go to one new slot, 1 leftover);
    //     the simpler "all-or-nothing into a single stack" rule
    //     keeps the wire-level `onUpdateItem` set predictable and
    //     dodges a tricky "two rows from one grant" outbox
    //     enqueue. A future enhancement can split if needed.
    //   - Charges and ammo_type are intentionally NOT in the
    //     match criteria. Consumable charges are typically 0
    //     (server tracks the count via stack_size, not via per-
    //     instance charges); ammo_type only matters for weapons,
    //     which are non-stackable.
    //
    // `FOR UPDATE` locks the target row for the rest of the
    // transaction so the subsequent UPDATE can't race with a
    // concurrent vendor sell of the same stack. The advisory
    // lock above already serialises grants per container, but a
    // vendor sell takes a different lock (per-item), so the row
    // lock is the belt-and-braces.
    #[derive(sqlx::FromRow)]
    struct MergeCandidate {
        item_id: i32,
        slot_id: i32,
    }
    let merge_candidate: Option<MergeCandidate> = match sqlx::query_as(
        "SELECT inv.item_id, inv.slot_id \
           FROM sgw_inventory inv \
           JOIN resources.items ri ON inv.type_id = ri.item_id \
          WHERE inv.character_id = $1 \
            AND inv.container_id = $2 \
            AND inv.type_id = $3 \
            AND inv.bound = false \
            AND ri.max_stack_size > 1 \
            AND inv.stack_size + $4 <= ri.max_stack_size \
          ORDER BY inv.slot_id \
          LIMIT 1 \
          FOR UPDATE",
    )
    .bind(player_id)
    .bind(container_id)
    .bind(item_id)
    .bind(count)
    .fetch_optional(&mut *db_tx)
    .await
    {
        Ok(c) => c,
        Err(e) => {
            let _ = db_tx.rollback().await;
            tracing::error!(
                player_id,
                item_id,
                container_id,
                "GrantItem: merge candidate lookup failed: {e}"
            );
            return;
        }
    };

    if let Some(target) = merge_candidate {
        // Merge into the existing stack. The UPDATE is keyed by
        // item_id (the per-row instance id of sgw_inventory) so it can
        // only touch the exact row we locked above.
        if let Err(e) =
            sqlx::query("UPDATE sgw_inventory SET stack_size = stack_size + $1 WHERE item_id = $2")
                .bind(count)
                .bind(target.item_id)
                .execute(&mut *db_tx)
                .await
        {
            let _ = db_tx.rollback().await;
            tracing::error!(
                player_id,
                item_id,
                target_item_id = target.item_id,
                "GrantItem: stack merge UPDATE failed: {e}"
            );
            return;
        }
        tracing::info!(
            player_id,
            item_id,
            container_id,
            slot = target.slot_id,
            count,
            "GrantItem: merged into existing stack"
        );

        // Stackable items are non-bandolier consumables in
        // practice (weapons have `max_stack_size = 1`), so the
        // bandolier-active-slot reconcile is irrelevant on the
        // merge path. Skip straight to outbox enqueue + commit +
        // inventory resync.
        let outbox_payload = CellOutboxPayload::InventoryItemGranted {
            item_id,
            container_id,
            slot_id: target.slot_id,
            quantity: count,
        };
        let outbox_id = match outbox::enqueue_in_tx(&mut db_tx, entity_id, &outbox_payload).await {
            Ok(id) => id,
            Err(e) => {
                let _ = db_tx.rollback().await;
                tracing::error!(
                    player_id,
                    item_id,
                    "GrantItem (merge): outbox enqueue failed, aborting: {e}"
                );
                return;
            }
        };
        if let Err(e) = db_tx.commit().await {
            tracing::error!(player_id, item_id, "GrantItem (merge): commit failed: {e}");
            return;
        }
        let total_items = send_full_inventory_update(
            entity_id,
            player_id,
            pool,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
        tracing::debug!(
            entity_id,
            player_id,
            item_id,
            total_items,
            "GrantItem (merge): sent full onUpdateItem to client"
        );
        if let Some(tx) = cell_tx {
            outbox::try_dispatch_now(pool.as_ref(), tx, outbox_id, entity_id, outbox_payload).await;
        }
        // Definitive GM feedback (merge path) — write committed.
        if notify_gm {
            send_gm_feedback_to_client(
                entity_id,
                &format!("gmGiveItem: gave {count}x item {item_id}"),
                transport,
                connected,
                entity_to_addr,
            )
            .await;
        }
        return;
    }

    // Reserve a free slot via the same hole-filling helper used by vendor purchase.
    // (reserve_free_inventory_slots takes a per-(player, container) advisory lock.)
    let next_slot: i32 =
        match reserve_free_inventory_slots(&mut db_tx, player_id, container_id, 1).await {
            Ok(Some(slots)) => match slots.into_iter().next() {
                Some(s) => s,
                None => {
                    let _ = db_tx.rollback().await;
                    tracing::warn!(
                        player_id,
                        item_id,
                        container_id,
                        "GrantItem: reserve returned empty"
                    );
                    return;
                }
            },
            Ok(None) => {
                let _ = db_tx.rollback().await;
                tracing::warn!(
                    player_id,
                    item_id,
                    container_id,
                    "GrantItem: container full"
                );
                return;
            }
            Err(e) => {
                let _ = db_tx.rollback().await;
                tracing::error!(
                    player_id,
                    item_id,
                    container_id,
                    "GrantItem: slot reserve failed: {e}"
                );
                return;
            }
        };

    // Default charges to the item's full charge capacity (consumables/abilities ammo)
    // rather than always inserting `charges = 0`. A DB error here aborts the grant
    // — we don't want a transient timeout to silently produce a depleted item.
    let default_charges: i32 = match sqlx::query_scalar::<_, Option<i32>>(
        "SELECT charges FROM resources.items WHERE item_id = $1",
    )
    .bind(item_id)
    .fetch_optional(&mut *db_tx)
    .await
    {
        Ok(Some(Some(c))) => c,
        Ok(Some(None)) | Ok(None) => 0,
        Err(e) => {
            let _ = db_tx.rollback().await;
            tracing::error!(player_id, item_id, "GrantItem: charges lookup failed: {e}");
            return;
        }
    };

    // Pull ammo_type / ammo_types / charges from resources.items so a granted
    // weapon arrives with its real ammo configuration. Without this, the
    // defaults are AMMO_NONE / [] / 0, which makes ranged grants unusable
    // until the player manually changes ammo. INSERT…SELECT keeps this in
    // a single round-trip and gives us COALESCE for ammo_type so items with
    // a NULL default still get a sane sentinel rather than NULL (the column
    // is NOT NULL in the schema).
    // `RETURNING item_id` hands back the freshly-allocated `sgw_inventory.item_id`
    // per-row instance id, which the bandolier fast-path below needs
    // as the ammo-persist TOCTOU guard. The design id is the `item_id` param; the
    // instance id is unique per physical row and is what distinguishes two copies
    // of the same weapon design occupying the bandolier over time.
    let result = sqlx::query_scalar::<_, i32>(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges, \
             ammo_type, ammo_types, ammo, flags) \
         SELECT $1, ri.item_id, $2, $3, $4, false, 100, $5, \
                COALESCE(ri.default_ammo_type, 'AMMO_NONE'::resources.\"EAmmoType\"), \
                ri.ammo_types, ri.charges, 0 \
         FROM resources.items ri WHERE ri.item_id = $6 \
         RETURNING item_id",
    )
    .bind(player_id)
    .bind(count)
    .bind(next_slot)
    .bind(container_id)
    .bind(default_charges)
    .bind(item_id)
    .fetch_one(&mut *db_tx)
    .await;

    let instance_id: i32 = match result {
        Ok(id) => {
            tracing::debug!(
                player_id,
                item_id,
                instance_id = id,
                container_id,
                slot = next_slot,
                charges = default_charges,
                "Item persisted to inventory"
            );
            id
        }
        Err(e) => {
            let _ = db_tx.rollback().await;
            tracing::error!(player_id, item_id, "Failed to persist item: {e}");
            return;
        }
    };

    // For bandolier grants, persist `bandolier_slot` in the SAME transaction so
    // the inventory insert and the active-slot move are atomic — a separate
    // post-commit UPDATE could silently leave the player with the new item
    // visible but the active slot still pointing at the old one.
    //
    // Only adopt the new slot when the player's *previous* selection no longer
    // points at a real item (e.g., empty bandolier, or the previously-selected
    // slot is now also gone). This matches the slot-preservation behavior of
    // `sync_bandolier_after_inventory_change` so a loot drop or quest reward
    // doesn't hot-swap a player's preferred weapon mid-combat.
    // Track whether the bandolier_slot UPDATE actually adopted the new slot
    // (i.e., rows_affected == 1). If the WHERE-NOT-EXISTS guard preserved the
    // player's existing selection, downstream messages must NOT advertise the
    // new slot as active — otherwise the cell mirrors the new active slot
    // even though the DB still points at the old one (desync).
    let mut bandolier_became_active = false;
    if container_id == 3 {
        let res = sqlx::query(
            "UPDATE sgw_player p \
                SET bandolier_slot = $1 \
              WHERE p.player_id = $2 \
                AND NOT EXISTS ( \
                  SELECT 1 FROM sgw_inventory inv \
                  WHERE inv.character_id = p.player_id \
                    AND inv.container_id = 3 \
                    AND inv.slot_id = p.bandolier_slot \
                )",
        )
        .bind(next_slot)
        .bind(player_id)
        .execute(&mut *db_tx)
        .await;

        match res {
            Ok(r) => {
                bandolier_became_active = r.rows_affected() == 1;
                tracing::debug!(
                    player_id,
                    slot_id = next_slot,
                    swapped = r.rows_affected() == 1,
                    "GrantItem: bandolier_slot reconciled (swapped only if previous selection vacant)"
                );
            }
            Err(e) => {
                let _ = db_tx.rollback().await;
                tracing::error!(
                    player_id,
                    slot_id = next_slot,
                    "GrantItem: bandolier_slot UPDATE failed inside tx, aborting grant: {e}"
                );
                return;
            }
        }
    }

    // Enqueue the cell-notification BEFORE commit so the outbox row and the
    // inventory mutation become visible atomically. After commit, try the
    // in-process dispatch; if the cell receiver is gone, the row stays
    // undelivered for the background drainer to retry.
    let outbox_payload = CellOutboxPayload::InventoryItemGranted {
        item_id,
        container_id,
        slot_id: next_slot,
        quantity: count,
    };
    let outbox_id = match outbox::enqueue_in_tx(&mut db_tx, entity_id, &outbox_payload).await {
        Ok(id) => id,
        Err(e) => {
            // Outbox INSERT failed — abort the grant rather than commit
            // an inventory mutation we can't durably notify the cell about.
            // The player retries the grant trigger, which is idempotent at
            // the chain level.
            let _ = db_tx.rollback().await;
            tracing::error!(
                player_id,
                item_id,
                "GrantItem: outbox enqueue failed, aborting: {e}"
            );
            return;
        }
    };

    if let Err(e) = db_tx.commit().await {
        tracing::error!(player_id, item_id, "GrantItem: commit failed: {e}");
        return;
    }

    let total_items = send_full_inventory_update(
        entity_id,
        player_id,
        pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
    tracing::debug!(
        entity_id,
        player_id,
        item_id,
        total_items,
        "Sent full onUpdateItem to client"
    );

    // Definitive GM feedback (new-slot path) — the grant committed above.
    // Fired here (post-commit, before the bandolier/visual epilogue) so every
    // remaining `return` in this function is after the write has landed.
    if notify_gm {
        send_gm_feedback_to_client(
            entity_id,
            &format!("gmGiveItem: gave {count}x item {item_id}"),
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }

    if let Some(tx) = cell_tx {
        outbox::try_dispatch_now(pool.as_ref(), tx, outbox_id, entity_id, outbox_payload).await;
    }

    let is_equipped = (3..=14).contains(&container_id);
    if !is_equipped {
        return;
    }

    if container_id == 3 {
        // Only broadcast the active-slot witness packet if the DB UPDATE above
        // actually adopted the new slot. If the WHERE-NOT-EXISTS guard kept
        // the player's existing selection, the cell/client must continue to
        // see the previous active slot — broadcasting `next_slot` here would
        // desync the client UI and the cell's `active_bandolier_slot` from
        // the persisted DB value.
        if bandolier_became_active {
            let mut args = Vec::with_capacity(8);
            args.extend_from_slice(&container_id.to_le_bytes());
            args.extend_from_slice(&(next_slot + 1).to_le_bytes());
            helpers::send_to_witness_reliable(
                transport,
                connected,
                entity_to_addr,
                entity_id,
                |key, seq, acks| {
                    build_player_entity_method_packet(
                        key,
                        seq,
                        acks,
                        entity_id,
                        method_idx::ON_ACTIVE_SLOT_UPDATE,
                        &args,
                    )
                },
            )
            .await;
        }

        if let Some(tx) = cell_tx {
            #[derive(sqlx::FromRow)]
            struct BandolierRow {
                item_id: i32,
                clip_size: i32,
                default_ammo_type_id: i32,
            }

            // Resolve the item's clip/ammo metadata. On Ok(None) — the granted
            // item type is not in resources.items, which is data corruption —
            // fall back to a full bandolier resync so combat doesn't see
            // clip_size=0. On Err — transient DB failure — also resync rather
            // than ship known-bad clip/ammo to the cell.
            let row = sqlx::query_as::<_, BandolierRow>(
                r#"
                SELECT item_id, COALESCE(clip_size, 0) AS clip_size,
                       CASE WHEN default_ammo_type IS NULL THEN 0
                            ELSE array_position(enum_range(NULL::resources."EAmmoType"), default_ammo_type) - 1
                       END AS default_ammo_type_id
                FROM resources.items
                WHERE item_id = $1
                "#,
            )
            .bind(item_id)
            .fetch_optional(pool.as_ref())
            .await;

            match row {
                Ok(Some(row)) => {
                    let item = cimmeria_entity::cell_entity::BandolierItem {
                        // The instance PK captured from the grant INSERT's
                        // RETURNING above — this is the ammo-persist TOCTOU
                        // guard, distinct from the design id (`row.item_id`).
                        instance_id,
                        item_id: row.item_id,
                        clip_size: row.clip_size,
                        default_ammo_type: row.default_ammo_type_id,
                        // Stage A: a freshly-granted bandolier item starts
                        // with an empty mag and the default ammo subtype.
                        // Stages B/C will pick up these defaults; today the
                        // shadow scalars on CellEntity still drive fire/reload.
                        current_ammo: 0,
                        cur_ammo_type: row.default_ammo_type_id,
                    };
                    if let Err(e) = tx
                        .send(BaseToCellMsg::UpdateBandolierItem {
                            entity_id,
                            slot_id: next_slot,
                            item,
                            // Only flip the cell's active slot when the DB
                            // UPDATE actually adopted next_slot. Otherwise
                            // the cell would mirror an active slot the DB
                            // disagrees with.
                            make_active: bandolier_became_active,
                        })
                        .await
                    {
                        tracing::warn!(
                            entity_id,
                            player_id,
                            item_id,
                            "GrantItem: cell channel closed sending UpdateBandolierItem: {e}"
                        );
                    }
                }
                Ok(None) | Err(_) => {
                    // Either the item type is missing from resources.items
                    // (data corruption — no clip/ammo metadata available) or
                    // the lookup hit a transient error. In both cases we
                    // delegate to the full sync path, which queries fresh
                    // bandolier state under FOR UPDATE and emits
                    // SyncBandolierItems with whatever the DB actually has.
                    if matches!(row, Err(ref _e)) {
                        if let Err(e) = row {
                            tracing::error!(item_id, "GrantItem: bandolier metadata lookup failed ({e}); falling back to sync_bandolier_after_inventory_change");
                        }
                    } else {
                        tracing::warn!(item_id, "GrantItem: no resources.items row for granted bandolier item; falling back to full bandolier resync");
                    }
                    super::super::vendor::helpers::sync_bandolier_after_inventory_change(
                        entity_id,
                        player_id,
                        db_pool,
                        cell_tx,
                        transport,
                        connected,
                        entity_to_addr,
                    )
                    .await;
                }
            }
        }
    }

    let visual: Option<String> = match sqlx::query_scalar(
        "SELECT visual_component FROM resources.items WHERE item_id = $1 AND visual_component IS NOT NULL",
    )
    .bind(item_id)
    .fetch_optional(pool.as_ref())
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(player_id, item_id, "GrantItem: visual_component lookup failed (skipping appearance refresh): {e}");
            None
        }
    };

    // Bandolier items (container_id 3) get their appearance refresh
    // from the cell side: `BaseToCellMsg::UpdateBandolierItem`'s
    // handler dispatches `RefreshAppearance` back to base after
    // flipping `weapon_holstered` correctly. Calling
    // `refresh_player_appearance` here too races the cell side and
    // can broadcast a stale "no weapon" appearance (cached state is
    // still `holstered=true` because the cell hasn't processed the
    // update yet) — that's why initial weapon equips appeared to
    // not show the weapon in playtest.
    //
    // Non-bandolier equipment (helmet, armor, accessories — slot 4
    // and up) doesn't go through the cell side, so we still need
    // this call for those.
    if visual.is_some() && container_id != 3 {
        tracing::info!(
            entity_id,
            player_id,
            item_id,
            container_id,
            "Equipped non-bandolier item has visual — resending BeingAppearance"
        );
        refresh_player_appearance(
            entity_id,
            player_id,
            db_pool,
            transport,
            connected,
            entity_to_addr,
        )
        .await;
    }
}
