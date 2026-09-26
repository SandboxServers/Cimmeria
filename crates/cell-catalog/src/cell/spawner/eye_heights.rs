//! Per-body-set eye heights (NA31), from `resources.body_sets.eye_height`.
//!
//! Line of sight is cast eye to eye. The values were measured from each body
//! set's reference skeletal mesh in the cooked client package; see
//! `docs/reverse-engineering/findings/being-eye-heights.md`. A body set with
//! no value (props, terminals, a set with no reference mesh) is left out, and
//! the entity takes `space_manager::DEFAULT_EYE_HEIGHT`.

use std::collections::HashMap;

use sqlx::PgPool;

/// Load `body_set -> eye_height` for every row with a value.
pub async fn load_body_set_eye_heights(pool: &PgPool) -> Result<HashMap<String, f32>, sqlx::Error> {
    let rows: Vec<(String, f32)> = sqlx::query_as(
        "SELECT body_set, eye_height FROM resources.body_sets WHERE eye_height IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;
    tracing::info!(
        target: "spawner",
        event = "body_set_eye_heights_loaded",
        count = rows.len(),
        "Loaded per-body-set eye heights for line of sight"
    );
    Ok(rows.into_iter().collect())
}
