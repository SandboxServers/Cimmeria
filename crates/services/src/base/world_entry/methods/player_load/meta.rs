use std::sync::Arc;

use sqlx::PgPool;

use cimmeria_entity::abilities::AbilityTreeData;

use crate::mercury::{archetype_ability_tree, PlayerLoadData};

fn archetype_resource_name(archetype_id: i32) -> Option<&'static str> {
    match archetype_id {
        0 => Some("ARCHETYPE_Any"),
        1 => Some("ARCHETYPE_Soldier"),
        2 => Some("ARCHETYPE_Commando"),
        3 => Some("ARCHETYPE_Scientist"),
        4 => Some("ARCHETYPE_Archeologist"),
        5 => Some("ARCHETYPE_Asgard"),
        6 => Some("ARCHETYPE_Goauld"),
        7 => Some("ARCHETYPE_Sholva"),
        8 => Some("ARCHETYPE_Jaffa"),
        _ => None,
    }
}

/// Default player load data when the DB is unavailable.
///
/// Caveat: `archetype` and `ability_tree` are both keyed to archetype id 1
/// here. If a caller ever partially overrides this struct (e.g., a half-loaded
/// row that fills `archetype` from DB but leaves the rest defaulted), the
/// tree won't match the new archetype. This is the "DB unavailable" sentinel
/// used as a whole, so today this is fine — but keep the two values in sync.
pub fn default_player_load_data() -> PlayerLoadData {
    PlayerLoadData {
        player_id: 0,
        level: 1,
        player_name: "Unknown".into(),
        extra_name: String::new(),
        alignment: 1,
        archetype: 1,
        gender: 1,
        bodyset: "BS_HumanMale.BS_HumanMale".into(),
        components: vec![],
        exp: 0,
        naquadah: 0,
        known_stargates: vec![],
        abilities: vec![],
        training_points: 0,
        applied_science_points: 0,
        blueprint_ids: vec![],
        first_login: 1,
        access_level: 0,
        skin_color_id: 0,
        active_bandolier_slot: 0,
        bandolier_items: vec![],
        ability_tree: archetype_ability_tree(1),
        items: vec![],
    }
}

#[derive(sqlx::FromRow)]
struct BandolierItemRow {
    slot_id: i32,
    item_id: i32,
    clip_size: i32,
    default_ammo_type_id: i32,
    /// Per-slot remaining ammo from `sgw_inventory.ammo`. Populated by Stage A
    /// but not yet read by fire/reload — Stage C wires the consumers.
    current_ammo: i32,
    /// Per-slot selected ammo subtype from `sgw_inventory.cur_ammo_type`. Zero
    /// means "no explicit choice" — Rust-side defaulting falls back to the
    /// item's default_ammo_type below.
    cur_ammo_type: i32,
}

const BANDOLIER_ITEMS_QUERY: &str = r#"
SELECT inv.slot_id, inv.type_id AS item_id, COALESCE(ri.clip_size, 0) AS clip_size,
       CASE WHEN ri.default_ammo_type IS NULL THEN 0
            ELSE array_position(enum_range(NULL::resources."EAmmoType"), ri.default_ammo_type) - 1
       END AS default_ammo_type_id,
       inv.ammo AS current_ammo,
       inv.cur_ammo_type
FROM sgw_inventory inv
JOIN resources.items ri ON ri.item_id = inv.type_id
WHERE inv.character_id = $1 AND inv.container_id = 3
ORDER BY inv.slot_id
"#;

fn map_bandolier_rows(
    rows: Vec<BandolierItemRow>,
) -> Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)> {
    rows.into_iter()
        .map(|row| {
            // Treat 0 as "no explicit choice" and fall back to the item's
            // default ammo type — matches the legacy Account.py behavior where
            // a slot with no override picks default_ammo_type at load time.
            let cur_ammo_type = if row.cur_ammo_type == 0 {
                row.default_ammo_type_id
            } else {
                row.cur_ammo_type
            };
            (
                row.slot_id,
                cimmeria_entity::cell_entity::BandolierItem {
                    item_id: row.item_id,
                    clip_size: row.clip_size,
                    default_ammo_type: row.default_ammo_type_id,
                    current_ammo: row.current_ammo,
                    cur_ammo_type,
                },
            )
        })
        .collect()
}

/// Query bandolier items from the database for a player.
///
/// Returns tuples of (slot_id, BandolierItem) containing equipped weapon info.
pub async fn query_bandolier_items(
    db_pool: &Option<Arc<PgPool>>,
    player_id: i32,
) -> Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)> {
    let pool = match db_pool {
        Some(p) => p,
        None => return vec![],
    };

    match sqlx::query_as::<_, BandolierItemRow>(BANDOLIER_ITEMS_QUERY)
        .bind(player_id)
        .fetch_all(pool.as_ref())
        .await
    {
        Ok(rows) => map_bandolier_rows(rows),
        Err(e) => {
            tracing::error!(player_id, "query_bandolier_items failed: {e}");
            vec![]
        }
    }
}

/// Transaction-aware variant — runs the bandolier read inside an existing tx
/// so callers that have already acquired locks (e.g. `FOR UPDATE` on
/// `sgw_player`) see a consistent snapshot.
pub async fn query_bandolier_items_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    player_id: i32,
) -> Result<Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, BandolierItemRow>(BANDOLIER_ITEMS_QUERY)
        .bind(player_id)
        .fetch_all(&mut **tx)
        .await?;
    Ok(map_bandolier_rows(rows))
}

/// Query archetype ability tree data from the database.
pub async fn query_archetype_ability_tree(
    pool: &PgPool,
    archetype_id: i32,
) -> Option<AbilityTreeData> {
    let archetype = archetype_resource_name(archetype_id)?;

    #[derive(sqlx::FromRow)]
    struct AbilityTreeRow {
        tree_index: i32,
        ability_id: i32,
    }

    let rows = match sqlx::query_as::<_, AbilityTreeRow>(
        "SELECT tree_index, ability_id \
         FROM resources.archetype_ability_tree \
         WHERE archetype = $1::resources.\"EArchetype\" \
         ORDER BY tree_index, ability_index",
    )
    .bind(archetype)
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::error!(archetype_id, archetype, "Failed to query ability tree: {e}");
            return None;
        }
    };

    if rows.is_empty() {
        return None;
    }

    // The DB schema constrains tree_index to a valid range — if a row violates
    // that, the data set is corrupted and silently skipping rows would ship a
    // partial ability tree. Bail with None so the caller's fallback (the
    // archetype-derived default tree) takes over instead.
    let mut ability_tree = AbilityTreeData::default();
    for row in rows {
        let tree_index = match usize::try_from(row.tree_index) {
            Ok(idx) if idx < ability_tree.trees.len() => idx,
            _ => {
                tracing::error!(
                    archetype_id,
                    tree_index = row.tree_index,
                    tree_count = ability_tree.trees.len(),
                    "Ability tree index out of range — schema constraint violated; bailing to fallback tree"
                );
                return None;
            }
        };
        ability_tree.trees[tree_index].push(row.ability_id);
    }

    Some(ability_tree)
}
