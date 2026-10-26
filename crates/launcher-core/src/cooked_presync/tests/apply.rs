use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::path::Path;

use cimmeria_cooked_pak::catalog::{CategoryBody, IndexEntry};
use cimmeria_cooked_pak::decide::Action;
use cimmeria_cooked_pak::digest::category_digest;
use cimmeria_cooked_pak::pak::{entry_name, read_pak, read_version, write_pak, METADATA_ENTRY};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

use super::*;

const PAK: &str = "cat4.pak";

/// The old records of a made-up category, as the source and the old cache hold them.
fn base() -> BTreeMap<u32, Vec<u8>> {
    BTreeMap::from([
        (1, b"one".to_vec()),
        (2, b"two".to_vec()),
        (3, b"three".to_vec()),
    ])
}

/// The records after the served change: key 2 is replaced, the rest come from the base.
fn composed() -> BTreeMap<u32, Vec<u8>> {
    BTreeMap::from([
        (1, b"one".to_vec()),
        (2, b"TWO".to_vec()),
        (3, b"three".to_vec()),
    ])
}

/// The served body of category `category_id` at `version`, with key 2 as the only changed record.
fn body_of(category_id: u32, version: u32) -> CategoryBody {
    let composed = composed();
    CategoryBody {
        category_id,
        version,
        keys: composed.keys().copied().collect(),
        changed: BTreeMap::from([(2, b"TWO".to_vec())]),
        digest: category_digest(
            category_id,
            version,
            composed.iter().map(|(key, bytes)| (*key, bytes.as_slice())),
        ),
    }
}

fn index_entry(version: u32) -> IndexEntry {
    IndexEntry {
        category_id: 4,
        pak: PAK.to_owned(),
        version,
        entries: 3,
        changed: 1,
        body_bytes: 0,
        digest: String::new(),
    }
}

/// Writes a PAK the stock way, with `MetaData` stored.
fn write_source(dir: &Path, name: &str, version: u32, entries: &BTreeMap<u32, Vec<u8>>) {
    fs::create_dir_all(dir).unwrap();
    write_pak(File::create(dir.join(name)).unwrap(), version, entries).unwrap();
}

/// Writes a PAK whose `MetaData` is deflated, as the client rewrites it after a run (F21).
fn write_client_rewritten(path: &Path, version: u32, entries: &BTreeMap<u32, Vec<u8>>) {
    let mut zip = ZipWriter::new(Cursor::new(Vec::new()));
    for (key, bytes) in entries {
        zip.start_file(entry_name(*key), SimpleFileOptions::default())
            .unwrap();
        zip.write_all(bytes).unwrap();
    }
    let deflated = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    zip.start_file(METADATA_ENTRY, deflated).unwrap();
    zip.write_all(&version.to_le_bytes()).unwrap();
    fs::write(path, zip.finish().unwrap().into_inner()).unwrap();
}

/// The temp files left in a folder, by name.
fn temps(dir: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(TEMP_SUFFIX))
        .collect()
}

#[test]
fn stage_then_commit_gives_the_served_version() {
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), root.path().join("cache"));
    write_source(&source, PAK, 5, &base());
    write_source(&cache, PAK, 4, &base());
    let staged = stage(4, &body_of(4, 6), PAK, &cache, &source).unwrap();
    assert_eq!(staged.category_id, 4);
    assert_eq!(temps(&cache).len(), 1);
    assert_eq!(commit(vec![staged], &cache), Ok(1));
    assert_eq!(read_version(&cache.join(PAK)).unwrap(), Some(6));
    assert_eq!(read_pak(&cache.join(PAK)).unwrap().entries, composed());
    assert!(temps(&cache).is_empty());
}

#[test]
fn a_digest_mismatch_stages_nothing_and_leaves_the_old_pak() {
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), root.path().join("cache"));
    write_source(&source, PAK, 5, &base());
    write_source(&cache, PAK, 4, &base());
    let old = fs::read(cache.join(PAK)).unwrap();
    let mut body = body_of(4, 6);
    body.digest = [0; 32];
    let error = stage(4, &body, PAK, &cache, &source).unwrap_err();
    assert_eq!(error, ApplyError::DigestMismatch);
    assert_eq!(fs::read(cache.join(PAK)).unwrap(), old);
    assert!(temps(&cache).is_empty());
}

#[test]
fn a_dropped_stage_deletes_its_temp() {
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), root.path().join("cache"));
    write_source(&source, PAK, 5, &base());
    write_source(&cache, PAK, 4, &base());
    let staged = stage(4, &body_of(4, 6), PAK, &cache, &source).unwrap();
    assert_eq!(temps(&cache).len(), 1);
    drop(staged);
    assert!(temps(&cache).is_empty());
}

#[cfg(windows)]
#[test]
fn a_locked_second_destination_renames_nothing() {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), root.path().join("cache"));
    for name in ["cat4.pak", "cat7.pak"] {
        write_source(&source, name, 5, &base());
        write_source(&cache, name, 4, &base());
    }
    let old_four = fs::read(cache.join("cat4.pak")).unwrap();
    // Category 7 is staged first, so only the sorted commit can keep 4 unrenamed.
    let staged = vec![
        stage(7, &body_of(7, 6), "cat7.pak", &cache, &source).unwrap(),
        stage(4, &body_of(4, 6), "cat4.pak", &cache, &source).unwrap(),
    ];
    let _held = OpenOptions::new()
        .read(true)
        .write(true)
        .share_mode(0)
        .open(cache.join("cat7.pak"))
        .unwrap();
    let error = commit(staged, &cache).unwrap_err();
    assert_eq!(error.committed, 0);
    assert_eq!(error.category_id, 7);
    assert_eq!(error.error.reason(), "pak_in_use");
    assert_eq!(
        error.error.io().map(|detail| detail.kind),
        Some("sharing_violation")
    );
    assert_eq!(fs::read(cache.join("cat4.pak")).unwrap(), old_four);
    assert!(temps(&cache).is_empty());
}

