//! Unit tests for [`ServedCatalog`] and the publish slot. The raw PAKs are
//! synthetic: each test writes a made-up items PAK into a temp dir.

use std::path::Path;

use cimmeria_cooked_pak::catalog::decode_body;
use cimmeria_cooked_pak::pak::write_pak;

use super::*;
use crate::base::item_overrides::ITEM_ADDITIONS;

const ITEMS: u32 = 4;
const ITEMS_PAK: &str = "CookedDataItems.pak";
/// An item the registered override patches (its pre-fix XML shape).
const OVERRIDDEN_ITEM: u32 = 2893;
const PLAIN_ITEM: u32 = 900_001;
const OTHER_ITEM: u32 = 900_002;

/// Item XML in the shape the item patcher matches (`IconLocation`, `MaxStackSize`).
fn item_xml(item_id: u32, icon: &str, max_stack: u32) -> Vec<u8> {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
         <COOKED_ITEM ID=\"{item_id}\" IconLocation=\"{icon}\" Name=\"Item {item_id}\" Tier=\"1\">\
         <InventorySet IsDeletable=\"true\" MaxStackSize=\"{max_stack}\" />\
         </COOKED_ITEM>"
    )
    .into_bytes()
}

/// Add a raw entry for every item Cimmeria adds, so the cache skips the
/// additions (the PAK already ships those ids) and the changed set holds only
/// what each test asserts.
fn with_addition_ids(mut entries: BTreeMap<u32, Vec<u8>>) -> BTreeMap<u32, Vec<u8>> {
    for item in ITEM_ADDITIONS {
        entries
            .entry(item.item_id)
            .or_insert_with(|| item_xml(item.item_id, "set:Unchanged", 1));
    }
    entries
}

/// Write `entries` as the items PAK in `dir` and load a cache over it.
fn cache_over(dir: &Path, entries: &BTreeMap<u32, Vec<u8>>) -> ResourceCache {
    let file = std::fs::File::create(dir.join(ITEMS_PAK)).unwrap();
    write_pak(file, 7542, entries).unwrap();
    ResourceCache::load_all_with_mission_additions(dir.to_str().unwrap(), &[]).unwrap()
}

#[test]
fn every_overridden_key_whose_bytes_differ_is_changed() {
    let dir = tempfile::tempdir().unwrap();
    let raw = with_addition_ids(BTreeMap::from([
        (
            OVERRIDDEN_ITEM,
            item_xml(OVERRIDDEN_ITEM, "set:CoreWidgets image:IconMissing", 1),
        ),
        (
            PLAIN_ITEM,
            item_xml(PLAIN_ITEM, "set:ItemIcon001 image:SomeOther", 5),
        ),
    ]));
    let cache = cache_over(dir.path(), &raw);
    let served = cache.category(ITEMS).unwrap();
    assert_ne!(
        served.elements[&OVERRIDDEN_ITEM], raw[&OVERRIDDEN_ITEM],
        "the registered override must change item {OVERRIDDEN_ITEM}"
    );

    let catalog = ServedCatalog::build(&cache, dir.path().to_str().unwrap()).unwrap();
    let body = &catalog.category(ITEMS).unwrap().body;
    let decoded = decode_body(body, &Limits::DEFAULT).unwrap();

    let changed_keys: Vec<u32> = decoded.changed.keys().copied().collect();
    assert_eq!(changed_keys, vec![OVERRIDDEN_ITEM]);
    assert_eq!(
        decoded.changed[&OVERRIDDEN_ITEM],
        served.elements[&OVERRIDDEN_ITEM]
    );
    assert_eq!(decoded.version, served.metadata);

    let sorted: BTreeMap<u32, Vec<u8>> = served
        .elements
        .iter()
        .map(|(key, bytes)| (*key, bytes.clone()))
        .collect();
    let expected = category_digest(
        ITEMS,
        served.metadata,
        sorted.iter().map(|(key, bytes)| (*key, bytes.as_slice())),
    );
    assert_eq!(decoded.digest, expected);
}

#[test]
fn an_unchanged_category_has_no_changed_entries_and_lists_every_key() {
    let dir = tempfile::tempdir().unwrap();
    let raw = with_addition_ids(BTreeMap::from([
        (PLAIN_ITEM, item_xml(PLAIN_ITEM, "set:Plain", 1)),
        (OTHER_ITEM, item_xml(OTHER_ITEM, "set:Other", 2)),
    ]));
    let cache = cache_over(dir.path(), &raw);

    let catalog = ServedCatalog::build(&cache, dir.path().to_str().unwrap()).unwrap();
    let index: Index = serde_json::from_slice(&catalog.index_json()).unwrap();
    assert_eq!(index.schema, 1);
    assert_eq!(
        index.categories.len(),
        1,
        "only the held category is indexed"
    );
    let row = &index.categories[0];
    assert_eq!(row.category_id, ITEMS);
    assert_eq!(row.pak, ITEMS_PAK);
    assert_eq!(row.changed, 0);

    let decoded = decode_body(&catalog.category(ITEMS).unwrap().body, &Limits::DEFAULT).unwrap();
    assert!(decoded.changed.is_empty());
    let every_key: Vec<u32> = raw.keys().copied().collect();
    assert_eq!(decoded.keys, every_key);
    assert_eq!(row.entries as usize, every_key.len());
}

