use super::*;

fn get(f: &Fields, key: &str) -> Option<serde_json::Value> {
    f.iter().find(|(k, _)| *k == key).map(|(_, v)| v.clone())
}

#[test]
fn a_version_is_zero_real_or_the_servers_resync_placeholder() {
    assert_eq!(version_kind(0), "zero");
    assert_eq!(version_kind(5802), "real");
    assert_eq!(version_kind(0x7fff_ffff), "real");
    // The server stamps `!version` while it pushes: `!5802`.
    assert_eq!(version_kind(!5802), "resync_pending");
    assert_eq!(version_kind(4_294_961_493), "resync_pending");
}

#[test]
fn the_outcome_names_the_step_that_failed() {
    let probe = |find_index, extract_ok, entry_ok| ReadProbe {
        find_index,
        extract_ok,
        entry_ok,
    };
    assert_eq!(read_outcome(probe(Some(4), Some(true), Some(true))), "read");
    assert_eq!(
        read_outcome(probe(Some(NOT_FOUND), None, Some(false))),
        "metadata_entry_not_found"
    );
    assert_eq!(
        read_outcome(probe(Some(4), Some(false), Some(false))),
        "metadata_extract_failed"
    );
    assert_eq!(
        read_outcome(probe(Some(4), Some(true), Some(false))),
        "metadata_empty"
    );
    // The entry read never ran, or none of the hooks under it is in.
    assert_eq!(
        read_outcome(ReadProbe::default()),
        "entry_read_not_attempted"
    );
    // Found, then an exception out of the extraction.
    assert_eq!(
        read_outcome(probe(Some(4), None, None)),
        "entry_read_failed"
    );
    // The library hooks are off; the entry read still reports itself.
    assert_eq!(read_outcome(probe(None, None, Some(true))), "read");
    assert_eq!(
        read_outcome(probe(None, None, Some(false))),
        "entry_read_failed"
    );
}

#[test]
fn a_failed_read_is_a_warning_that_keeps_the_stale_version_visible() {
    let failed = ReadProbe {
        find_index: Some(NOT_FOUND),
        extract_ok: None,
        entry_ok: Some(false),
    };
    assert_eq!(read_level(failed), "warn");
    let f = version_read_fields(Some("TextStrings.pak"), Some(0), Some(0), failed);
    assert_eq!(get(&f, "pak"), Some(json!("TextStrings.pak")));
    assert_eq!(get(&f, "outcome"), Some(json!("metadata_entry_not_found")));
    assert_eq!(get(&f, "version"), Some(json!(0)));
    assert_eq!(get(&f, "version_kind"), Some(json!("zero")));
    assert_eq!(get(&f, "previous"), None, "nothing changed");
    assert_eq!(get(&f, "entry_index"), None);
}

#[test]
fn a_good_read_reports_the_version_and_where_the_entry_was() {
    let read = ReadProbe {
        find_index: Some(29_126),
        extract_ok: Some(true),
        entry_ok: Some(true),
    };
    assert_eq!(read_level(read), "info");
    let f = version_read_fields(Some("TextStrings.pak"), Some(0), Some(5802), read);
    assert_eq!(get(&f, "outcome"), Some(json!("read")));
    assert_eq!(get(&f, "version"), Some(json!(5802)));
    assert_eq!(get(&f, "version_kind"), Some(json!("real")));
    assert_eq!(get(&f, "previous"), Some(json!(0)));
    assert_eq!(get(&f, "entry_index"), Some(json!(29_126)));
    // An unreadable `out` pointer is reported as null, not as zero.
    let f = version_read_fields(None, None, None, read);
    assert_eq!(get(&f, "version"), Some(serde_json::Value::Null));
    assert_eq!(get(&f, "pak"), Some(serde_json::Value::Null));
}

#[test]
fn a_stamp_carries_the_old_and_new_version() {
    let f = version_set_fields(Some("CookedDataItems.pak"), Some(0), !44_303);
    assert_eq!(get(&f, "version"), Some(json!(!44_303u32)));
    assert_eq!(get(&f, "version_kind"), Some(json!("resync_pending")));
    assert_eq!(get(&f, "previous"), Some(json!(0)));
}

#[test]
fn the_login_snapshot_counts_what_the_client_holds() {
    let held = vec![
        ("TextStrings.pak".to_string(), Some(0)),
        ("CookedDataItems.pak".to_string(), Some(44_303)),
        ("CookedSciences.pak".to_string(), Some(!2202)),
        ("0x0badf00d".to_string(), None),
    ];
    let f = versions_held_fields(&held);
    assert_eq!(get(&f, "storages"), Some(json!(4)));
    assert_eq!(get(&f, "zero"), Some(json!(1)));
    assert_eq!(get(&f, "real"), Some(json!(1)));
    assert_eq!(get(&f, "resync_pending"), Some(json!(1)));
    let versions = get(&f, "versions").unwrap();
    assert_eq!(versions["TextStrings.pak"], json!(0));
    assert_eq!(versions["CookedDataItems.pak"], json!(44_303));
    assert_eq!(versions["0x0badf00d"], serde_json::Value::Null);
}
