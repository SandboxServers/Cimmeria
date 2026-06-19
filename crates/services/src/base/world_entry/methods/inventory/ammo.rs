//! SQL helpers for persisting per-bandolier-slot ammo state.

use sqlx::PgPool;

/// Persist the current ammo and selected ammo subtype for a bandolier slot.
///
/// The bandolier is `container_id = 3` (`INV_Bandolier`); the (character_id,
/// container_id, slot_id) triple is unique under
/// `sgw_inventory_unique_slot`, so this UPDATE touches at most one row.
///
/// `expected_instance_id` is added to the WHERE clause as a TOCTOU guard.
/// Between the cell sending `BandolierAmmoUpdate` and this UPDATE running, the
/// player could have swapped the slot's weapon. The predicate keys on the
/// `sgw_inventory.item_id` per-row instance id (a server-allocated surrogate,
/// unique per slot via `sgw_inventory_unique_slot`), so it fires even
/// when the swapped-in weapon shares the same *design* (`type_id`) as the one
/// the ammo writeback was computed for. Keying on `type_id` instead — as an
/// earlier version did — left a same-type-swap window: two physical instances
/// of the same weapon design passed the predicate and could scribble each
/// other's ammo (a dupe vector). A zero-row result here means the slot is
/// unequipped *or* the instance changed — in both cases dropping the write is
/// correct.
pub async fn update_bandolier_ammo(
    pool: &PgPool,
    character_id: i32,
    slot_id: i32,
    expected_instance_id: i32,
    current_ammo: i32,
    cur_ammo_type: i32,
) -> Result<(), sqlx::Error> {
    let res = sqlx::query(
        "UPDATE sgw_inventory \
         SET ammo = $1, cur_ammo_type = $2 \
         WHERE character_id = $3 AND container_id = 3 AND slot_id = $4 AND item_id = $5",
    )
    .bind(current_ammo)
    .bind(cur_ammo_type)
    .bind(character_id)
    .bind(slot_id)
    .bind(expected_instance_id)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        tracing::debug!(
            character_id,
            slot_id,
            expected_instance_id,
            current_ammo,
            cur_ammo_type,
            "update_bandolier_ammo: no rows updated (slot empty or item instance swapped)"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Live-DB integration tests for update_bandolier_ammo.
    //!
    //! Skip cleanly when DATABASE_URL is unset; against the bundled local
    //! Postgres they exercise the happy-path UPDATE, the TOCTOU guard
    //! fired by a stale instance PK (both a different-design swap and the
    //! same-design-different-instance swap this fix closes), and the
    //! empty-slot no-op path.

    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for bandolier-ammo tests, stepped past prior
    /// live-DB sentinels reserved elsewhere in the crate.
    const TEST_BASE: i32 = 0x7000_0B00;

    const INV_BANDOLIER: i32 = 3;

    /// Pick two distinct `resources.items.item_id` values that are
    /// allowed in the bandolier (`container_sets` is NULL/empty or
    /// includes container 3). Querying instead of hard-coding keeps
    /// the test from breaking if a seed-data renumber removes the
    /// specific ids we picked. The two ids returned are the lowest
    /// allowed ones, which is what the TOCTOU test needs: a primary
    /// type for the "matching" path and a distinct secondary type to
    /// simulate "player swapped the slot's weapon."
    async fn pick_two_bandolier_types(pool: &PgPool) -> (i32, i32) {
        let rows: Vec<i32> = sqlx::query_scalar(
            "SELECT item_id FROM resources.items \
             WHERE container_sets IS NULL OR 3 = ANY(container_sets) \
             ORDER BY item_id LIMIT 2",
        )
        .fetch_all(pool)
        .await
        .expect("pick two bandolier-allowed types");
        assert!(
            rows.len() >= 2,
            "seed must provide at least two bandolier-allowed item types; got {rows:?}",
        );
        (rows[0], rows[1])
    }

    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE character_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    async fn insert_account_and_player(pool: &PgPool, account_id: i32, player_id: i32) {
        sqlx::query(
            "INSERT INTO account (account_id, account_name, password) \
             VALUES ($1, $2, '')",
        )
        .bind(account_id)
        .bind(format!("ammo-test-{account_id}"))
        .execute(pool)
        .await
        .expect("insert account");

        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id, naquadah, bandolier_slot\
             ) VALUES ($1, $2, 1, 0, 1, 1, $3, '', 'CombatSim', 'BS_HumanMale.BS_HumanMale', \
                       0.0, 0.0, 0.0, 0, 0, 0)",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("test-{player_id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }

    /// Insert a sgw_inventory row at (INV_BANDOLIER, slot_id) with the
    /// given type_id. Returns the auto-generated item_id.
    async fn insert_bandolier_item(
        pool: &PgPool,
        player_id: i32,
        type_id: i32,
        slot_id: i32,
        ammo: i32,
        cur_ammo_type: i32,
    ) -> i32 {
        sqlx::query_scalar(
            "INSERT INTO sgw_inventory \
                (character_id, type_id, stack_size, slot_id, container_id, \
                 bound, durability, charges, ammo, cur_ammo_type) \
             VALUES ($1, $2, 1, $3, $4, false, 100, 0, $5, $6) \
             RETURNING item_id",
        )
        .bind(player_id)
        .bind(type_id)
        .bind(slot_id)
        .bind(INV_BANDOLIER)
        .bind(ammo)
        .bind(cur_ammo_type)
        .fetch_one(pool)
        .await
        .expect("insert bandolier row")
    }

    async fn ammo_state_of(pool: &PgPool, player_id: i32, slot_id: i32) -> Option<(i32, i32, i32)> {
        sqlx::query_as::<_, (i32, i32, i32)>(
            "SELECT type_id, ammo, cur_ammo_type FROM sgw_inventory \
             WHERE character_id = $1 AND container_id = $2 AND slot_id = $3",
        )
        .bind(player_id)
        .bind(INV_BANDOLIER)
        .bind(slot_id)
        .fetch_optional(pool)
        .await
        .expect("ammo_state query")
    }

    /// Happy path: the slot exists and the call passes its real instance
    /// PK (`sgw_inventory.item_id`), so the UPDATE writes the new ammo +
    /// cur_ammo_type and rows_affected==1.
    #[tokio::test]
    async fn update_writes_ammo_and_cur_ammo_type_when_type_matches() {
        let pool = require_db_or_skip!();
        let (primary_type, _) = pick_two_bandolier_types(&pool).await;
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        // Start with ammo=10, cur_ammo_type=0 so the post-call asserts
        // can pin the exact written values rather than just "non-zero".
        // Capture the RETURNING instance PK — that's the TOCTOU guard now.
        let instance_id = insert_bandolier_item(&pool, player_id, primary_type, 1, 10, 0).await;

        update_bandolier_ammo(&pool, player_id, 1, instance_id, 42, 7)
            .await
            .expect("update_bandolier_ammo must succeed on matching slot");

        assert_eq!(
            ammo_state_of(&pool, player_id, 1).await,
            Some((primary_type, 42, 7)),
            "ammo and cur_ammo_type must be written when expected_instance_id matches",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// TOCTOU guard, different-type swap: the slot exists but holds a row
    /// whose instance PK differs from the one the call expects (the player
    /// swapped the bandolier slot's weapon between the cell event and the
    /// persistence call, to a DIFFERENT design). The `AND item_id = $5`
    /// predicate must reject the write so the new weapon's ammo stays
    /// untouched.
    #[tokio::test]
    async fn update_no_op_when_slot_holds_different_type() {
        let pool = require_db_or_skip!();
        let (primary_type, swapped_type) = pick_two_bandolier_types(&pool).await;
        let account_id = TEST_BASE + 100;
        let player_id = TEST_BASE + 101;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        // Insert the ORIGINAL row, capture its instance PK, then delete it
        // and re-insert a DIFFERENT design at the same slot. The call passes
        // the now-stale original instance id, which no longer matches any
        // row — TOCTOU guard must fire.
        let stale_instance = insert_bandolier_item(&pool, player_id, primary_type, 2, 10, 0).await;
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE item_id = $1")
            .bind(stale_instance)
            .execute(&pool)
            .await;
        insert_bandolier_item(&pool, player_id, swapped_type, 2, 5, 1).await;

        update_bandolier_ammo(&pool, player_id, 2, stale_instance, 999, 99)
            .await
            .expect("must NOT error when instance_id mismatches; just no-op");

        assert_eq!(
            ammo_state_of(&pool, player_id, 2).await,
            Some((swapped_type, 5, 1)),
            "TOCTOU mismatch must leave ammo and cur_ammo_type untouched",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// TOCTOU guard, SAME-type swap (the issue this fix closes): the slot
    /// holds a row of the SAME weapon *design* as the one the writeback was
    /// computed for, but it is a DIFFERENT physical instance (the original
    /// was removed and a fresh copy of the same design re-equipped at the
    /// same slot between the cell event and the persist). Keying the guard
    /// on `type_id` would let the stale writeback scribble the new
    /// instance's ammo — a dupe vector. Keying on the `item_id` PK rejects
    /// it.
    ///
    /// Revert-verifier: changing the WHERE back to `type_id = $5` makes
    /// this test FAIL — both rows share design `T`, so the predicate
    /// matches the new instance and 999/99 gets written.
    #[tokio::test]
    async fn update_no_op_for_same_type_swap_different_instance() {
        let pool = require_db_or_skip!();
        // Only need ONE design — both physical instances share it.
        let (design_t, _) = pick_two_bandolier_types(&pool).await;
        let account_id = TEST_BASE + 300;
        let player_id = TEST_BASE + 301;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;

        // Instance A: original copy of design T at slot 1.
        let instance_a = insert_bandolier_item(&pool, player_id, design_t, 1, 10, 0).await;
        // Swap: delete A, re-insert the SAME design T at the same slot.
        // Postgres hands out a fresh item_id PK, so instance_b != instance_a.
        let _ = sqlx::query("DELETE FROM sgw_inventory WHERE item_id = $1")
            .bind(instance_a)
            .execute(&pool)
            .await;
        let instance_b = insert_bandolier_item(&pool, player_id, design_t, 1, 5, 1).await;
        assert_ne!(
            instance_a, instance_b,
            "same-design re-insert must get a distinct instance PK for this test to be meaningful",
        );

        // Persist using the STALE instance A id (computed before the swap).
        update_bandolier_ammo(&pool, player_id, 1, instance_a, 999, 99)
            .await
            .expect("must NOT error on a same-type-swap stale instance; just no-op");

        // Instance B (the live row) must be untouched.
        assert_eq!(
            ammo_state_of(&pool, player_id, 1).await,
            Some((design_t, 5, 1)),
            "same-type swap to a new instance must reject the stale writeback \
             (reverting the WHERE to type_id = $5 makes this fail — both rows \
             share design T so the predicate would match instance B)",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// Empty-slot path: no row at the (player, container, slot) triple,
    /// so the UPDATE matches no rows. Function returns Ok silently —
    /// callers don't distinguish "unequipped" from "swapped" (per the
    /// docstring on update_bandolier_ammo).
    #[tokio::test]
    async fn update_no_op_when_slot_is_empty() {
        let pool = require_db_or_skip!();
        let (primary_type, _) = pick_two_bandolier_types(&pool).await;
        let account_id = TEST_BASE + 200;
        let player_id = TEST_BASE + 201;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        // Deliberately do NOT insert a bandolier row at slot 3.

        update_bandolier_ammo(&pool, player_id, 3, primary_type, 42, 7)
            .await
            .expect("must NOT error on empty slot; just no-op");

        assert!(
            ammo_state_of(&pool, player_id, 3).await.is_none(),
            "empty slot must remain empty — function must not insert a row",
        );

        cleanup(&pool, account_id, player_id).await;
    }
}
