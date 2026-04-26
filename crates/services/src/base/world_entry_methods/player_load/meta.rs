use sqlx::PgPool;

use cimmeria_entity::abilities::AbilityTreeData;

use crate::mercury::{PlayerLoadData, archetype_ability_tree};

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
        active_weapon_clip_size: 0,
        active_ammo_type: 0,
        ability_tree: archetype_ability_tree(1),
        items: vec![],
    }
}

/// Query bandolier items from the database for a player.
///
/// Returns tuples of (slot_id, BandolierItem) containing equipped weapon info.
pub async fn query_bandolier_items(
    db_pool: &Option<std::sync::Arc<PgPool>>,
    player_id: i32,
) -> Vec<(i32, cimmeria_entity::cell_entity::BandolierItem)> {
    let pool = match db_pool {
        Some(p) => p,
        None => return vec![],
    };

    #[derive(sqlx::FromRow)]
    struct Row {
        slot_id: i32,
        item_id: i32,
        clip_size: i32,
        default_ammo_type_id: i32,
    }

    sqlx::query_as::<_, Row>(
        r#"
        SELECT inv.slot_id, inv.type_id AS item_id, COALESCE(ri.clip_size, 0) AS clip_size,
               CASE WHEN ri.default_ammo_type IS NULL THEN 0
                    ELSE array_position(enum_range(NULL::resources."EAmmoType"), ri.default_ammo_type) - 1
               END AS default_ammo_type_id
        FROM sgw_inventory inv
        JOIN resources.items ri ON ri.item_id = inv.type_id
        WHERE inv.character_id = $1 AND inv.container_id = 3
        ORDER BY inv.slot_id
        "#,
    )
    .bind(player_id)
    .fetch_all(pool.as_ref())
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|row| {
        (
            row.slot_id,
            cimmeria_entity::cell_entity::BandolierItem {
                item_id: row.item_id,
                clip_size: row.clip_size,
                default_ammo_type: row.default_ammo_type_id,
            },
        )
    })
    .collect()
}

/// Query archetype ability tree data from the database.
pub async fn query_archetype_ability_tree(pool: &PgPool, archetype_id: i32) -> Option<AbilityTreeData> {
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

    let mut ability_tree = AbilityTreeData::default();
    for row in rows {
        let Ok(tree_index) = usize::try_from(row.tree_index) else {
            tracing::warn!(
                archetype_id,
                tree_index = row.tree_index,
                "Ignoring invalid ability tree index"
            );
            continue;
        };
        if let Some(tree) = ability_tree.trees.get_mut(tree_index) {
            tree.push(row.ability_id);
        } else {
            tracing::warn!(
                archetype_id,
                tree_index,
                "Ignoring out-of-range ability tree index"
            );
        }
    }

    Some(ability_tree)
}

/// Query active weapon stats from the player's equipped bandolier slot.
pub async fn query_active_weapon_stats(
    pool: &PgPool,
    player_id: i32,
    bandolier_slot: i32,
) -> (i32, i32) {
    #[derive(sqlx::FromRow)]
    struct ActiveWeaponRow {
        clip_size: i32,
        default_ammo_type_id: i32,
    }

    sqlx::query_as::<_, ActiveWeaponRow>(
        r#"
        SELECT COALESCE(ri.clip_size, 0) AS clip_size,
               CASE WHEN ri.default_ammo_type IS NULL THEN 0
                    ELSE array_position(enum_range(NULL::resources."EAmmoType"), ri.default_ammo_type) - 1
               END AS default_ammo_type_id
        FROM sgw_inventory inv
        JOIN resources.items ri ON ri.item_id = inv.type_id
        WHERE inv.character_id = $1
          AND inv.container_id = 3
          AND inv.slot_id = $2
        LIMIT 1
        "#,
    )
    .bind(player_id)
    .bind(bandolier_slot)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(|row| (row.clip_size, row.default_ammo_type_id))
    .unwrap_or((0, 0))
}