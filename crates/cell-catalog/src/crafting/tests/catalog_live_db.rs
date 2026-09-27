//! `CraftingCatalog::load` against the seeded database (audit C-20 to C-24).

use super::super::*;
use crate::test_support::require_db_or_skip;

/// The seed's shape, as a fresh `db/database.sql` loads it: 78 disciplines,
/// 498 blueprints, 2,556 component rows. Blueprint 21 has no components and
/// blueprint 412 has four alternative sets. Every item is loaded with its
/// crafting columns, and the flag facts the verbs rely on hold.
#[tokio::test]
async fn catalog_loads_seed_counts_sets_and_item_attrs() {
    let pool = require_db_or_skip!();
    let catalog = CraftingCatalog::load(&pool).await.expect("catalog load");

    assert_eq!(catalog.disciplines.len(), 78, "disciplines (C-20)");
    assert_eq!(catalog.blueprints.len(), 498, "blueprints (C-22)");
    assert_eq!(catalog.component_count(), 2556, "component rows (C-23)");

    let items: i64 = sqlx::query_scalar("SELECT count(*) FROM resources.items")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(catalog.items.len() as i64, items, "one entry per item");

    // Blueprint 21 (Ambernol Vial) has no component rows.
    let bp21 = catalog.blueprint(21).expect("blueprint 21");
    assert!(
        bp21.component_sets.is_empty(),
        "blueprint 21 has no components"
    );

    // Blueprint 412 (Titanium Plating): four alternative sets.
    let bp412 = catalog.blueprint(412).expect("blueprint 412");
    assert_eq!(bp412.discipline_id, Some(21));
    assert_eq!(bp412.product_id, Some(5401));
    assert_eq!(
        bp412
            .component_sets
            .iter()
            .map(|s| s.set_id)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
    let set1 = bp412.component_set(1).unwrap();
    assert_eq!(
        set1.components,
        [Component {
            item_id: 5254,
            quantity: 14
        }],
        "set 1 is 14x Steel Core"
    );
    assert_eq!(bp412.component_set(2).unwrap().components.len(), 2);

    // 40 alloy blueprints, each making 2 and needing elementary components.
    let alloys: Vec<_> = catalog.blueprints.values().filter(|b| b.is_alloy).collect();
    assert_eq!(alloys.len(), 40, "alloy blueprints (C-22)");
    assert!(alloys
        .iter()
        .all(|b| b.quantity == 2 && b.requires_elementary_components));

    // Discipline 21 is a real root: Common paradigm level 5 (C-21).
    let d21 = catalog.discipline(21).expect("discipline 21");
    assert_eq!(d21.racial_paradigm_level, 5);
    assert!(!d21.name.is_empty());

    // Item attributes: the research/reverse-engineer sample (audit section 2).
    let pistol = catalog.item(5481).expect("item 5481");
    assert_eq!(pistol.tech_comp, 20);
    assert_eq!(pistol.discipline_ids, [21, 22]);
    assert!(pistol.flags.is_researchable());
    assert!(pistol.flags.is_reverse_engineerable());

    // Kicker is on exactly items 5668-5671 (C-24).
    let mut kickers: Vec<i32> = catalog
        .items
        .iter()
        .filter(|(_, a)| a.flags.is_kicker())
        .map(|(&id, _)| id)
        .collect();
    kickers.sort_unstable();
    assert_eq!(kickers, [5668, 5669, 5670, 5671]);

    // The alloy sample: 5192 "Cell (Bio-Medical)" is tier 2, Good.
    let cell = catalog.item(5192).expect("item 5192");
    assert_eq!((cell.tier, cell.quality), (2, ItemQuality::Good));

    // Each loaded field comes from its own column: compare every item with
    // the raw row, so a swapped column fails here.
    type Raw = (i32, i32, i32, String, i32, Vec<i32>, Option<i32>);
    let rows: Vec<Raw> = sqlx::query_as(
        "SELECT item_id, flags, tier, quality_id::text, tech_comp, discipline_ids, \
                applied_science_id FROM resources.items",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    for (id, flags, tier, quality, tc, disciplines, science) in rows {
        let a = catalog.item(id).unwrap();
        assert_eq!(a.flags, ItemFlags(flags as u32), "item {id} flags");
        assert_eq!(a.tier, tier, "item {id} tier");
        assert_eq!(
            Some(a.quality),
            ItemQuality::from_db_label(&quality),
            "item {id}"
        );
        assert_eq!(a.tech_comp, tc, "item {id} tech_comp");
        assert_eq!(a.discipline_ids, disciplines, "item {id} disciplines");
        assert_eq!(a.applied_science_id, science, "item {id} applied science");
    }
}

/// The paradigm names crafting text uses are the seed's rows, in full.
#[tokio::test]
async fn racial_paradigm_names_match_the_seed() {
    let pool = require_db_or_skip!();
    let rows: Vec<(i32, String)> =
        sqlx::query_as("SELECT id, name FROM resources.racial_paradigm ORDER BY id")
            .fetch_all(&pool)
            .await
            .expect("racial_paradigm rows");
    let names: Vec<(i32, String)> = RACIAL_PARADIGM_NAMES
        .iter()
        .map(|&(id, name)| (id, name.to_string()))
        .collect();
    assert_eq!(rows, names);
    assert_eq!(racial_paradigm_name(3), Some("Goa'uld"));
    assert_eq!(racial_paradigm_name(6), None);
}
