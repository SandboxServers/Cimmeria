//! Pet-summon catalog: which entity template a summon ability spawns.
//!
//! Loaded once at startup from `resources.pet_summons` (pets campaign,
//! `docs/analysis/pets/`). The 2009 data never encoded this link: the summon
//! abilities (2826 Summon Straegis and its siblings) carry no effects, and the
//! editor-only "Spawn Mob" effects name no template, in the seed and in the
//! client's cooked abilities alike. So the binding is Cimmeria seed data, and
//! the ability pipeline asks [`PetSummonCatalog::pet_summon_for`] whether a
//! fired ability summons a pet.
//!
//! Like every other startup cache this is a snapshot: a seed edit needs a
//! server restart.

use std::collections::HashMap;

use sqlx::PgPool;

/// One `resources.pet_summons` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct PetSummon {
    /// The summon ability (`resources.abilities.ability_id`).
    pub ability_id: i32,
    /// The pet template it spawns (`resources.entity_templates`, 350-369).
    pub template_id: i32,
    /// How many pets from this ability one owner may have out at once
    /// (D-PT04: 1). The table's CHECK keeps it at 1 or more.
    pub max_active: i32,
}

/// Every summon ability's [`PetSummon`], keyed by ability id.
#[derive(Debug, Clone, Default)]
pub struct PetSummonCatalog {
    by_ability: HashMap<i32, PetSummon>,
}

impl PetSummonCatalog {
    /// Build a catalog from rows. A later row for the same ability replaces
    /// an earlier one; the table's primary key rules that out for DB rows.
    pub fn from_rows(rows: impl IntoIterator<Item = PetSummon>) -> Self {
        Self {
            by_ability: rows.into_iter().map(|r| (r.ability_id, r)).collect(),
        }
    }

    /// The summon row for `ability_id`, or `None` when the ability does not
    /// summon a pet.
    pub fn pet_summon_for(&self, ability_id: i32) -> Option<PetSummon> {
        self.by_ability.get(&ability_id).copied()
    }

    /// Number of summon abilities.
    pub fn len(&self) -> usize {
        self.by_ability.len()
    }

    /// True when no ability summons a pet (the table is empty or failed to
    /// load).
    pub fn is_empty(&self) -> bool {
        self.by_ability.is_empty()
    }
}

/// Load `resources.pet_summons` into a [`PetSummonCatalog`].
pub async fn load_pet_summons(pool: &PgPool) -> Result<PetSummonCatalog, sqlx::Error> {
    let rows = sqlx::query_as::<_, PetSummon>(
        "SELECT ability_id, template_id, max_active FROM resources.pet_summons",
    )
    .fetch_all(pool)
    .await?;
    let catalog = PetSummonCatalog::from_rows(rows);
    tracing::info!(count = catalog.len(), "Loaded pet summons");
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    const STRAEGIS: PetSummon = PetSummon {
        ability_id: 2826,
        template_id: 350,
        max_active: 1,
    };

    #[test]
    fn pet_summon_for_finds_the_row_by_ability_id() {
        let catalog = PetSummonCatalog::from_rows([STRAEGIS]);
        assert_eq!(catalog.pet_summon_for(2826), Some(STRAEGIS));
        assert_eq!(catalog.len(), 1);
    }

    #[test]
    fn pet_summon_for_misses_a_non_summon_ability() {
        let catalog = PetSummonCatalog::from_rows([STRAEGIS]);
        // 1643 Summon Jaffa has no row until PT-11; 592 is Pistol Shot.
        assert_eq!(catalog.pet_summon_for(1643), None);
        assert_eq!(catalog.pet_summon_for(592), None);
        // The template id is not a key.
        assert_eq!(catalog.pet_summon_for(350), None);
    }

    #[test]
    fn empty_catalog_summons_nothing() {
        let catalog = PetSummonCatalog::default();
        assert!(catalog.is_empty());
        assert_eq!(catalog.pet_summon_for(2826), None);
    }
}
