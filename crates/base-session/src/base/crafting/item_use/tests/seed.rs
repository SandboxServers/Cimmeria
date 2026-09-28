//! The seed guard for `resources.crafting_item_effects`: it holds exactly
//! the resolved mapping, every id it names exists, each item is one kind,
//! and no content chain can also consume one of its items.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use crate::test_support::require_db_or_skip;

/// `(item_id, blueprint_id, racial_paradigm_id)` for every row.
type EffectRow = (i32, Option<i32>, Option<i32>);

/// The item ids the mapping CSV leaves unresolved (`confidence = none`),
/// and the resolved ones with their blueprint ids.
fn csv_mapping() -> (BTreeSet<i32>, BTreeMap<i32, Vec<i32>>) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/analysis/crafting/source/blueprint-items.csv");
    let text = std::fs::read_to_string(&path).expect("read blueprint-items.csv");
    let mut unresolved = BTreeSet::new();
    let mut resolved = BTreeMap::new();
    for line in text.lines().skip(1) {
        // item_id,item_name,blueprint_id,product_id,method,confidence,...
        // Names hold no commas before the confidence column except inside
        // the quoted note, which comes last, so split from the left.
        let cols: Vec<&str> = line.splitn(7, ',').collect();
        let item_id: i32 = cols[0].parse().expect("item id");
        match cols[5] {
            "high" | "medium-high" | "medium" => {
                let ids = cols[2]
                    .split(';')
                    .map(|b| b.parse().expect("blueprint id"))
                    .collect();
                resolved.insert(item_id, ids);
            }
            "none" => {
                unresolved.insert(item_id);
            }
            other => panic!("item {item_id}: unknown confidence {other:?}"),
        }
    }
    (unresolved, resolved)
}

#[tokio::test]
async fn live_db_seed_holds_exactly_the_resolved_mapping_and_the_guides() {
    let pool = require_db_or_skip!();
    let rows: Vec<EffectRow> = sqlx::query_as(
        "SELECT item_id, blueprint_id, racial_paradigm_id FROM resources.crafting_item_effects",
    )
    .fetch_all(&pool)
    .await
    .expect("read crafting_item_effects");

    let mut blueprints: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    let mut guides: BTreeMap<i32, Vec<i32>> = BTreeMap::new();
    for &(item_id, blueprint_id, paradigm_id) in &rows {
        match (blueprint_id, paradigm_id) {
            (Some(b), None) => blueprints.entry(item_id).or_default().push(b),
            (None, Some(p)) => guides.entry(item_id).or_default().push(p),
            other => panic!("item {item_id}: row {other:?} is not one effect"),
        }
    }
    for ids in blueprints.values_mut() {
        ids.sort_unstable();
    }

    let (unresolved, resolved) = csv_mapping();
    assert_eq!(resolved.len(), 193, "the CSV's resolved Blueprint items");
    assert_eq!(blueprints, resolved, "the seed is the CSV's resolved rows");
    assert_eq!(blueprints[&8882], vec![367, 369], "8882 teaches both");
    let leaked: Vec<&i32> = unresolved
        .iter()
        .filter(|id| blueprints.contains_key(id))
        .collect();
    assert!(leaked.is_empty(), "unresolved items seeded: {leaked:?}");

    // One guide per paradigm, and no item is both kinds.
    assert_eq!(
        guides,
        BTreeMap::from([
            (7805, vec![2]),
            (7806, vec![1]),
            (7807, vec![4]),
            (7808, vec![3]),
            (7809, vec![5]),
        ])
    );
    assert!(guides.keys().all(|id| !blueprints.contains_key(id)));

    // Every id exists and names what the row says it does.
    let dangling: Vec<(i32, String)> = sqlx::query_as(
        "SELECT e.item_id, 'item' FROM resources.crafting_item_effects e \
           LEFT JOIN resources.items i ON i.item_id = e.item_id WHERE i.item_id IS NULL \
         UNION ALL \
         SELECT e.item_id, 'blueprint ' || e.blueprint_id FROM resources.crafting_item_effects e \
           LEFT JOIN resources.blueprints b ON b.blueprint_id = e.blueprint_id \
           WHERE e.blueprint_id IS NOT NULL AND b.blueprint_id IS NULL \
         UNION ALL \
         SELECT e.item_id, 'paradigm ' || e.racial_paradigm_id FROM resources.crafting_item_effects e \
           LEFT JOIN resources.racial_paradigm p ON p.id = e.racial_paradigm_id \
           WHERE e.racial_paradigm_id IS NOT NULL AND p.id IS NULL \
         UNION ALL \
         SELECT e.item_id, 'guide named ' || i.name FROM resources.crafting_item_effects e \
           JOIN resources.items i ON i.item_id = e.item_id \
           JOIN resources.racial_paradigm p ON p.id = e.racial_paradigm_id \
           WHERE i.name <> 'Racial Paradigm Guide: ' || p.name",
    )
    .fetch_all(&pool)
    .await
    .expect("dangling-id query");
    assert!(dangling.is_empty(), "rows naming nothing: {dangling:?}");
}

/// The crafting use is the only consumer of these items: an `item_use`
/// content trigger on one would fire nowhere (the item-use path hands them
/// to crafting and never raises `OnItemUse`), and one that removed the item
/// would consume it twice.
#[tokio::test]
async fn live_db_no_content_trigger_listens_for_a_crafting_item() {
    let pool = require_db_or_skip!();
    let triggers: Vec<(i32, String)> = sqlx::query_as(
        "SELECT t.chain_id, t.event_key FROM resources.content_triggers t \
           JOIN resources.crafting_item_effects e ON t.event_key = e.item_id::text \
          WHERE t.event_type = 'item_use'",
    )
    .fetch_all(&pool)
    .await
    .expect("trigger query");
    assert!(
        triggers.is_empty(),
        "item_use chains on crafting items: {triggers:?}"
    );
}
