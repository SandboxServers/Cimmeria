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
