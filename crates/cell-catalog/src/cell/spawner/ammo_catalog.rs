//! The ammo catalog (ammo campaign AM-F, issue #1026): which item holds each
//! special ammo type's reserve rounds (`resources.ammo_item_types`), and what
//! a shot fired with that type does differently (`resources.ammo_modifiers`).
//!
//! Loaded once at startup into `SpaceManager::ammo_catalog`, like every other
//! startup cache; a seed edit needs a server restart. Keys and the
//! `damage_type` are `EAmmoType` / `EDamageType` **ordinals**
//! (`cimmeria_entity::ammo_type`), converted in SQL the way `load_item_defs`
//! converts `default_ammo_type`.
//!
//! Per D-AM07 the server applies an [`AmmoModifier`] directly on every shot
//! fired with that type loaded; [`AmmoModifier::toggle_ability_id`] is
//! provenance only and is never cast.

use std::collections::HashMap;

use sqlx::PgPool;

/// One `resources.ammo_modifiers` row.
#[derive(Debug, Clone, Copy, PartialEq, sqlx::FromRow)]
pub struct AmmoModifier {
    /// `EAmmoType` ordinal (the table's primary key).
    pub ammo_type: i32,
    /// Multiplier on the shot's damage. The table's CHECK keeps it `> 0`.
    pub damage_mult: f32,
    /// Multiplier on the shot's penetration. The table's CHECK keeps it `> 0`.
    pub penetration_mult: f32,
    /// `EDamageType` ordinal that replaces the ability's damage type, or
    /// `None` to keep it.
    pub damage_type: Option<i32>,
    /// Effect applied to the target on a hit, if any.
    pub on_hit_effect_id: Option<i32>,
    /// The toggle ability the numbers were reconstructed from. Provenance
    /// only (D-AM07): never launched by the server.
    pub toggle_ability_id: i32,
    /// The ammo helps its target (a heal or a cleanse) and never harms it
    /// (AM-11d). A player's shot with it loaded may land on an ally or on
    /// the shooter and is refused at a hostile target. Explicit in the seed,
    /// never inferred from the on-hit effect's script.
    pub beneficial: bool,
}

/// The startup snapshot of both ammo tables.
#[derive(Debug, Clone, Default)]
pub struct AmmoCatalog {
    modifiers: HashMap<i32, AmmoModifier>,
    /// `EAmmoType` ordinal → reserve item design id.
    item_by_ammo_type: HashMap<i32, i32>,
    /// Reserve item design id → `EAmmoType` ordinal.
    ammo_type_by_item: HashMap<i32, i32>,
}

impl AmmoCatalog {
    /// Build a catalog from rows. A later row for the same key replaces an
    /// earlier one; the tables' primary and unique keys rule that out.
    pub fn from_rows(
        modifiers: impl IntoIterator<Item = AmmoModifier>,
        item_types: impl IntoIterator<Item = (i32, i32)>,
    ) -> Self {
        let item_by_ammo_type: HashMap<i32, i32> = item_types.into_iter().collect();
        let ammo_type_by_item = item_by_ammo_type.iter().map(|(&t, &i)| (i, t)).collect();
        Self {
            modifiers: modifiers.into_iter().map(|m| (m.ammo_type, m)).collect(),
            item_by_ammo_type,
            ammo_type_by_item,
        }
    }

    /// The modifier for `ammo_type`, or `None` when the type has no row
    /// (default ammo, or a family whose packet has not shipped yet): the
    /// shot fires unmodified.
    pub fn modifier(&self, ammo_type: i32) -> Option<&AmmoModifier> {
        self.modifiers.get(&ammo_type)
    }

    /// The reserve item design id for `ammo_type`, or `None` for a type with
    /// no reserve item (default ammo, daggers).
    pub fn item_id_for(&self, ammo_type: i32) -> Option<i32> {
        self.item_by_ammo_type.get(&ammo_type).copied()
    }

    /// The `EAmmoType` ordinal whose reserve item is `item_id`, or `None`
    /// when the item is not an ammo item.
    pub fn ammo_type_for_item(&self, item_id: i32) -> Option<i32> {
        self.ammo_type_by_item.get(&item_id).copied()
    }

    /// Whether `ability_id` is the toggle ability of some ammo row. The
    /// server never launches a toggle (D-AM07), but a press of one must not
    /// draw the "no effect" refusal either (ability-mechanics AB-12): it
    /// names ammo the player can load.
    pub fn is_toggle_ability(&self, ability_id: i32) -> bool {
        self.modifiers
            .values()
            .any(|m| m.toggle_ability_id == ability_id)
    }

