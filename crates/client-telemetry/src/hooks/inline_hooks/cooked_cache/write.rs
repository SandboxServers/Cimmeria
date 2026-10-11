//! What the cache writes to disk, and which tier the bytes belong to.
//!
//! Two functions carry every cooked-cache write (checked against the QA
//! `SGW.exe` with headless Ghidra on 2026-10-11):
//!
//! | Function | Address | Signature |
//! |---|---|---|
//! | `ZipStorageBase::WriteStreamToFile` | `0x00479930` | `thiscall(this, const wchar_t* entry, std::ostrstream*)`, `ret 8`, `bool` |
//! | source-archive copy (`CopyFileW` wrapper) | `0x0047ab90` | `cdecl(const wchar_t* source, const wchar_t* cache)`, `bool` |
//!
//! `WriteStreamToFile` adds one entry to the storage's open archive
//! (`this + 0x20`). That archive was opened under the writable cache
//! directory (`CachePath`, `Cache.en-US`), so every pushed element
//! (`ServerSource_WriteElement` `0x0043bdb0` through `0x0043a9d0`, entry
//! `_<key>`) and every version stamp (`WriteMetaDataVersion` `0x00479e10`,
//! entry `MetaData`) lands in the `cache` tier. Its three callers are those
//! two and `0x009060b0`.
//!
//! The `source_cache` tier is never written. The only thing that touches it
//! is `UpdateFromSourceArchives` (`0x00479fa0`, `ZipStorage.cpp:0x44..0x6b`):
//! when the bundled PAK in `SourceCachePath` (`SourceCache.en-us`) holds a
//! newer `MetaData` version than the storage, it copies the whole PAK over the
//! cache one with `0x0047ab90(source, cache)`. That copy is the event with
//! `tier = source_cache`: the bytes come from the bundled tier and replace the
//! cache PAK. The function's own error line is `Error copying source archive`.
//!
//! The event is `client.cooked_cache.write {category_id, version, tier,
//! result}` plus `pak` and `entry_kind`. `category_id` comes from the PAK name
//! (the storage keeps no category id where the telemetry reads: its `+0x04` is
//! the PAK-name vector, see [`layout`](super::layout)). `version` is what the
//! storage holds after the write, so a `MetaData` stamp reports the version it
//! wrote. A source copy has no storage in hand (the function is `cdecl` and
//! takes two paths), and the version it installs is read into a local, so its
//! `version` is `null`.
//!
//! A resync pushes tens of thousands of elements. Failed writes, `MetaData`
//! stamps and source copies are always reported; a successful element write is
//! reported once per storage and version, by [`should_report`].

// The detours that use most of this exist only in the 32-bit client build.
#![cfg_attr(not(all(target_os = "windows", target_arch = "x86")), allow(dead_code))]

use serde_json::json;

use crate::hooks::entity_trace::Fields;

pub(crate) const TARGET_WRITE: &str = "client.cooked_cache.write";

pub(crate) const ADDR_WRITE_STREAM: usize = 0x0047_9930;
pub(crate) const ADDR_COPY_SOURCE: usize = 0x0047_ab90;

/// The `WriteStreamToFile` entry name of a version stamp.
const METADATA_ENTRY: &str = "MetaData";

/// Which cache directory the bytes belong to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tier {
    /// `CachePath` (`Cache.en-US`): writable, the server's push lands here.
    Cache,
    /// `SourceCachePath` (`SourceCache.en-us`): bundled, only ever copied from.
    SourceCache,
}

impl Tier {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Tier::Cache => "cache",
            Tier::SourceCache => "source_cache",
        }
    }
}

/// What was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryKind {
    /// The 4-byte version stamp.
    MetaData,
    /// One pushed element, `_<key>`.
    Element,
    /// A whole bundled PAK copied over the cache one.
    SourceArchive,
}

impl EntryKind {
    pub(crate) fn name(self) -> &'static str {
        match self {
            EntryKind::MetaData => "metadata",
            EntryKind::Element => "element",
            EntryKind::SourceArchive => "source_archive",
        }
    }
}

/// The kind of entry `WriteStreamToFile` was asked to add.
pub(crate) fn entry_kind(entry: &str) -> EntryKind {
    if entry == METADATA_ENTRY {
        EntryKind::MetaData
    } else {
        EntryKind::Element
    }
}

/// The 21 `ServerSource` categories by PAK file name
/// (`docs/reverse-engineering/address-map.md`, Category to PAK mapping).
const CATEGORY_PAKS: [(u32, &str); 21] = [
    (1, "CookedDataKismetSeqEvent.pak"),
    (2, "CookedDataAbilities.pak"),
    (3, "CookedDataMissions.pak"),
    (4, "CookedDataItems.pak"),
    (5, "CookedDataDialogs.pak"),
    (6, "CookedDataKismetSetEvent.pak"),
    (7, "CookedCharCreation.pak"),
    (8, "CookedInteractionSet.pak"),
    (9, "CookedDataEffects.pak"),
    (10, "TextStrings.pak"),
    (11, "ErrorStrings.pak"),
    (12, "CookedWorldInfo.pak"),
    (13, "CookedDataStargates.pak"),
    (14, "CookedDataContainers.pak"),
    (15, "CookedBlueprints.pak"),
    (16, "CookedSciences.pak"),
    (17, "CookedDisciplines.pak"),
    (18, "CookedParadigm.pak"),
    (19, "SpecialWords.pak"),
    (20, "CookedInteractions.pak"),
    (21, "CookedBehaviorEvents.pak"),
];

