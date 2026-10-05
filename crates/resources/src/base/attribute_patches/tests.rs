//! Guards for the cooked attribute patches (DA-F6): what the client is
//! served, that a client on the shipped category resyncs, and agreement with
//! the seed.

use std::io::Read;

use super::{ATTRIBUTE_PATCHES, CATEGORY_ABILITIES, CATEGORY_ERROR_STRINGS, MEDKIT_ICON};
use crate::base::resources::ResourceCache;

fn repo(rel: &str) -> String {
    format!("{}/../../{rel}", env!("CARGO_MANIFEST_DIR"))
}

fn data_dir() -> String {
    repo("data/cache")
}

/// Read a shipped PAK straight from the ZIP, so "shipped" never means
/// "already patched".
fn shipped(pak: &str, name: &str) -> Vec<u8> {
    let file = std::fs::File::open(format!("{}/{pak}", data_dir())).expect("PAK opens");
    let mut zip = zip::ZipArchive::new(file).expect("PAK is a ZIP");
    let mut entry = zip.by_name(name).expect("entry ships");
    let mut buf = Vec::new();
    entry.read_to_end(&mut buf).expect("entry reads");
    buf
}

fn shipped_metadata(pak: &str) -> u32 {
    let bytes = shipped(pak, "MetaData");
    u32::from_le_bytes(bytes[..4].try_into().unwrap())
}

fn served(cache: &ResourceCache, category: u32, id: u32) -> String {
    String::from_utf8(cache.get(category, id).expect("served").clone()).unwrap()
}

/// The two starter health heals are served with the Medkit icon and the
/// range error with readable text; nothing else in either entry changes;
/// both categories are listed and their metadata moves off the shipped
/// value, so a client holding the shipped PAK resyncs. Emptying
/// `ATTRIBUTE_PATCHES` or dropping the `load_all` call fails every assert.
#[test]
fn served_entries_carry_the_patches_and_both_categories_resync() {
    let cache = ResourceCache::load_all(&data_dir()).expect("committed PAKs load");

    for id in [1218, 1646] {
        let xml = served(&cache, CATEGORY_ABILITIES, id);
        assert!(
            xml.contains(&format!("IconLocation=\"{MEDKIT_ICON}\"")),
            "ability {id}: {xml}"
        );
        assert!(!xml.contains("IconMissing"), "ability {id}");
        let shipped = String::from_utf8(shipped("CookedDataAbilities.pak", &format!("_{id}")))
            .unwrap()
            .replace("set:CoreWidgets image:IconMissing", MEDKIT_ICON);
        assert_eq!(xml, shipped, "only the icon changes on ability {id}");
    }
    let text = served(&cache, CATEGORY_ERROR_STRINGS, 42);
    assert!(
        text.contains("Text=\"Your target is out of range\""),
        "{text}"
    );
    assert!(
        text.contains("MonikerName=\"CONDITION_FEEDBACK_OutsideWeaponRange\""),
        "the moniker stays: {text}"
    );

    assert_eq!(cache.overridden_elements(CATEGORY_ABILITIES), &[1218, 1646]);
    assert_eq!(cache.overridden_elements(CATEGORY_ERROR_STRINGS), &[42]);
    assert_ne!(
        cache.category(CATEGORY_ABILITIES).unwrap().metadata,
        shipped_metadata("CookedDataAbilities.pak")
    );
    assert_ne!(
        cache.category(CATEGORY_ERROR_STRINGS).unwrap().metadata,
        shipped_metadata("ErrorStrings.pak")
    );
    // An untouched neighbour is served as shipped.
    assert_eq!(
        cache.get(CATEGORY_ABILITIES, 597),
        Some(&shipped("CookedDataAbilities.pak", "_597"))
    );
}

/// Every patch targets an entry the PAK ships (a missing one would be
/// skipped silently at startup) and no other override pass owns its
/// category: `load_all` merges the per-category id lists with `extend`,
/// which would replace one pass's list with another's. The second half is
/// read off the served cache, not a hand-kept list of the other passes: a
/// patched category must list exactly this module's ids, so a future pass
/// that also touches it fails here whichever of the two `extend`s wins.
#[test]
fn patches_target_shipped_entries_in_categories_no_other_pass_owns() {
    let cache = ResourceCache::load_all(&data_dir()).expect("committed PAKs load");
    for p in ATTRIBUTE_PATCHES {
        let mut ours: Vec<u32> = ATTRIBUTE_PATCHES
            .iter()
            .filter(|q| q.category == p.category)
            .map(|q| q.element_id)
            .collect();
        ours.sort_unstable();
        assert_eq!(
            cache.overridden_elements(p.category),
            ours.as_slice(),
            "category {} must be listed with exactly this module's patches",
            p.category
        );
        let pak = match p.category {
            CATEGORY_ABILITIES => "CookedDataAbilities.pak",
            CATEGORY_ERROR_STRINGS => "ErrorStrings.pak",
            other => panic!("no PAK mapping for category {other} in this test"),
        };
        let xml = String::from_utf8(shipped(pak, &format!("_{}", p.element_id))).unwrap();
        assert!(
            xml.contains(&format!(" {}=\"", p.attribute)),
            "{p:?} must name an attribute the shipped entry has"
        );
    }
}

/// The seed carries the same values, so `resources.abilities.icon` and
/// `resources.error_texts.text` (the names book, tools, the content editor)
/// agree with what the client is served. Changing a patch without its seed
/// row, or the reverse, fails here.
#[test]
fn seed_rows_match_the_patches() {
    let abilities = std::fs::read_to_string(repo("db/resources/Abilities/Seed/abilities.sql"))
        .expect("abilities seed");
    let errors = std::fs::read_to_string(repo("db/resources/Texts/Seed/error_texts.sql"))
        .expect("error_texts seed");
    for p in ATTRIBUTE_PATCHES {
        let (seed, row_start) = match p.category {
            CATEGORY_ABILITIES => (&abilities, format!("VALUES ({}, ", p.element_id)),
            CATEGORY_ERROR_STRINGS => (&errors, format!("VALUES ({}, ", p.element_id)),
            other => panic!("no seed mapping for category {other}"),
        };
        let row = seed
            .split("INSERT INTO ")
            .find(|r| r.contains(&row_start))
            .unwrap_or_else(|| panic!("{p:?}: no seed row"));
        assert!(
            row.contains(&format!("'{}'", p.value)),
            "{p:?}: the seed row must carry the patched value"
        );
    }
}

/// A zero bump would leave clients on the shipped version; the low bit keeps
/// every patched category's bump non-zero. The bump is FNV-1a, so its value
/// is fixed across toolchains: pin it, so a change to the hash or to the
/// patches is a visible edit here rather than a silent client resync.
#[test]
fn every_bump_is_non_zero_and_stable() {
    for category in [CATEGORY_ABILITIES, CATEGORY_ERROR_STRINGS] {
        assert_eq!(super::bump_for(category) & 1, 1, "category {category}");
    }
    assert_eq!(super::bump_for(CATEGORY_ABILITIES), PINNED_ABILITIES_BUMP);
    assert_eq!(super::bump_for(CATEGORY_ERROR_STRINGS), PINNED_ERROR_BUMP);
}

/// FNV-1a of the two Medkit patches (computed independently in Python).
const PINNED_ABILITIES_BUMP: u32 = 0x4d34_0d53;
/// FNV-1a of the error-42 text patch.
const PINNED_ERROR_BUMP: u32 = 0xef2d_c891;