#[test]
fn a_served_key_the_raw_pak_lacks_is_changed_and_listed() {
    let dir = tempfile::tempdir().unwrap();
    // No `with_addition_ids`: the raw PAK lacks the items Cimmeria adds.
    let raw = BTreeMap::from([(PLAIN_ITEM, item_xml(PLAIN_ITEM, "set:Plain", 1))]);
    let cache = cache_over(dir.path(), &raw);
    assert!(!ITEM_ADDITIONS.is_empty());
    let id = ITEM_ADDITIONS[0].item_id;
    let served = cache.category(ITEMS).unwrap();
    assert!(served.elements.contains_key(&id));

    let catalog = ServedCatalog::build(&cache, dir.path().to_str().unwrap()).unwrap();
    let decoded = decode_body(&catalog.category(ITEMS).unwrap().body, &Limits::DEFAULT).unwrap();
    assert!(decoded.changed.contains_key(&id));
    assert_eq!(decoded.changed[&id], served.elements[&id]);
    assert!(decoded.keys.contains(&id));
    assert!(decoded.keys.windows(2).all(|w| w[0] < w[1]));
    assert!(!decoded.changed.contains_key(&PLAIN_ITEM));

    let index: Index = serde_json::from_slice(&catalog.index_json()).unwrap();
    assert_eq!(index.categories[0].entries as usize, served.elements.len());
}

#[test]
fn an_oversize_body_fails_the_build_naming_the_category() {
    let dir = tempfile::tempdir().unwrap();
    let raw = with_addition_ids(BTreeMap::from([(
        PLAIN_ITEM,
        item_xml(PLAIN_ITEM, "set:Plain", 1),
    )]));
    let cache = cache_over(dir.path(), &raw);
    let tiny = Limits {
        max_keys: 1_000_000,
        max_entry_bytes: 4 << 20,
        max_total_bytes: 16,
    };

    let err = ServedCatalog::build_with_limits(&cache, dir.path().to_str().unwrap(), &tiny)
        .err()
        .expect("a 16-byte cap refuses any category body");
    assert!(
        err.starts_with("category 4: category_too_large ("),
        "the error must name the category and the code, got: {err}"
    );
    assert_eq!(failure_code(&err), "category_too_large");
}

#[test]
fn a_changed_record_over_the_entry_cap_fails_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let raw = with_addition_ids(BTreeMap::from([(
        OVERRIDDEN_ITEM,
        item_xml(OVERRIDDEN_ITEM, "set:CoreWidgets image:IconMissing", 1),
    )]));
    let cache = cache_over(dir.path(), &raw);
    // The total cap is roomy; the per-record cap refuses the changed item,
    // which the launcher's decoder would otherwise reject as `bad_body`.
    let strict = Limits {
        max_keys: 1_000_000,
        max_entry_bytes: 1,
        max_total_bytes: 32 << 20,
    };

    let err = ServedCatalog::build_with_limits(&cache, dir.path().to_str().unwrap(), &strict)
        .err()
        .expect("a changed record over the entry cap fails the build");
    assert!(
        err.starts_with("category 4: category_too_large ("),
        "the error must name the category and the code, got: {err}"
    );
    assert_eq!(failure_code(&err), "category_too_large");
}

#[test]
fn an_unreadable_raw_pak_fails_the_build() {
    let dir = tempfile::tempdir().unwrap();
    let raw = with_addition_ids(BTreeMap::from([(
        PLAIN_ITEM,
        item_xml(PLAIN_ITEM, "set:Plain", 1),
    )]));
    let cache = cache_over(dir.path(), &raw);
    // The cache holds category 4 in memory; the raw file is now gone.
    std::fs::remove_file(dir.path().join(ITEMS_PAK)).unwrap();

    let err = ServedCatalog::build(&cache, dir.path().to_str().unwrap())
        .err()
        .expect("a missing raw PAK fails the build, it is not skipped");
    assert_eq!(err, "category 4: raw_pak_unreadable");
    assert_eq!(failure_code(&err), "raw_pak_unreadable");
}

#[test]
fn an_unknown_build_error_maps_to_build_failed() {
    assert_eq!(failure_code("something else went wrong"), "build_failed");
}

#[test]
fn publish_succeeds_once() {
    let slot = OnceLock::new();
    assert!(
        current_in(&slot).is_none(),
        "nothing is published before the build"
    );

    let ready = Published::Ready(Arc::new(ServedCatalog::from_bodies(Vec::new())));
    assert!(publish_into(&slot, ready));
    assert!(matches!(current_in(&slot), Some(Published::Ready(_))));

    assert!(!publish_into(&slot, Published::Failed("build_failed")));
    assert!(
        matches!(current_in(&slot), Some(Published::Ready(_))),
        "the first outcome stands"
    );
}
