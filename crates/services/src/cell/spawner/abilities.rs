//! Ability + effect definitions.
//!
//! Loads ability/effect rows + the `event_set_id → sequence_id` lookup table
//! the client needs for `onSequence` calls. Effect NVPs are joined onto their
//! parent effect after the initial load.

use sqlx::PgPool;

/// Event IDs for ability sequence lookups (from Atrea.enums).
pub const EVENT_ABILITY_BEGIN: i32 = 1000;
pub const EVENT_ABILITY_END: i32 = 1001;

/// Load the event set → sequence mapping from the database.
///
/// Joins `resources.event_sets_sequences` with `resources.sequences` to build
/// a lookup from `(event_set_id, event_id) → sequence_id`. This resolves the
/// correct KismetEventSetSeqID to send in `onSequence` calls.
///
/// The lookup chain: ability has event_set_id → event_sets_sequences join →
/// sequences table has (sequence_id, event_id). The client expects sequence_id,
/// NOT event_set_id.
pub async fn load_event_set_sequences(
    pool: &PgPool,
) -> Result<std::collections::HashMap<(i32, i32), i32>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT ess.event_set_id, s.sequence_id, s.event_id \
         FROM resources.event_sets_sequences ess \
         JOIN resources.sequences s ON s.sequence_id = ess.sequence_id"
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let event_set_id: i32 = r.get("event_set_id");
        let sequence_id: i32 = r.get("sequence_id");
        let event_id: i32 = r.get("event_id");
        map.insert((event_set_id, event_id), sequence_id);
    }

    tracing::info!(count = map.len(), "Loaded event_set sequence mappings");
    Ok(map)
}

/// Load all ability definitions from `resources.abilities`.
pub async fn load_ability_defs(pool: &PgPool) -> Result<std::collections::HashMap<i32, cimmeria_entity::abilities::AbilityDef>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AbilityRow>(
        "SELECT ability_id, name, cooldown, warmup, flags, is_ranged, \
         min_range, max_range, target_type_id, effect_ids, \
         required_ammo, event_set_id, velocity \
         FROM resources.abilities"
    )
    .fetch_all(pool)
    .await?;

    let mut defs = std::collections::HashMap::with_capacity(rows.len());
    for r in rows {
        defs.insert(r.ability_id, cimmeria_entity::abilities::AbilityDef {
            ability_id: r.ability_id,
            name: r.name,
            cooldown: r.cooldown,
            warmup: r.warmup,
            flags: r.flags as u32,
            is_ranged: r.is_ranged,
            min_range: r.min_range,
            max_range: r.max_range,
            target_type_id: r.target_type_id,
            effect_ids: r.effect_ids,
            moniker_ids: vec![],
            required_ammo: r.required_ammo,
            event_set_id: r.event_set_id,
            velocity: r.velocity,
        });
    }

    tracing::info!(count = defs.len(), "Loaded ability definitions");
    Ok(defs)
}

#[derive(sqlx::FromRow)]
struct AbilityRow {
    ability_id: i32,
    name: String,
    cooldown: f32,
    warmup: f32,
    flags: i32,
    is_ranged: bool,
    min_range: i32,
    max_range: i32,
    target_type_id: i32,
    effect_ids: Vec<i32>,
    required_ammo: i32,
    event_set_id: Option<i32>,
    velocity: f32,
}

/// Load all effect definitions from `resources.effects` + `resources.effect_nvps`.
pub async fn load_effect_defs(pool: &PgPool) -> Result<std::collections::HashMap<i32, cimmeria_entity::abilities::EffectDef>, sqlx::Error> {
    // Load effects
    let rows = sqlx::query_as::<_, EffectRow>(
        "SELECT effect_id, ability_id, delay, effect_sequence, event_set_id, script_name \
         FROM resources.effects"
    )
    .fetch_all(pool)
    .await?;

    let mut defs: std::collections::HashMap<i32, cimmeria_entity::abilities::EffectDef> =
        std::collections::HashMap::with_capacity(rows.len());
    for r in rows {
        defs.insert(r.effect_id, cimmeria_entity::abilities::EffectDef {
            effect_id: r.effect_id,
            ability_id: r.ability_id,
            delay: r.delay,
            effect_sequence: r.effect_sequence,
            event_set_id: r.event_set_id,
            script_name: r.script_name,
            params: std::collections::HashMap::new(),
        });
    }

    // Load NVPs and attach to effects
    let nvps = sqlx::query_as::<_, EffectNvpRow>(
        "SELECT effect_id, name, value FROM resources.effect_nvps"
    )
    .fetch_all(pool)
    .await?;

    for nvp in nvps {
        if let Some(effect) = defs.get_mut(&nvp.effect_id) {
            effect.params.insert(nvp.name, nvp.value);
        }
    }

    tracing::info!(count = defs.len(), "Loaded effect definitions");
    Ok(defs)
}

#[derive(sqlx::FromRow)]
struct EffectRow {
    effect_id: i32,
    ability_id: i32,
    delay: i32,
    effect_sequence: i32,
    event_set_id: Option<i32>,
    script_name: Option<String>,
}

#[derive(sqlx::FromRow)]
struct EffectNvpRow {
    effect_id: i32,
    name: String,
    value: String,
}