    /// Number of modifier rows.
    pub fn modifier_count(&self) -> usize {
        self.modifiers.len()
    }

    /// Number of ammo types with a reserve item.
    pub fn item_type_count(&self) -> usize {
        self.item_by_ammo_type.len()
    }
}

/// Load `resources.ammo_modifiers`, keyed by `EAmmoType` ordinal.
pub async fn load_ammo_modifiers(pool: &PgPool) -> Result<HashMap<i32, AmmoModifier>, sqlx::Error> {
    let rows = sqlx::query_as::<_, AmmoModifier>(
        "SELECT (array_position(enum_range(NULL::resources.\"EAmmoType\"), ammo_type) - 1)::integer \
                    AS ammo_type, \
                damage_mult, penetration_mult, \
                (array_position(enum_range(NULL::resources.\"EDamageType\"), damage_type) - 1)::integer \
                    AS damage_type, \
                on_hit_effect_id, toggle_ability_id, beneficial \
         FROM resources.ammo_modifiers",
    )
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().map(|m| (m.ammo_type, m)).collect())
}

/// Load `resources.ammo_item_types` as `(EAmmoType ordinal, item design id)`.
pub async fn load_ammo_item_types(pool: &PgPool) -> Result<Vec<(i32, i32)>, sqlx::Error> {
    sqlx::query_as::<_, (i32, i32)>(
        "SELECT (array_position(enum_range(NULL::resources.\"EAmmoType\"), ammo_type) - 1)::integer, \
                item_id \
         FROM resources.ammo_item_types",
    )
    .fetch_all(pool)
    .await
}

/// Load both tables into an [`AmmoCatalog`].
pub async fn load_ammo_catalog(pool: &PgPool) -> Result<AmmoCatalog, sqlx::Error> {
    let modifiers = load_ammo_modifiers(pool).await?;
    let item_types = load_ammo_item_types(pool).await?;
    let catalog = AmmoCatalog::from_rows(modifiers.into_values(), item_types);
    tracing::info!(
        target: "ammo",
        event = "catalog_loaded",
        modifiers = catalog.modifier_count(),
        item_types = catalog.item_type_count(),
        "Loaded ammo catalog"
    );
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cimmeria_entity::ammo_type::{BULLET_DEFAULT, BULLET_HOLLOW_POINT};

    const HP: AmmoModifier = AmmoModifier {
        ammo_type: BULLET_HOLLOW_POINT,
        damage_mult: 1.25,
        penetration_mult: 0.75,
        damage_type: None,
        on_hit_effect_id: None,
        toggle_ability_id: 715,
        beneficial: false,
    };

    #[test]
    fn lookups_hit_by_ammo_type_and_by_item() {
        let catalog = AmmoCatalog::from_rows([HP], [(BULLET_HOLLOW_POINT, 9001)]);
        assert_eq!(catalog.modifier(BULLET_HOLLOW_POINT), Some(&HP));
        assert_eq!(catalog.item_id_for(BULLET_HOLLOW_POINT), Some(9001));
        assert_eq!(catalog.ammo_type_for_item(9001), Some(BULLET_HOLLOW_POINT));
        assert_eq!(catalog.modifier_count(), 1);
        assert_eq!(catalog.item_type_count(), 1);
    }

    #[test]
    fn default_ammo_and_unknown_ids_miss() {
        let catalog = AmmoCatalog::from_rows([HP], [(BULLET_HOLLOW_POINT, 9001)]);
        assert_eq!(catalog.modifier(BULLET_DEFAULT), None);
        assert_eq!(catalog.item_id_for(BULLET_DEFAULT), None);
        // The ammo type is not an item id, and the item id is not a type.
        assert_eq!(catalog.ammo_type_for_item(BULLET_HOLLOW_POINT), None);
        assert_eq!(catalog.item_id_for(9001), None);
    }

    #[test]
    fn empty_catalog_modifies_nothing() {
        let catalog = AmmoCatalog::default();
        assert_eq!(catalog.modifier(BULLET_HOLLOW_POINT), None);
        assert_eq!(catalog.item_id_for(BULLET_HOLLOW_POINT), None);
        assert!(!catalog.is_toggle_ability(715));
    }

    /// A toggle ability is matched by ability id, never by ammo type or item.
    #[test]
    fn toggle_abilities_match_by_ability_id() {
        let catalog = AmmoCatalog::from_rows([HP], [(BULLET_HOLLOW_POINT, 9001)]);
        assert!(catalog.is_toggle_ability(715));
        assert!(!catalog.is_toggle_ability(719));
        assert!(!catalog.is_toggle_ability(BULLET_HOLLOW_POINT));
        assert!(!catalog.is_toggle_ability(9001));
    }
}
