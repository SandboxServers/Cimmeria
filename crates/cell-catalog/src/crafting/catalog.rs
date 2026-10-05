//! `CraftingCatalog`: disciplines, blueprints with their component sets, and
//! the crafting attributes of every item, loaded once from `resources.*`.

use std::collections::HashMap;

use sqlx::PgPool;

use super::{ItemFlags, ItemQuality};

/// One `resources.disciplines` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discipline {
    pub discipline_id: i32,
    pub applied_science_id: i32,
    pub racial_paradigm_id: i32,
    /// The paradigm level a player needs to learn it (audit C-21).
    pub racial_paradigm_level: i32,
    pub tech_competency: i32,
    /// Disciplines that must be known first (`required_discipline_ids`).
    pub required_discipline_ids: Vec<i32>,
    pub name: String,
}

/// One component requirement of a component set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Component {
    /// Item design id (`resources.items.item_id`).
    pub item_id: i32,
    pub quantity: i32,
}

/// One recipe for a blueprint. A blueprint's sets are **alternatives**: a
/// craft uses exactly one of them, not all in turn (audit C-23).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentSet {
    /// `component_set_id`, 1-4 in the seed.
    pub set_id: i32,
    /// Ordered by `item_id`.
    pub components: Vec<Component>,
}

/// One `resources.blueprints` row with its component sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blueprint {
    pub blueprint_id: i32,
    pub discipline_id: Option<i32>,
    pub is_alloy: bool,
    /// Item design id of the product.
    pub product_id: Option<i32>,
    /// Units of the product one craft makes.
    pub quantity: i32,
    pub requires_elementary_components: bool,
    /// Ordered by `set_id`. Empty for a blueprint no seed row feeds (21).
    pub component_sets: Vec<ComponentSet>,
}

impl Blueprint {
    /// The component set with id `set_id`.
    pub fn component_set(&self, set_id: i32) -> Option<&ComponentSet> {
        self.component_sets.iter().find(|s| s.set_id == set_id)
    }
}

/// The crafting columns of one `resources.items` row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CraftItemAttrs {
    pub flags: ItemFlags,
    pub tier: i32,
    pub quality: ItemQuality,
    /// Tech competency (`tech_comp`).
    pub tech_comp: i32,
    pub discipline_ids: Vec<i32>,
    pub applied_science_id: Option<i32>,
}

/// Every discipline, blueprint and item's crafting attributes, keyed by id.
#[derive(Debug, Clone, Default)]
pub struct CraftingCatalog {
    pub disciplines: HashMap<i32, Discipline>,
    pub blueprints: HashMap<i32, Blueprint>,
    pub items: HashMap<i32, CraftItemAttrs>,
}

/// One `resources.blueprints_components` row, before grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentRow {
    pub blueprint_id: i32,
    pub set_id: i32,
    pub item_id: i32,
    pub quantity: i32,
}

impl CraftingCatalog {
    /// Build a catalog from rows. Component rows are grouped into their
    /// blueprint's sets by `component_set_id`; a row naming a blueprint that
    /// is not in `blueprints` is dropped with a WARN (the schema's foreign
    /// key makes that unreachable from the database).
    pub fn from_rows(
        disciplines: impl IntoIterator<Item = Discipline>,
        blueprints: impl IntoIterator<Item = Blueprint>,
        components: impl IntoIterator<Item = ComponentRow>,
        items: impl IntoIterator<Item = (i32, CraftItemAttrs)>,
    ) -> Self {
        let mut catalog = Self {
            disciplines: disciplines
                .into_iter()
                .map(|d| (d.discipline_id, d))
                .collect(),
            blueprints: blueprints
                .into_iter()
                .map(|b| (b.blueprint_id, b))
                .collect(),
            items: items.into_iter().collect(),
        };
        for row in components {
            let Some(blueprint) = catalog.blueprints.get_mut(&row.blueprint_id) else {
                tracing::warn!(
                    target: "crafting",
                    event = "catalog_orphan_component",
                    blueprint_id = row.blueprint_id, // nt:id-only unknown blueprint, no row to name
                    item_id = row.item_id,
                    item_name = cimmeria_names::book().item(row.item_id),
                    "crafting catalog: component row names an unknown blueprint, dropped"
                );
                continue;
            };
            let component = Component {
                item_id: row.item_id,
                quantity: row.quantity,
            };
            match blueprint
                .component_sets
                .iter_mut()
                .find(|s| s.set_id == row.set_id)
            {
                Some(set) => set.components.push(component),
                None => blueprint.component_sets.push(ComponentSet {
                    set_id: row.set_id,
                    components: vec![component],
                }),
            }
        }
        for blueprint in catalog.blueprints.values_mut() {
            blueprint.component_sets.sort_by_key(|s| s.set_id);
            for set in &mut blueprint.component_sets {
                set.components.sort_by_key(|c| c.item_id);
            }
        }
        catalog
    }

    pub fn discipline(&self, discipline_id: i32) -> Option<&Discipline> {
        self.disciplines.get(&discipline_id)
    }

