//! NPC ability sets: `resources.ability_set_abilities`, keyed by set id.
//!
//! A template's `ability_set_id` is already folded into each
//! [`super::SpawnRecord`]'s `ability_ids` at load. This map keeps the sets
//! themselves, so `gmSetMobAbilitySet` (SGWGmPlayer 158, AB-N2) can swap a
//! live mob onto any set, including one no template uses. A startup
//! snapshot like the other caches: a seed edit needs a restart.

use std::collections::HashMap;

use sqlx::PgPool;

/// Load every ability set, each set's ids ascending.
pub async fn load_ability_sets(pool: &PgPool) -> Result<HashMap<i32, Vec<i32>>, sqlx::Error> {
    let rows: Vec<(i32, Vec<i32>)> = sqlx::query_as(
        "SELECT ability_set_id, array_agg(ability_id ORDER BY ability_id) \
           FROM resources.ability_set_abilities \
          GROUP BY ability_set_id",
    )
    .fetch_all(pool)
    .await?;
    let sets: HashMap<i32, Vec<i32>> = rows.into_iter().collect();
    tracing::info!(count = sets.len(), "Loaded NPC ability sets");
    Ok(sets)
}
