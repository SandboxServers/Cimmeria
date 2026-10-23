use super::*;
fn entries(bytes: &'static [u8]) -> Vec<Entry> {
    let digest = Box::leak(hex(Sha256::digest(bytes)).into_boxed_str());
    NAMES
        .iter()
        .map(|name| Entry {
            name,
            sha256: digest,
            bytes,
        })
        .collect()
}
fn cache(root: &tempfile::TempDir) -> PathBuf {
    root.path().canonicalize().unwrap().join("payloads")
}
#[test]
fn cold_start_and_reuse_need_only_embedded_bytes_not_nearby_sidecars() {
    let root = tempfile::tempdir().unwrap();
    let cache = cache(&root);
    let entries = entries(b"compiled payload");
    let first = prepare(&cache, &entries).unwrap();
    let path = first.root().join("windows").join(NAMES[0]);
    let modified = fs::metadata(&path).unwrap().modified().unwrap();
    // Reuse while another live launcher has this same payload leased.
    let second = prepare(&cache, &entries).unwrap();
    assert_eq!(first.root(), second.root());
    assert_eq!(fs::read(path).unwrap(), b"compiled payload");
    assert_eq!(
        fs::metadata(second.root().join("windows").join(NAMES[0]))
            .unwrap()
            .modified()
            .unwrap(),
        modified
    );
}
#[test]
fn interrupted_or_corrupt_extraction_is_repaired_from_embedded_bytes() {
    let root = tempfile::tempdir().unwrap();
    let cache = cache(&root);
    let entries = entries(b"trusted");
    let lease = prepare(&cache, &entries).unwrap();
    let dir = lease.root().join("windows");
    drop(lease);
    fs::write(dir.join(NAMES[0]), b"corrupt").unwrap();
    fs::remove_file(dir.join(NAMES[1])).unwrap();
    let lease = prepare(&cache, &entries).unwrap();
    for name in NAMES {
        assert_eq!(
            fs::read(lease.root().join("windows").join(name)).unwrap(),
            b"trusted"
        );
    }
}
#[test]
fn repair_never_replaces_bytes_leased_by_another_launcher() {
    let root = tempfile::tempdir().unwrap();
    let cache = cache(&root);
    let entries = entries(b"trusted");
    let lease = prepare(&cache, &entries).unwrap();
    let path = lease.root().join("windows").join(NAMES[1]);
    fs::write(&path, b"corrupted").unwrap();
    assert!(prepare(&cache, &entries).is_err());
    assert_eq!(fs::read(path).unwrap(), b"corrupted");
}
#[test]
fn new_versions_keep_active_old_versions_and_sweep_only_after_their_lease_ends() {
    let root = tempfile::tempdir().unwrap();
    let cache = cache(&root);
    let old = prepare(&cache, &entries(b"old")).unwrap();
    let old_path = old.root().to_owned();
    let new = prepare(&cache, &entries(b"new")).unwrap();
    assert_ne!(old.root(), new.root());
    assert!(old_path.exists());
    for name in NAMES {
        assert_eq!(
            fs::read(old_path.join("windows").join(name)).unwrap(),
            b"old"
        );
    }
    drop(old);
    let again = prepare(&cache, &entries(b"new")).unwrap();
    assert_eq!(again.root(), new.root());
    assert!(!old_path.exists());
}
#[test]
fn unknown_content_and_unowned_digest_shaped_folders_are_preserved() {
    let root = tempfile::tempdir().unwrap();
    let cache = cache(&root);
    let old = prepare(&cache, &entries(b"old")).unwrap();
    let old_path = old.root().to_owned();
    fs::write(old.root().join("windows/player-file.txt"), b"preserve").unwrap();
    drop(old);
    let unknown = cache.join("a".repeat(64));
    fs::create_dir(&unknown).unwrap();
    fs::write(unknown.join("personal.txt"), b"preserve").unwrap();
    let _new = prepare(&cache, &entries(b"new")).unwrap();
    assert_eq!(
        fs::read(old_path.join("windows").join(NAMES[0])).unwrap(),
        b"old"
    );
    assert_eq!(fs::read(unknown.join("personal.txt")).unwrap(), b"preserve");
}
#[test]
fn mismatched_bytes_duplicate_names_and_paths_are_refused_before_creating_cache() {
    let root = tempfile::tempdir().unwrap();
    let cache = cache(&root);
    for entries in [
        vec![Entry {
            name: NAMES[0],
            sha256: "0".repeat(64).leak(),
            bytes: b"wrong",
        }],
        vec![Entry {
            name: "../outside.exe",
            sha256: Box::leak(hex(Sha256::digest(b"x")).into_boxed_str()),
            bytes: b"x",
        }],
        vec![entries(b"x").remove(0), entries(b"x").remove(0)],
    ] {
        assert!(prepare(&cache, &entries).is_err());
        assert!(!cache.exists());
    }
}
#[test]
fn unexpected_file_types_and_owner_records_are_not_overwritten() {
    let root = tempfile::tempdir().unwrap();
    let cache = cache(&root);
    let entries = entries(b"trusted");
    let lease = prepare(&cache, &entries).unwrap();
    let dir = lease.root().to_owned();
    drop(lease);
    fs::write(dir.join(".owner"), b"not ours").unwrap();
    fs::write(dir.join("windows").join(NAMES[0]), b"leave unchanged").unwrap();
    assert!(prepare(&cache, &entries).is_err());
    assert_eq!(fs::read(dir.join(".owner")).unwrap(), b"not ours");
    assert_eq!(
        fs::read(dir.join("windows").join(NAMES[0])).unwrap(),
        b"leave unchanged"
    );
    fs::write(dir.join(".owner"), OWNER).unwrap();
    fs::remove_file(dir.join("windows").join(NAMES[1])).unwrap();
    fs::create_dir(dir.join("windows").join(NAMES[1])).unwrap();
    assert!(prepare(&cache, &entries).is_err());
}

