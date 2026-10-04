//! What a new character starts with besides its looks: the starter abilities
//! (`resources.char_creation_abilities`), the starter items
//! (`resources.char_creation_items`, the pistol every class spawns with) and
//! the inventory rows for both those and the item-bearing visual choices.
//!
//! The seeded playtest characters in `db/sgw/Players/Seed/sgw_player.sql` and
//! `db/sgw/Inventory/Seed/sgw_inventory.sql` are copies of what this module
//! writes for a Praxis Commando; `seed_parity_live_db_tests` fails when the
//! two drift apart.

use std::collections::HashMap;
use std::net::SocketAddr;

use sqlx::PgPool;

use super::super::resources::{bag_min_slot, pick_first_open_bag, BAG_FILL_ORDER};

/// The bandolier container. A weapon placed here is the one the character
/// holds at spawn (`sgw_player.bandolier_slot` defaults to slot 0).
const INV_BANDOLIER: i32 = cimmeria_entity::inventory::INV_BANDOLIER;

/// One starter ability, with its name for the creation log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StarterAbility {
    pub(super) ability_id: i32,
    pub(super) ability_name: Option<String>,
}

/// One item the new character receives: a visual choice that carries an item
/// (prison clothes, glasses) or a `char_creation_items` row (the pistol).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct StarterItem {
    pub(super) item_type_id: i32,
    pub(super) stack_size: i32,
    pub(super) bound: bool,
    pub(super) durability: i32,
}

/// Where a starter item landed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PlacedItem {
    pub(super) item_type_id: i32,
    pub(super) item_name: Option<String>,
    pub(super) container_id: i32,
    pub(super) slot_id: i32,
    pub(super) ammo: i32,
}

/// The starter abilities for `char_def_id`, in ability-id order so a created
/// character's `abilities` array is the same every time (the seed copies it).
pub(super) async fn load_starter_abilities(pool: &PgPool, char_def_id: i32) -> Vec<StarterAbility> {
    let rows = sqlx::query_as::<_, (i32, Option<String>)>(
        "SELECT ca.ability_id, a.name \
         FROM resources.char_creation_abilities ca \
         LEFT JOIN resources.abilities a ON a.ability_id = ca.ability_id \
         WHERE ca.char_def_id = $1 \
         ORDER BY ca.ability_id",
    )
    .bind(char_def_id)
    .fetch_all(pool)
    .await;
    match rows {
        Ok(rows) => rows
            .into_iter()
            .map(|(ability_id, name)| StarterAbility {
                ability_id,
                ability_name: real_name(name),
            })
            .collect(),
        Err(e) => {
            tracing::error!(
                event = "starter_abilities_load_failed",
                reason = "db_error",
                char_def_id, // nt:id-only char_def rows carry no name column
                error = %e,
                "character_create: starting abilities lookup failed"
            );
            Vec::new()
        }
    }
}

/// The `char_creation_items` rows for `char_def_id`, in item-id order.
pub(super) async fn load_starter_items(pool: &PgPool, char_def_id: i32) -> Vec<StarterItem> {
    let rows = sqlx::query_as::<_, (i32, i32)>(
        "SELECT item_id, stack_size FROM resources.char_creation_items \
         WHERE char_def_id = $1 ORDER BY item_id",
    )
    .bind(char_def_id)
    .fetch_all(pool)
    .await;
    match rows {
        Ok(rows) => rows
            .into_iter()
            .map(|(item_type_id, stack_size)| StarterItem {
                item_type_id,
                stack_size,
                bound: false,
                durability: -1,
            })
            .collect(),
        Err(e) => {
            tracing::error!(
                event = "starter_items_load_failed",
                reason = "db_error",
                char_def_id, // nt:id-only char_def rows carry no name column
                error = %e,
                "character_create: starter items lookup failed"
            );
            Vec::new()
        }
    }
}

