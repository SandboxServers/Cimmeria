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

/// One ability entry in an archetype's training tree.
///
/// Mirrors `resources.archetype_ability_tree` row. Used to validate that a
/// player can train a given ability:
/// - `level` is the player level required to train this ability
/// - `prerequisite_abilities` must all be already-known by the player
/// - `tree_index` is 0/1/2 (each archetype has three skill trees)
#[derive(Debug, Clone)]
pub struct ArchetypeAbilityTreeEntry {
    pub ability_id: i32,
    pub tree_index: i32,
    pub level: i32,
    pub prerequisite_abilities: Vec<i32>,
}

/// Load `resources.archetype_ability_tree` into a per-archetype lookup.
///
/// Returns `HashMap<archetype_id, Vec<entries>>`. Used cell-side for
/// `train_ability` validation: check the requested ability is in the
/// player's archetype tree, the player level meets the entry's `level`
/// requirement, and every `prerequisite_abilities` entry is in the
/// player's known set.
///
/// Today the table has 169 rows total: Soldier (84) and Commando (85)
/// each have full trees; the other 7 archetypes are empty pending
/// Phase 7 content authoring. Validation will reject train attempts
/// for those archetypes — they can still get baseline abilities via
/// `char_creation_abilities` (Phase 2) but can't deepen the tree.
///
/// Archetype id encoding matches the `EArchetype` enum position
/// (0=Any, 1=Soldier, 2=Commando, 3=Scientist, 4=Archeologist, 5=Asgard,
/// 6=Goauld, 7=Sholva, 8=Jaffa) — confirmed by the existing
/// `archetype_count_matches_earchetype_enum_cardinality` test.
pub async fn load_archetype_ability_trees(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, Vec<ArchetypeAbilityTreeEntry>>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT \
             array_position(enum_range(NULL::resources.\"EArchetype\"), archetype) - 1 \
                 AS archetype_id, \
             ability_id, tree_index, level, prerequisite_abilities \
         FROM resources.archetype_ability_tree \
         ORDER BY archetype, tree_index, ability_index",
    )
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<i32, Vec<ArchetypeAbilityTreeEntry>> =
        std::collections::HashMap::new();
    for r in &rows {
        let archetype_id: i32 = r.get("archetype_id");
        let ability_id: i32 = r.get("ability_id");
        let tree_index: i32 = r.get("tree_index");
        let level: i32 = r.get("level");
        let prerequisite_abilities: Vec<i32> = r.get("prerequisite_abilities");
        map.entry(archetype_id)
            .or_default()
            .push(ArchetypeAbilityTreeEntry {
                ability_id,
                tree_index,
                level,
                prerequisite_abilities,
            });
    }

    tracing::info!(
        archetypes = map.len(),
        total_entries = rows.len(),
        "Loaded archetype ability trees"
    );
    Ok(map)
}

/// Load `resources.trainer_abilities` keyed by `(list_id, archetype_id)`.
///
/// Returns `HashMap<(list_id, archetype_id), Vec<ability_id>>`. Used by the
/// trainer NPC interaction (`sendAbilityList`) to enumerate which abilities a
/// specific trainer NPC offers to a player of a given archetype.
///
/// Today there's exactly one populated list (`list_id=1`, "Debug ability
/// list") referenced by template_id=25 ("Interaction Debug NPC"). The other
/// 152 templates have NULL `trainer_ability_list_id` so they're not trainers.
/// More trainers + lists are Phase 7 content work.
pub async fn load_trainer_abilities(
    pool: &PgPool,
) -> Result<std::collections::HashMap<(i32, i32), Vec<i32>>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT \
             list_id, \
             array_position(enum_range(NULL::resources.\"EArchetype\"), archetype) - 1 \
                 AS archetype_id, \
             ability_id \
         FROM resources.trainer_abilities",
    )
    .fetch_all(pool)
    .await?;

    let mut map: std::collections::HashMap<(i32, i32), Vec<i32>> = std::collections::HashMap::new();
    for r in &rows {
        let list_id: i32 = r.get("list_id");
        let archetype_id: i32 = r.get("archetype_id");
        let ability_id: i32 = r.get("ability_id");
        map.entry((list_id, archetype_id))
            .or_default()
            .push(ability_id);
    }

    tracing::info!(
        rows = rows.len(),
        unique_lists = map
            .keys()
            .map(|k| k.0)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        "Loaded trainer ability lists"
    );
    Ok(map)
}

/// Load entity_templates `trainer_ability_list_id` for templates that ARE
/// trainers. Returns `HashMap<template_id, list_id>`.
///
/// Most templates have NULL trainer_ability_list_id — the query filters
/// those out so the runtime check is a fast HashMap miss for non-trainer
/// NPCs (the overwhelming majority).
pub async fn load_template_trainer_lists(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, i32>, sqlx::Error> {
    use sqlx::Row;

    let rows = sqlx::query(
        "SELECT template_id, trainer_ability_list_id \
         FROM resources.entity_templates \
         WHERE trainer_ability_list_id IS NOT NULL",
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let template_id: i32 = r.get("template_id");
        let list_id: i32 = r.get("trainer_ability_list_id");
        map.insert(template_id, list_id);
    }

    tracing::info!(
        count = map.len(),
        "Loaded trainer-template → ability-list mappings"
    );
    Ok(map)
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
/// table — see , fixed in this commit.
///
/// Returns `Err` on DB failure; the startup wiring in
/// `cell/service/startup.rs` logs the error and leaves
/// `space_mgr.item_event_set_abilities` as the empty default map, so a
/// fresh checkout without seeded data still boots. Callers must treat
/// a missing `(item_id, event_id)` lookup as "use the caller-supplied
/// default" rather than panicking.
pub async fn load_item_event_set_abilities(
    pool: &PgPool,
) -> Result<std::collections::HashMap<(i32, i32), i32>, sqlx::Error> {
    use sqlx::Row;

    // ORDER BY item_id, event_id, ability_id pins the duplicate
    // resolution: without it Postgres returns rows in physical (heap)
    // order, which can shift between runs after VACUUM/INSERT. Pairing
    // with `or_insert` below gives the lowest-id ability deterministic
    // priority when duplicate (item_id, event_id) rows exist in seed.
    let rows = sqlx::query(
        "SELECT item_id, event_id, ability_id FROM resources.items_event_sets \
         ORDER BY item_id, event_id, ability_id",
    )
    .fetch_all(pool)
    .await?;

    let mut map = std::collections::HashMap::with_capacity(rows.len());
    for r in &rows {
        let item_id: i32 = r.get("item_id");
        let event_id: i32 = r.get("event_id");
        let ability_id: i32 = r.get("ability_id");
        // Lowest-ability-id binding wins on duplicates. The seed data
        // has duplicates for some items (e.g., quest items that appear
        // in multiple sets) but per-event_id the binding is unique in
        // practice. ORDER BY ability_id above pins determinism so the
        // same DB content always resolves to the same ability.
        map.entry((item_id, event_id)).or_insert(ability_id);
    }

    tracing::info!(
        count = map.len(),
        "Loaded items_event_sets ability bindings"
    );
    Ok(map)
}

/// Load all effect definitions from `resources.effects` + `resources.effect_nvps`.
///
/// The columns beyond `script_name` (pulse_count, pulse_duration,
/// is_channeled, target_collection_method, tcm_param1/2, flags) drive
/// the cone-AoE collection and the active-effect pulsing loop — see
/// `cell::effects` for the dispatcher that reads them.
pub async fn load_effect_defs(
    pool: &PgPool,
) -> Result<std::collections::HashMap<i32, cimmeria_entity::abilities::EffectDef>, sqlx::Error> {
    // `target_collection_method` is a PG ENUM (`resources."ETargetCollectionMethod"`),
    // not TEXT. sqlx-postgres won't auto-coerce a PG ENUM into
    // `Option<String>` (the `EffectRow` field type), and without the cast
    // the whole `fetch_all` returns a decode error — the WARN at startup
    // hides the fact that `effect_defs` ends up EMPTY for the entire
    // process lifetime. Empty effect_defs silently disables every combat
    // ability that resolves through an effect. Cast inline so the existing
    // `Option<String>` shape (with `unwrap_or_else(TCM_SINGLE)` downstream)
    // keeps working. Long-term fix is a `#[sqlx(type_name = ...)]` Rust
    // enum mirror; the cast is the safe immediate patch.
    let rows = sqlx::query_as::<_, EffectRow>(
        "SELECT effect_id, ability_id, delay, effect_sequence, event_set_id, script_name, \
                pulse_count, pulse_duration, is_channeled, \
                target_collection_method::TEXT AS target_collection_method, \
                tcm_param1, tcm_param2, flags \
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
                pulse_count: r.pulse_count,
                pulse_duration: r.pulse_duration,
                is_channeled: r.is_channeled,
                // Default to single-target when the DB cell is empty.
                target_collection_method: r
                    .target_collection_method
                    .unwrap_or_else(|| cimmeria_entity::abilities::TCM_SINGLE.to_string()),
                tcm_param1: r.tcm_param1.unwrap_or_default(),
                tcm_param2: r.tcm_param2.unwrap_or_default(),
                // `flags` is `integer` in the schema; cast through i32 → u32.
                flags: r.flags as u32,
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
    pulse_count: i32,
    pulse_duration: f32,
    is_channeled: bool,
    target_collection_method: Option<String>,
    tcm_param1: Option<String>,
    tcm_param2: Option<String>,
    flags: i32,
}

#[derive(sqlx::FromRow)]
struct EffectNvpRow {
    effect_id: i32,
    name: String,
    value: String,
}
