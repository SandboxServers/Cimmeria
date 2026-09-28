//! The grant transaction: advisory lock → BIND_ON_ACQUIRE check →
//! stack-merge fast path → fresh-slot reserve/INSERT → bandolier reconcile
//! → outbox enqueue → commit.
//!
//! It returns what committed, or why nothing did. A [`PersistOutcome::Refused`]
//! is only ever built before the commit, so a caller may hand the item back
//! (a loot pickup returns it to the corpse) without risking a duplicate.

use cimmeria_cell_catalog::crafting::ItemFlags;
use sqlx::PgPool;

use super::super::super::vendor::serializers::reserve_free_inventory_slots;
use crate::base::outbox::{self, CellOutboxPayload};
use crate::cell::messages::GrantRefusal;

/// What one committed grant wrote.
#[derive(Debug)]
pub(super) struct Committed {
    pub container_id: i32,
    pub slot_id: i32,
    /// The new row's instance id; `None` when the grant merged into a stack.
    pub instance_id: Option<i32>,
    /// The stack size in that slot before and after the grant (a fresh slot
    /// starts at 0), so a merge is distinguishable from a new slot.
    pub qty_before: i32,
    pub qty_after: i32,
    /// Whether a bandolier grant became the active slot.
    pub bandolier_became_active: bool,
    pub outbox_id: i64,
    pub outbox_payload: CellOutboxPayload,
}

/// How the grant transaction ended.
#[derive(Debug)]
pub(super) enum PersistOutcome {
    Committed(Committed),
    /// Nothing committed: the transaction rolled back or never started.
    Refused(GrantRefusal),
    /// `COMMIT` failed without a server answer (the connection dropped), so
    /// the write may or may not have landed. Never hand the item back.
    CommitUnknown,
}

/// Persist a grant of `count` × `item_id` into `container_id`.
pub(super) async fn persist_grant(
    pool: &PgPool,
    entity_id: u32,
    player_id: i32,
    item_id: i32,
    container_id: i32,
    count: i32,
) -> PersistOutcome {
    let mut db_tx = match pool.begin().await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!(player_id, item_id, "GrantItem: begin tx failed: {e}");
            return db_refused();
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
        return db_refused();
    }

    // Whether this design is BIND_ON_ACQUIRE (`resources.items.flags & 4`,
    // SS-914). A bound grant must never land in — or become — a stack
    // another (unbound) acquisition could still reach, so it skips the
    // merge fast path below entirely and is always inserted as its own
    // row with `bound = true`. An unresolvable `item_id` defaults to
    // `false` here; the INSERT…SELECT further down re-reads
    // `resources.items` with the same `WHERE item_id = $item_id` and
    // finds nothing either, so the grant is refused there regardless of
    // this default.
    let is_bound: bool =
        match sqlx::query_scalar::<_, i32>("SELECT flags FROM resources.items WHERE item_id = $1")
            .bind(item_id)
            .fetch_optional(&mut *db_tx)
            .await
        {
            Ok(Some(flags)) => flags & (ItemFlags::BIND_ON_ACQUIRE as i32) != 0,
            Ok(None) => false,
            Err(e) => {
                let _ = db_tx.rollback().await;
                tracing::error!(
                    player_id,
                    item_id,
                    "GrantItem: item flags lookup failed: {e}"
                );
                return db_refused();
            }
        };

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
    //     A *bound* incoming grant never runs this query at all
    //     (`is_bound` above short-circuits it to `None`): merging a
    //     bound quantity into an unbound target row would silently
    //     launder it back to unbound, and merging it into another
    //     bound row would conflate two distinct soul-bound grants
    //     into one stack. Consistent rule: bound items always take a
    //     fresh row.
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
        stack_size: i32,
    }
    let merge_candidate: Option<MergeCandidate> = if is_bound {
        None
    } else {
        match sqlx::query_as(
            "SELECT inv.item_id, inv.slot_id, inv.stack_size \
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
                return db_refused();
            }
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
            return db_refused();
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
        // merge path. Skip straight to outbox enqueue + commit.
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
                return db_refused();
            }
        };
        if let Err(e) = db_tx.commit().await {
            tracing::error!(player_id, item_id, "GrantItem (merge): commit failed: {e}");
            return commit_failed(&e);
        }
        return PersistOutcome::Committed(Committed {
            container_id,
            slot_id: target.slot_id,
            instance_id: None,
            qty_before: target.stack_size,
            qty_after: target.stack_size + count,
            bandolier_became_active: false,
            outbox_id,
            outbox_payload,
        });
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
                    return PersistOutcome::Refused(GrantRefusal::ContainerFull);
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
                return PersistOutcome::Refused(GrantRefusal::ContainerFull);
            }
            Err(e) => {
                let _ = db_tx.rollback().await;
                tracing::error!(
                    player_id,
                    item_id,
                    container_id,
                    "GrantItem: slot reserve failed: {e}"
                );
                return db_refused();
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
            return db_refused();
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
    // per-row instance id, which the bandolier fast-path needs as the
    // ammo-persist TOCTOU guard. The design id is the `item_id` param; the
    // instance id is unique per physical row and is what distinguishes two copies
    // of the same weapon design occupying the bandolier over time.
    let result = sqlx::query_scalar::<_, i32>(
        "INSERT INTO sgw_inventory \
            (character_id, type_id, stack_size, slot_id, container_id, \
             bound, durability, charges, \
             ammo_type, ammo_types, ammo, flags) \
         SELECT $1, ri.item_id, $2, $3, $4, $7, 100, $5, \
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
    .bind(is_bound)
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
            return db_refused();
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
                return db_refused();
            }
        }
    }

    // Enqueue the cell-notification BEFORE commit so the outbox row and the
    // inventory mutation become visible atomically. After commit, the caller
    // tries the in-process dispatch; if the cell receiver is gone, the row
    // stays undelivered for the background drainer to retry.
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
            return db_refused();
        }
    };

    if let Err(e) = db_tx.commit().await {
        tracing::error!(player_id, item_id, "GrantItem: commit failed: {e}");
        return commit_failed(&e);
    }

    PersistOutcome::Committed(Committed {
        container_id,
        slot_id: next_slot,
        instance_id: Some(instance_id),
        qty_before: 0,
        qty_after: count,
        bandolier_became_active,
        outbox_id,
        outbox_payload,
    })
}

/// A database error before the commit: the transaction rolled back.
fn db_refused() -> PersistOutcome {
    PersistOutcome::Refused(GrantRefusal::DatabaseError)
}

/// A failed `COMMIT`. A server error answer (`sqlx::Error::Database`) means
/// Postgres rolled the transaction back, so nothing landed. Any other error
/// (I/O, protocol, a closed pool) leaves the outcome unknown.
fn commit_failed(e: &sqlx::Error) -> PersistOutcome {
    if matches!(e, sqlx::Error::Database(_)) {
        PersistOutcome::Refused(GrantRefusal::DatabaseError)
    } else {
        PersistOutcome::CommitUnknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_commit_without_a_server_answer_is_unknown() {
        assert!(matches!(
            commit_failed(&sqlx::Error::PoolClosed),
            PersistOutcome::CommitUnknown
        ));
        assert!(matches!(
            commit_failed(&sqlx::Error::Io(std::io::Error::other("reset"))),
            PersistOutcome::CommitUnknown
        ));
    }
}
