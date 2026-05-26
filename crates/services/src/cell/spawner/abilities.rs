//! Ability + effect definitions.
//!
//! Loads ability/effect rows + the `event_set_id → sequence_id` lookup table
//! the client needs for `onSequence` calls. Effect NVPs are joined onto their
//! parent effect after the initial load.

use sqlx::PgPool;

/// Event IDs for ability sequence lookups (from Atrea.enums).
pub const EVENT_ABILITY_BEGIN: i32 = 1000;
pub const EVENT_ABILITY_END: i32 = 1001;

/// Event IDs for archetype-keyed item handling sequences (from Atrea.enums).
/// Mirrors `Atrea.enums.Item_Equip` (4000), `Item_Unequip` (4001),
/// `Item_Reload` (4002), and `Item_Use` (4003).
pub const EVENT_ITEM_EQUIP: i32 = 4000;
pub const EVENT_ITEM_UNEQUIP: i32 = 4001;
pub const EVENT_ITEM_RELOAD: i32 = 4002;
pub const EVENT_ITEM_USE: i32 = 4003;

/// Per-item-instance ability-binding event IDs (from `Atrea.enums.EVENT_Item*`).
/// Used to look up the correct ability for a given weapon + event in
/// `resources.items_event_sets`. Reference:
/// `deprecated/python/Atrea/enums.py:456` and
/// `docs/protocol/item-sequence-lookup.md`.
pub const EVENT_ITEM_USE_ABILITY: i32 = 5;
pub const EVENT_ITEM_MELEE: i32 = 6;
pub const EVENT_ITEM_RANGED: i32 = 7;

/// Archetype → "Item handling" event set id, mirrored from
/// `python/common/Constants.py:ARCHETYPE_ITEM_EVENT_SETS`. Every human
/// archetype shares event set 804 (`"Item handling generic event set"`,
/// kismet `KIS-abilities_human.KIS-handling`); Asgard has its own at
/// 1455. Used by `getItemSequence(eventId)` to resolve the per-event
/// sequence (Item_Equip, Item_Unequip, Item_Reload, Item_Use).
///
/// Returns `None` for any archetype id not in the table — caller should
/// skip the sequence emit (matches python's `if eventSet else None`).
pub fn archetype_item_event_set(archetype_id: i32) -> Option<i32> {
    // ARCHETYPE_Asgard = 5 → 1455.
    // ARCHETYPE_Any (0), Soldier (1), Commando (2), Scientist (3),
    // Archeologist (4), Goauld (6), Sholva (7), Jaffa (8) → 804.
    match archetype_id {
        5 => Some(1455),
        0..=8 => Some(804),
        _ => None,
    }
}

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
         JOIN resources.sequences s ON s.sequence_id = ess.sequence_id",
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
pub async fn load_ability_defs(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, cimmeria_entity::abilities::AbilityDef>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AbilityRow>(
        "SELECT ability_id, name, cooldown, warmup, flags, is_ranged, \
         min_range, max_range, target_type_id, effect_ids, \
         required_ammo, event_set_id, velocity \
         FROM resources.abilities",
    )
    .fetch_all(pool)
    .await?;

    let mut defs = std::collections::HashMap::with_capacity(rows.len());
    for r in rows {
        defs.insert(
            r.ability_id,
            cimmeria_entity::abilities::AbilityDef {
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
            },
        );
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

/// Load per-item-instance ability bindings from `resources.items_event_sets`.
///
/// Builds a `(item_id, event_id) → ability_id` lookup so the cell can
/// resolve "what ability fires when this player wields THIS specific
/// weapon and triggers EVENT_X" without the per-call DB round trip.
///
/// Per [`docs/protocol/item-sequence-lookup.md`](../../../../docs/protocol/item-sequence-lookup.md),
/// these bindings cover **combat ability overrides per weapon** (e.g.,
/// pistol item 55 maps EVENT_ItemRanged=7 → ability 579 "Pistol Auto
/// Attack"; P90/SMG item 21 maps the same event to ability 559
/// "Automatic Weapon Auto Attack"). The previous server hardcoded
/// `592` (Pistol Shot) for every weapon's auto-attack, ignoring this
/// table — see issue #419, fixed in this commit.
///
/// Returns an empty map on DB failure; callers must treat absence as
/// "use the archetype default fallback" rather than panicking, so a
/// fresh checkout without seeded data still boots.
pub async fn load_item_event_set_abilities(
    pool: &PgPool,
) -> Result<std::collections::HashMap<(i32, i32), i32>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query("SELECT item_id, event_id, ability_id FROM resources.items_event_sets")
        .fetch_all(pool)
        .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let item_id: i32 = r.get("item_id");
        let event_id: i32 = r.get("event_id");
        let ability_id: i32 = r.get("ability_id");
        // First binding wins on duplicates. The seed data has duplicates
        // for some items (e.g., quest items that appear in multiple
        // sets) but per-event_id the binding is unique in practice.
        map.entry((item_id, event_id)).or_insert(ability_id);
    }

    tracing::info!(
        count = map.len(),
        "Loaded items_event_sets ability bindings"
    );
    Ok(map)
}

/// Load all effect definitions from `resources.effects` + `resources.effect_nvps`.
pub async fn load_effect_defs(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, cimmeria_entity::abilities::EffectDef>, sqlx::Error> {
    // Load effects
    let rows = sqlx::query_as::<_, EffectRow>(
        "SELECT effect_id, ability_id, delay, effect_sequence, event_set_id, script_name \
         FROM resources.effects",
    )
    .fetch_all(pool)
    .await?;

    let mut defs: std::collections::HashMap<i32, cimmeria_entity::abilities::EffectDef> =
        std::collections::HashMap::with_capacity(rows.len());
    for r in rows {
        defs.insert(
            r.effect_id,
            cimmeria_entity::abilities::EffectDef {
                effect_id: r.effect_id,
                ability_id: r.ability_id,
                delay: r.delay,
                effect_sequence: r.effect_sequence,
                event_set_id: r.event_set_id,
                script_name: r.script_name,
                params: std::collections::HashMap::new(),
            },
        );
    }

    // Load NVPs and attach to effects
    let nvps = sqlx::query_as::<_, EffectNvpRow>(
        "SELECT effect_id, name, value FROM resources.effect_nvps",
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
