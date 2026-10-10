//! Item monikers from `resources.items`, for the player weapon requirement
//! (Class Start v6 OD-CS11).

use std::collections::HashMap;

use sqlx::PgPool;

/// Load the `moniker_ids` of every item that has any, keyed by item design
/// id (`BandolierItem::item_id`).
///
/// The use_ability launch reads the active bandolier item's entry and
/// refuses a player's cast of an ability whose `item_monikers` share none
/// of them (`CONDITION_FEEDBACK_WrongWeaponType`). Every item row is
/// considered, not only weapons: the check is "does the active item carry
/// the moniker", whatever the item is.
///
/// Returns `Err` on DB failure; the startup wiring logs it and leaves the
/// map empty, and every weapon-requiring ability is then refused for
/// players (the rule is not weakened when the data is missing).
pub async fn load_weapon_monikers(pool: &PgPool) -> Result<HashMap<i32, Vec<i64>>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT item_id, moniker_ids \
         FROM resources.items \
         WHERE cardinality(moniker_ids) > 0",
    )
    .fetch_all(pool)
    .await?;

    let mut map = HashMap::with_capacity(rows.len());
    for r in &rows {
        let item_id: i32 = r.get("item_id");
        let monikers: Vec<i64> = r.get("moniker_ids");
        map.insert(item_id, monikers);
    }

    tracing::info!(count = map.len(), "Loaded item monikers");
    Ok(map)
}