/// A path's last component, whichever separator it uses.
pub(crate) fn leaf(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

/// The category whose PAK this is. The name may be a bare file name or a
/// path; the case is ignored (Windows file names).
pub(crate) fn category_id(pak: &str) -> Option<u32> {
    let name = leaf(pak);
    CATEGORY_PAKS
        .iter()
        .find(|(_, known)| known.eq_ignore_ascii_case(name))
        .map(|(id, _)| *id)
}

/// `warn` for a write that failed, `info` otherwise.
pub(crate) fn write_level(ok: bool) -> &'static str {
    if ok {
        "info"
    } else {
        "warn"
    }
}

/// Whether to emit this write. `last` is the version already reported for
/// elements of this storage.
pub(crate) fn should_report(
    kind: EntryKind,
    ok: bool,
    last: &mut Option<u32>,
    version: Option<u32>,
) -> bool {
    if !ok || kind != EntryKind::Element {
        return true;
    }
    if *last == version {
        return false;
    }
    *last = version;
    true
}

/// The fields of one `client.cooked_cache.write`.
pub(crate) fn write_fields(
    pak: Option<&str>,
    kind: EntryKind,
    tier: Tier,
    version: Option<u32>,
    ok: bool,
) -> Fields {
    vec![
        ("category_id", json!(pak.and_then(category_id))),
        ("version", json!(version)),
        ("tier", json!(tier.name())),
        ("result", json!(if ok { "ok" } else { "failed" })),
        ("pak", json!(pak.map(leaf))),
        ("entry_kind", json!(kind.name())),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(f: &Fields, key: &str) -> serde_json::Value {
        f.iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.clone())
            .unwrap_or(serde_json::Value::Null)
    }

    #[test]
    fn a_pushed_write_is_the_cache_tier() {
        let f = write_fields(
            Some("CookedDataItems.pak"),
            EntryKind::Element,
            Tier::Cache,
            Some(5802),
            true,
        );
        assert_eq!(get(&f, "category_id"), json!(4));
        assert_eq!(get(&f, "version"), json!(5802));
        assert_eq!(get(&f, "tier"), json!("cache"));
        assert_eq!(get(&f, "result"), json!("ok"));
        assert_eq!(get(&f, "entry_kind"), json!("element"));
    }

    #[test]
    fn a_bundled_copy_is_the_source_cache_tier_and_names_its_pak_from_the_path() {
        let path = r"C:\game\SGWGame\SourceCache.en-us\TEXTSTRINGS.PAK";
        let f = write_fields(
            Some(path),
            EntryKind::SourceArchive,
            Tier::SourceCache,
            None,
            false,
        );
        assert_eq!(get(&f, "category_id"), json!(10));
        assert_eq!(get(&f, "tier"), json!("source_cache"));
        assert_eq!(get(&f, "result"), json!("failed"));
        assert_eq!(get(&f, "version"), serde_json::Value::Null);
        assert_eq!(get(&f, "pak"), json!("TEXTSTRINGS.PAK"));
        assert_eq!(get(&f, "entry_kind"), json!("source_archive"));
    }

    #[test]
    fn every_category_resolves_and_an_unknown_pak_has_no_id() {
        for (id, pak) in CATEGORY_PAKS {
            assert_eq!(category_id(pak), Some(id), "{pak}");
        }
        assert_eq!(category_id("covernodes_local.pak"), None);
        let f = write_fields(None, EntryKind::MetaData, Tier::Cache, None, true);
        assert_eq!(get(&f, "category_id"), serde_json::Value::Null);
    }

    #[test]
    fn the_entry_name_decides_the_kind() {
        assert_eq!(entry_kind("MetaData"), EntryKind::MetaData);
        assert_eq!(entry_kind("_123456"), EntryKind::Element);
    }

    #[test]
    fn a_successful_element_is_reported_once_per_version() {
        let mut last = None;
        assert!(should_report(EntryKind::Element, true, &mut last, Some(7)));
        assert!(!should_report(EntryKind::Element, true, &mut last, Some(7)));
        assert!(!should_report(EntryKind::Element, true, &mut last, Some(7)));
        // A new version announces again.
        assert!(should_report(EntryKind::Element, true, &mut last, Some(8)));
        // Stamps and failures are never swallowed.
        assert!(should_report(EntryKind::MetaData, true, &mut last, Some(8)));
        assert!(should_report(EntryKind::Element, false, &mut last, Some(8)));
        assert_eq!(write_level(false), "warn");
        assert_eq!(write_level(true), "info");
    }
}
