//! SQL helpers for persisting per-bandolier-slot ammo state.

use sqlx::PgPool;

/// Persist the current ammo and selected ammo subtype for a bandolier slot.
///
/// The bandolier is `container_id = 3` (`INV_Bandolier`); the (character_id,
/// container_id, slot_id) triple is unique under
/// `sgw_inventory_unique_slot`, so this UPDATE touches at most one row.
///
/// `expected_item_id` is added to the WHERE clause as a TOCTOU guard. Between
/// the cell sending `BandolierAmmoUpdate` and this UPDATE running, the player
/// could have swapped the slot's weapon. Without the `type_id` predicate the
/// UPDATE would silently scribble the old weapon's ammo onto the new one.
/// A zero-row result here means either the slot is unequipped *or* the item
/// changed — in both cases dropping the write is correct.
pub async fn update_bandolier_ammo(
    pool: &PgPool,
    character_id: i32,
    slot_id: i32,
    expected_item_id: i32,
    current_ammo: i32,
    cur_ammo_type: i32,
) -> Result<(), sqlx::Error> {
    let res = sqlx::query(
        "UPDATE sgw_inventory \
         SET ammo = $1, cur_ammo_type = $2 \
         WHERE character_id = $3 AND container_id = 3 AND slot_id = $4 AND type_id = $5",
    )
    .bind(current_ammo)
    .bind(cur_ammo_type)
    .bind(character_id)
    .bind(slot_id)
    .bind(expected_item_id)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        tracing::debug!(
            character_id,
            slot_id,
            expected_item_id,
            current_ammo,
            cur_ammo_type,
            "update_bandolier_ammo: no rows updated (slot empty or item swapped)"
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
    //! fired by an unexpected `type_id`, and the empty-slot no-op path.

    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel base for bandolier-ammo tests. Distinct from prior live-DB
    /// sentinels (outbox 0x000 / grant_cash +0x100 / move +0x200 /
    /// grant_item +0x300 / missions +0x400 / mail +0x500 /
    /// vendor/repair +0x600 / paid_repair +0x700 / sell +0x800 /
    /// buyback +0x900 / purchase +0x0A00).
    const TEST_BASE: i32 = 0x7000_0B00;

    /// Both designs are bandolier-allowed (verified via
    /// `SELECT item_id FROM resources.items WHERE container_sets IS NULL
    /// OR 3 = ANY(container_sets)`). Picking a SECOND distinct allowed
    /// type matters: it lets the TOCTOU test simulate "player swapped
    /// the bandolier slot's weapon" with a real foreign-key-valid type.
    const FIRST_BANDOLIER_TYPE: i32 = 17;
    const SWAPPED_BANDOLIER_TYPE: i32 = 21;

    const INV_BANDOLIER: i32 = 3;

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

    /// Happy path: the slot exists with the expected `type_id`, so the
    /// UPDATE writes the new ammo + cur_ammo_type and rows_affected==1.
    #[tokio::test]
    async fn update_writes_ammo_and_cur_ammo_type_when_type_matches() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE;
        let player_id = TEST_BASE + 1;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        // Start with ammo=10, cur_ammo_type=0 so the post-call asserts
        // can pin the exact written values rather than just "non-zero".
        insert_bandolier_item(&pool, player_id, FIRST_BANDOLIER_TYPE, 1, 10, 0).await;

        update_bandolier_ammo(&pool, player_id, 1, FIRST_BANDOLIER_TYPE, 42, 7)
            .await
            .expect("update_bandolier_ammo must succeed on matching slot");

        assert_eq!(
            ammo_state_of(&pool, player_id, 1).await,
            Some((FIRST_BANDOLIER_TYPE, 42, 7)),
            "ammo and cur_ammo_type must be written when expected_item_id matches",
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// TOCTOU guard: the slot exists but holds a DIFFERENT type_id (the
    /// player swapped the bandolier slot's weapon between the cell event
    /// and the persistence call). The `AND type_id = $5` predicate must
    /// reject the write so the new weapon's ammo stays untouched. The
    /// pre-fix bug shape this protects against: scribbling the old
    /// weapon's ammo onto the new weapon.
    #[tokio::test]
    async fn update_no_op_when_slot_holds_different_type() {
        let pool = require_db_or_skip!();
        let account_id = TEST_BASE + 100;
        let player_id = TEST_BASE + 101;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        // Slot has SWAPPED_BANDOLIER_TYPE, but the call passes
        // FIRST_BANDOLIER_TYPE as expected — TOCTOU guard must fire.
        insert_bandolier_item(&pool, player_id, SWAPPED_BANDOLIER_TYPE, 2, 5, 1).await;

        update_bandolier_ammo(&pool, player_id, 2, FIRST_BANDOLIER_TYPE, 999, 99)
            .await
            .expect("must NOT error when type_id mismatches; just no-op");

        assert_eq!(
            ammo_state_of(&pool, player_id, 2).await,
            Some((SWAPPED_BANDOLIER_TYPE, 5, 1)),
            "TOCTOU mismatch must leave ammo and cur_ammo_type untouched",
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
        let account_id = TEST_BASE + 200;
        let player_id = TEST_BASE + 201;
        cleanup(&pool, account_id, player_id).await;
        insert_account_and_player(&pool, account_id, player_id).await;
        // Deliberately do NOT insert a bandolier row at slot 3.

        update_bandolier_ammo(&pool, player_id, 3, FIRST_BANDOLIER_TYPE, 42, 7)
            .await
            .expect("must NOT error on empty slot; just no-op");

        assert!(
            ammo_state_of(&pool, player_id, 3).await.is_none(),
            "empty slot must remain empty — function must not insert a row",
        );

        cleanup(&pool, account_id, player_id).await;
    }
}
