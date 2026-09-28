//! Deployable catalog: which entity template a deployable ability places,
//! and which of its effects time it and hit with it.
//!
//! Loaded once at startup from `resources.deployables` (deployables Phase 0,
//! `docs/gameplay/deployables.md`). The 2009 data describes a deployable
//! only in prose: 1012 "Deployable: Microwave Emitter" carries a "Pulser"
//! effect (5065, "30 pulses x1 Second duration / Despawn Target on Finish")
//! and a "Damage" effect (5066, "Medium Radius AE / Secondary -100F"), and
//! neither names a template or says which one rides on the other. So the
//! binding is Cimmeria seed data, as `pet_summons` is for pets, and the
//! ability pipeline asks [`DeployableCatalog::deployable_for`] whether a fired
//! ability places a deployable.
//!
//! Like every other startup cache this is a snapshot: a seed edit needs a
//! server restart.

use std::collections::HashMap;

use sqlx::PgPool;

/// One `resources.deployables` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::FromRow)]
pub struct DeployableSpec {
    /// The deployable ability (`resources.abilities.ability_id`).
    pub ability_id: i32,
    /// The template it places (`resources.entity_templates`, 380-389).
    pub template_id: i32,
    /// The effect whose `pulse_count` x `pulse_duration` is the deployable's
    /// lifetime and pulse cadence (1012: 5065 "Pulser", 30 x 1 s).
    pub lifetime_effect_id: i32,
    /// The effect each pulse applies around the deployable (1012: 5066
    /// "Damage", `TCM_AERadius` "Medium").
    pub pulse_effect_id: i32,
    /// How many deployables from this ability one owner may have out at
    /// once. A re-cast past the cap removes the owner's oldest one. The
    /// table's CHECK keeps it at 1 or more.
    pub max_active: i32,
}

/// Every deployable ability's [`DeployableSpec`], keyed by ability id.
#[derive(Debug, Clone, Default)]
pub struct DeployableCatalog {
    by_ability: HashMap<i32, DeployableSpec>,
}

impl DeployableCatalog {
    /// Build a catalog from rows. A later row for the same ability replaces
    /// an earlier one; the table's primary key rules that out for DB rows.
    pub fn from_rows(rows: impl IntoIterator<Item = DeployableSpec>) -> Self {
        Self {
            by_ability: rows.into_iter().map(|r| (r.ability_id, r)).collect(),
        }
    }

    /// The deployable row for `ability_id`, or `None` when the ability does
    /// not place a deployable.
    pub fn deployable_for(&self, ability_id: i32) -> Option<DeployableSpec> {
        self.by_ability.get(&ability_id).copied()
    }

    /// Number of deployable abilities.
    pub fn len(&self) -> usize {
        self.by_ability.len()
    }

    /// True when no ability places a deployable (the table is empty or
    /// failed to load).
    pub fn is_empty(&self) -> bool {
        self.by_ability.is_empty()
    }
}

/// Load `resources.deployables` into a [`DeployableCatalog`].
pub async fn load_deployables(pool: &PgPool) -> Result<DeployableCatalog, sqlx::Error> {
    let rows = sqlx::query_as::<_, DeployableSpec>(
        "SELECT ability_id, template_id, lifetime_effect_id, pulse_effect_id, max_active \
         FROM resources.deployables",
    )
    .fetch_all(pool)
    .await?;
    let catalog = DeployableCatalog::from_rows(rows);
    tracing::info!(count = catalog.len(), "Loaded deployables");
    Ok(catalog)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MICROWAVE: DeployableSpec = DeployableSpec {
        ability_id: 1012,
        template_id: 380,
        lifetime_effect_id: 5065,
        pulse_effect_id: 5066,
        max_active: 1,
    };

    #[test]
    fn deployable_for_finds_the_row_by_ability_id() {
        let catalog = DeployableCatalog::from_rows([MICROWAVE]);
        assert_eq!(catalog.deployable_for(1012), Some(MICROWAVE));
        assert_eq!(catalog.len(), 1);
    }

    #[test]
    fn deployable_for_misses_other_ids() {
        let catalog = DeployableCatalog::from_rows([MICROWAVE]);
        // 1236 Aggression Inducer has no row in Phase 0; 592 is Pistol Shot.
        assert_eq!(catalog.deployable_for(1236), None);
        assert_eq!(catalog.deployable_for(592), None);
        // Neither the template nor an effect id is a key.
        assert_eq!(catalog.deployable_for(380), None);
        assert_eq!(catalog.deployable_for(5065), None);
    }

    #[test]
    fn empty_catalog_places_nothing() {
        let catalog = DeployableCatalog::default();
        assert!(catalog.is_empty());
        assert_eq!(catalog.deployable_for(1012), None);
    }
}
