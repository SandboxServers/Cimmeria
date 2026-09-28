//! Weapon reach from `resources.items`, for abilities flagged
//! `UseWeaponRange` (#1017).

use std::collections::HashMap;

use cimmeria_entity::abilities::WeaponRanges;
use sqlx::PgPool;

/// Load the reach of every item with a non-zero ranged or melee range,
/// keyed by item design id (`BandolierItem::item_id`).
///
/// The four columns are integer **metres** (30 / 35 / 40, melee 2 / 3),
/// the same numbers as the cooked items' `RangeRanges` / `MeleeRanges`, so
/// they are not converted the way ability ranges are (UE3 units, #919).
/// Every item row is considered, not only the `clip_size > 0` ones
/// `load_item_defs` keeps: the seeded Jaffa staffs have clip 0 and a
/// 3-30 m reach.
///
/// Returns `Err` on DB failure; the startup wiring logs it and leaves the
/// map empty, and a flagged ability then falls back to its own range.
pub async fn load_weapon_ranges(pool: &PgPool) -> Result<HashMap<i32, WeaponRanges>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT item_id, \
                COALESCE(min_ranged_range, 0) AS min_ranged_range, \
                COALESCE(max_ranged_range, 0) AS max_ranged_range, \
                COALESCE(min_melee_range, 0) AS min_melee_range, \
                COALESCE(max_melee_range, 0) AS max_melee_range \
         FROM resources.items \
         WHERE max_ranged_range > 0 OR max_melee_range > 0",
    )
    .fetch_all(pool)
    .await?;

    let mut map = HashMap::with_capacity(rows.len());
    for r in &rows {
        let item_id: i32 = r.get("item_id");
        let metres = |col: &str| r.get::<i32, _>(col) as f32;
        map.insert(
            item_id,
            WeaponRanges {
                min_ranged: metres("min_ranged_range"),
                max_ranged: metres("max_ranged_range"),
                min_melee: metres("min_melee_range"),
                max_melee: metres("max_melee_range"),
            },
        );
    }

    tracing::info!(count = map.len(), "Loaded weapon ranges");
    Ok(map)
}