#[test]
fn sweep_removes_only_presync_temps() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path();
    let stray = format!("cat4.pak{TEMP_SUFFIX}");
    fs::write(dir.join("cat4.pak"), b"keep").unwrap();
    fs::write(dir.join("cat4.pak.tmp"), b"keep").unwrap();
    fs::write(dir.join(&stray), b"go").unwrap();
    fs::create_dir(dir.join("sub")).unwrap();
    fs::write(dir.join("sub").join(&stray), b"keep").unwrap();
    assert_eq!(sweep_temps(dir), 1);
    assert!(dir.join("cat4.pak").exists());
    assert!(dir.join("cat4.pak.tmp").exists());
    assert!(dir.join("sub").join(&stray).exists());
    assert!(!dir.join(&stray).exists());
}

#[cfg(windows)]
#[test]
fn a_linked_cache_is_unsafe_path() {
    let root = tempfile::tempdir().unwrap();
    let firesky = root.path().join("firesky");
    fs::create_dir_all(firesky.join("SGWGame")).unwrap();
    assert_eq!(ensure_not_linked(&firesky), Ok(()));
    let elsewhere = root.path().join("elsewhere");
    fs::create_dir_all(&elsewhere).unwrap();
    if !make_link(&cache_dir(&firesky), &elsewhere) {
        // The OS refused both a symlink and a junction here, so there is nothing to test.
        return;
    }
    assert_eq!(ensure_not_linked(&firesky), Err(ApplyError::UnsafePath));
    // The `SGWGame` link is refused too, not only the cache folder.
    let firesky2 = root.path().join("firesky2");
    fs::create_dir_all(&firesky2).unwrap();
    let elsewhere2 = root.path().join("elsewhere2");
    fs::create_dir_all(&elsewhere2).unwrap();
    if make_link(&firesky2.join("SGWGame"), &elsewhere2) {
        assert_eq!(ensure_not_linked(&firesky2), Err(ApplyError::UnsafePath));
    }
}

/// Makes `link` point at `target`: a directory symlink if the OS allows it,
/// else a junction (which needs no privilege). False when neither works.
#[cfg(windows)]
fn make_link(link: &Path, target: &Path) -> bool {
    if std::os::windows::fs::symlink_dir(target, link).is_ok() {
        return true;
    }
    std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn a_missing_source_pak_is_source_missing() {
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), root.path().join("cache"));
    let error = stage(4, &body_of(4, 6), PAK, &cache, &source).unwrap_err();
    assert_eq!(error, ApplyError::SourceMissing);
}

#[test]
fn a_client_rewritten_cache_pak_is_up_to_date() {
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), root.path().join("cache"));
    write_source(&source, PAK, 5, &base());
    fs::create_dir_all(&cache).unwrap();
    write_client_rewritten(&cache.join(PAK), 6, &composed());
    let planned = plan(&index_entry(6), &cache, &source);
    assert_eq!(planned.action, Action::UpToDate);
    assert_eq!(planned.cache_version, Some(6));
    assert_eq!(planned.source_version, Some(5));
    assert!(temps(&cache).is_empty());
}

#[test]
fn an_unreadable_cache_pak_counts_as_none() {
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), root.path().join("cache"));
    write_source(&source, PAK, 5, &base());
    fs::create_dir_all(&cache).unwrap();
    fs::write(cache.join(PAK), b"not a zip").unwrap();
    let planned = plan(&index_entry(6), &cache, &source);
    assert_eq!(planned.cache_version, None);
    assert_eq!(planned.action, Action::Write);
}

#[test]
fn a_first_install_creates_the_cache_folder_and_the_pak() {
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), root.path().join("cache"));
    write_source(&source, PAK, 5, &base());
    assert!(!cache.exists());
    let staged = stage(4, &body_of(4, 6), PAK, &cache, &source).unwrap();
    assert_eq!(commit(vec![staged], &cache), Ok(1));
    assert_eq!(read_version(&cache.join(PAK)).unwrap(), Some(6));
    assert_eq!(read_pak(&cache.join(PAK)).unwrap().entries, composed());
    assert!(temps(&cache).is_empty());
}

#[test]
fn a_pak_name_that_leaves_the_folder_is_unsafe_path() {
    let root = tempfile::tempdir().unwrap();
    let (source, cache) = (root.path().join("source"), root.path().join("cache"));
    write_source(&source, PAK, 5, &base());
    for name in [
        "",
        "..",
        "..\\x.pak",
        "../x.pak",
        "sub/x.pak",
        "sub\\x.pak",
        "C:x.pak",
    ] {
        let error = stage(4, &body_of(4, 6), name, &cache, &source).unwrap_err();
        assert_eq!(error, ApplyError::UnsafePath, "{name}");
    }
    assert!(!cache.exists());
}