#[cfg(windows)]
#[test]
fn a_payload_file_still_used_after_launcher_exit_defers_cleanup_until_a_later_start() {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_SHARE_READ;
    let root = tempfile::tempdir().unwrap();
    let cache = cache(&root);
    let old = prepare(&cache, &entries(b"old")).unwrap();
    let old_root = old.root().to_owned();
    let file_path = old_root.join("windows").join(NAMES[1]);
    let in_use = File::options()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(&file_path)
        .unwrap();
    drop(old);
    let _new = prepare(&cache, &entries(b"new")).unwrap();
    assert_eq!(fs::read(&file_path).unwrap(), b"old");
    assert!(old_root.exists());
    drop(in_use);
    let _retry = prepare(&cache, &entries(b"new")).unwrap();
    assert!(!old_root.exists());
}
#[test]
fn failure_logging_uses_closed_codes_without_paths_or_error_strings() {
    let root = tempfile::tempdir().unwrap();
    let (result, events) = crate::test_capture::capture(|| prepare(&cache(&root), &[]));
    assert!(result.is_err());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].field("event"), Some("payload_cache_refused"));
    assert_eq!(events[0].field("reason"), Some("prepare_failed"));
}
#[cfg(unix)]
#[test]
fn symlinked_cache_ancestors_payloads_and_cleanup_versions_are_never_followed() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let cache = cache(&root);
    symlink(outside.path(), &cache).unwrap();
    assert!(prepare(&cache, &entries(b"trusted")).is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    fs::remove_file(&cache).unwrap();
    let lease = prepare(&cache, &entries(b"trusted")).unwrap();
    let path = lease.root().join("windows").join(NAMES[0]);
    drop(lease);
    fs::remove_file(&path).unwrap();
    let target = outside.path().join("file");
    fs::write(&target, b"private").unwrap();
    symlink(&target, &path).unwrap();
    assert!(prepare(&cache, &entries(b"trusted")).is_err());
    symlink(outside.path(), cache.join("a".repeat(64))).unwrap();
    let _new = prepare(&cache, &entries(b"new")).unwrap();
    assert_eq!(fs::read(target).unwrap(), b"private");
}
#[cfg(windows)]
#[test]
fn junctions_refuse_writes_and_directory_handles_prevent_ancestor_replacement() {
    use base64::Engine;
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let cache = cache(&root);
    let quote = |p: &Path| format!("'{}'", p.display().to_string().replace('\'', "''"));
    let command = format!(
        "New-Item -ItemType Junction -Path {} -Target {} | Out-Null",
        quote(&cache),
        quote(outside.path())
    );
    let bytes: Vec<u8> = command.encode_utf16().flat_map(u16::to_le_bytes).collect();
    assert!(std::process::Command::new("pwsh")
        .args([
            "-NoProfile",
            "-EncodedCommand",
            &base64::engine::general_purpose::STANDARD.encode(bytes)
        ])
        .status()
        .unwrap()
        .success());
    assert!(prepare(&cache, &entries(b"trusted")).is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
    fs::remove_dir(&cache).unwrap();
    let lease = prepare(&cache, &entries(b"trusted")).unwrap();
    assert!(fs::rename(&cache, root.path().join("replaced")).is_err());
    drop(lease);
    fs::rename(cache, root.path().join("replaced")).unwrap();
}