/// Insert `items` into `player_id`'s inventory, in order (Account.py:182-207).
///
/// Each item goes to the first bag in `BAG_FILL_ORDER` that it may live in
/// and that still has room, so clothes land on the body and a weapon lands
/// in the bandolier. A weapon starts with a full magazine (`ammo` =
/// `clip_size`): without it Pistol Shot is refused with NoAmmo on the first
/// press, and the starter pistol would need a reload before it fired.
pub(super) async fn insert_starter_inventory(
    pool: &PgPool,
    addr: SocketAddr,
    player_id: i32,
    player_name: &str,
    items: &[StarterItem],
) -> Vec<PlacedItem> {
    let mut slot_indices: HashMap<i32, i32> = HashMap::new();
    let mut placed = Vec::with_capacity(items.len());
    for item in items {
        let design = sqlx::query_as::<_, (Vec<i32>, i32, String)>(
            "SELECT container_sets, clip_size, name FROM resources.items WHERE item_id = $1",
        )
        .bind(item.item_type_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten();
        let (container_sets, clip_size, item_name) = match design {
            Some((sets, clip, name)) => (sets, clip, real_name(Some(name))),
            None => (Vec::new(), 0, None),
        };

        // Pick the first bag that's both valid for this item AND still has
        // room. Pre-fix this picked the first valid bag unconditionally and
        // `continue`d if it was full, so an item that could overflow to a
        // later bag was silently dropped (live observation 2026-06-02: item
        // 4343 lost at character create because its primary bag filled up
        // first while a later valid bag still had room).
        let Some(bag_id) = pick_first_open_bag(&container_sets, &slot_indices) else {
            // Either the item has no valid container (content gap) or every
            // valid container is genuinely full. Both are operator-actionable,
            // so the reason tells them apart.
            let reason = if BAG_FILL_ORDER.iter().any(|b| container_sets.contains(b)) {
                "all_valid_containers_full"
            } else {
                "no_valid_container"
            };
            tracing::warn!(
                event = "starter_item_dropped",
                reason,
                %addr,
                player_id,
                player_name,
                item_type_id = item.item_type_id,
                item_name = item_name.as_deref(),
                "character_create: starter item dropped"
            );
            continue;
        };

        let entry = slot_indices
            .entry(bag_id)
            .or_insert_with(|| bag_min_slot(bag_id));
        let slot_id = *entry;
        *entry += 1;
        let ammo = clip_size.max(0);

        if let Err(e) = sqlx::query(
            "INSERT INTO sgw_inventory \
             (container_id, slot_id, type_id, character_id, durability, bound, stack_size, ammo) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(bag_id)
        .bind(slot_id)
        .bind(item.item_type_id)
        .bind(player_id)
        .bind(item.durability)
        .bind(item.bound)
        .bind(item.stack_size)
        .bind(ammo)
        .execute(pool)
        .await
        {
            tracing::error!(
                event = "starter_item_insert_failed",
                reason = "db_error",
                %addr,
                player_id,
                player_name,
                item_type_id = item.item_type_id,
                item_name = item_name.as_deref(),
                error = %e,
                "character_create: failed to insert starter item"
            );
            continue;
        }
        placed.push(PlacedItem {
            item_type_id: item.item_type_id,
            item_name,
            container_id: bag_id,
            slot_id,
            ammo,
        });
    }
    placed
}

/// `592 Pistol Shot, 594 Strike` for the creation log line.
pub(super) fn describe_abilities(abilities: &[StarterAbility]) -> String {
    abilities
        .iter()
        .map(|a| match &a.ability_name {
            Some(name) => format!("{} {name}", a.ability_id),
            None => a.ability_id.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `55 SI 3 9mm Pistol @3/0 ammo 15, 3440 Prison Jacket @7/0` for the
/// creation log line: type id, name, container/slot, and the magazine when
/// the item has one.
pub(super) fn describe_items(items: &[PlacedItem]) -> String {
    items
        .iter()
        .map(|i| {
            let mut s = i.item_type_id.to_string();
            if let Some(name) = &i.item_name {
                s.push(' ');
                s.push_str(name);
            }
            s.push_str(&format!(" @{}/{}", i.container_id, i.slot_id));
            if i.ammo > 0 {
                s.push_str(&format!(" ammo {}", i.ammo));
            }
            s
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Whether a placed item is a loaded weapon in the bandolier: what makes
/// Pistol Shot fire at spawn.
pub(super) fn has_loaded_bandolier_weapon(items: &[PlacedItem]) -> bool {
    items
        .iter()
        .any(|i| i.container_id == INV_BANDOLIER && i.ammo > 0)
}

/// A seed placeholder (`NO ITEM NAME`) is not a name (Rule 6).
fn real_name(name: Option<String>) -> Option<String> {
    name.filter(|n| !cimmeria_names::is_placeholder(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn placed(item_type_id: i32, name: Option<&str>, container_id: i32, ammo: i32) -> PlacedItem {
        PlacedItem {
            item_type_id,
            item_name: name.map(str::to_string),
            container_id,
            slot_id: 0,
            ammo,
        }
    }

    #[test]
    fn abilities_describe_as_id_and_name() {
        let list = [
            StarterAbility {
                ability_id: 592,
                ability_name: Some("Pistol Shot".into()),
            },
            StarterAbility {
                ability_id: 9999,
                ability_name: None,
            },
        ];
        assert_eq!(describe_abilities(&list), "592 Pistol Shot, 9999");
    }

    #[test]
    fn items_describe_with_place_and_magazine() {
        let list = [
            placed(55, Some("SI 3 9mm Pistol"), 3, 15),
            placed(3437, None, 11, 0),
        ];
        assert_eq!(
            describe_items(&list),
            "55 SI 3 9mm Pistol @3/0 ammo 15, 3437 @11/0"
        );
    }

    #[test]
    fn only_a_loaded_bandolier_weapon_counts_as_armed() {
        assert!(has_loaded_bandolier_weapon(&[placed(55, None, 3, 15)]));
        assert!(!has_loaded_bandolier_weapon(&[placed(55, None, 3, 0)]));
        assert!(!has_loaded_bandolier_weapon(&[placed(55, None, 1, 15)]));
    }

    #[test]
    fn a_placeholder_name_is_unresolved() {
        assert_eq!(real_name(Some("NO ITEM NAME".into())), None);
        assert_eq!(
            real_name(Some("Prison Jacket".into())),
            Some("Prison Jacket".into())
        );
    }
}