    pub fn blueprint(&self, blueprint_id: i32) -> Option<&Blueprint> {
        self.blueprints.get(&blueprint_id)
    }

    pub fn item(&self, item_id: i32) -> Option<&CraftItemAttrs> {
        self.items.get(&item_id)
    }

    /// Total component rows across every blueprint and set.
    pub fn component_count(&self) -> usize {
        self.blueprints
            .values()
            .flat_map(|b| &b.component_sets)
            .map(|s| s.components.len())
            .sum()
    }

    /// Load the catalog.
    ///
    /// An item whose `quality_id` label this build does not know is skipped
    /// with a WARN rather than failing the load; the enum has exactly five
    /// labels, all pinned by the constants test.
    pub async fn load(pool: &PgPool) -> Result<Self, sqlx::Error> {
        #[derive(sqlx::FromRow)]
        struct DisciplineRow {
            discipline_id: i32,
            applied_science_id: i32,
            racial_paradigm_id: i32,
            racial_paradigm_level: i32,
            tech_competency: i32,
            required_discipline_ids: Vec<i32>,
            name: String,
        }
        #[derive(sqlx::FromRow)]
        struct BlueprintRow {
            blueprint_id: i32,
            discipline_id: Option<i32>,
            is_alloy: Option<bool>,
            product_id: Option<i32>,
            quantity: i32,
            requires_elementary_components: Option<bool>,
        }
        #[derive(sqlx::FromRow)]
        struct ComponentDbRow {
            blueprint_id: i32,
            item_id: i32,
            quantity: i32,
            component_set_id: i32,
        }
        #[derive(sqlx::FromRow)]
        struct ItemRow {
            item_id: i32,
            flags: i32,
            tier: i32,
            quality: String,
            tech_comp: i32,
            discipline_ids: Vec<i32>,
            applied_science_id: Option<i32>,
        }

        let disciplines = sqlx::query_as::<_, DisciplineRow>(
            "SELECT discipline_id, applied_science_id, racial_paradigm_id, \
                    racial_paradigm_level, tech_competency, required_discipline_ids, name \
             FROM resources.disciplines",
        )
        .fetch_all(pool)
        .await?;
        let blueprints = sqlx::query_as::<_, BlueprintRow>(
            "SELECT blueprint_id, discipline_id, is_alloy, product_id, quantity, \
                    requires_elementary_components \
             FROM resources.blueprints",
        )
        .fetch_all(pool)
        .await?;
        let components = sqlx::query_as::<_, ComponentDbRow>(
            "SELECT blueprint_id, item_id, quantity, component_set_id \
             FROM resources.blueprints_components",
        )
        .fetch_all(pool)
        .await?;
        let items = sqlx::query_as::<_, ItemRow>(
            "SELECT item_id, flags, tier, quality_id::text AS quality, tech_comp, \
                    discipline_ids, applied_science_id \
             FROM resources.items",
        )
        .fetch_all(pool)
        .await?;

        let items = items.into_iter().filter_map(|r| {
            let Some(quality) = ItemQuality::from_db_label(&r.quality) else {
                tracing::warn!(
                    target: "crafting",
                    event = "catalog_unknown_quality",
                    item_id = r.item_id,
                    item_name = cimmeria_names::book().item(r.item_id),
                    quality = %r.quality,
                    "crafting catalog: item has an unknown quality label, skipped"
                );
                return None;
            };
            Some((
                r.item_id,
                CraftItemAttrs {
                    // The column is a signed `integer` holding a UINT32
                    // bitfield; the top bit is unused, so the cast is exact.
                    flags: ItemFlags(r.flags as u32),
                    tier: r.tier,
                    quality,
                    tech_comp: r.tech_comp,
                    discipline_ids: r.discipline_ids,
                    applied_science_id: r.applied_science_id,
                },
            ))
        });

        let catalog = Self::from_rows(
            disciplines.into_iter().map(|r| Discipline {
                discipline_id: r.discipline_id,
                applied_science_id: r.applied_science_id,
                racial_paradigm_id: r.racial_paradigm_id,
                racial_paradigm_level: r.racial_paradigm_level,
                tech_competency: r.tech_competency,
                required_discipline_ids: r.required_discipline_ids,
                name: r.name,
            }),
            blueprints.into_iter().map(|r| Blueprint {
                blueprint_id: r.blueprint_id,
                discipline_id: r.discipline_id,
                is_alloy: r.is_alloy.unwrap_or(false),
                product_id: r.product_id,
                quantity: r.quantity,
                requires_elementary_components: r.requires_elementary_components.unwrap_or(false),
                component_sets: Vec::new(),
            }),
            components.into_iter().map(|r| ComponentRow {
                blueprint_id: r.blueprint_id,
                set_id: r.component_set_id,
                item_id: r.item_id,
                quantity: r.quantity,
            }),
            items,
        );

        tracing::info!(
            target: "crafting",
            event = "catalog_loaded",
            disciplines = catalog.disciplines.len(),
            blueprints = catalog.blueprints.len(),
            components = catalog.component_count(),
            items = catalog.items.len(),
            "Loaded crafting catalog"
        );
        Ok(catalog)
    }
}
