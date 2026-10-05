use super::imports::*;
use super::*;
use crate::hooks::entity_trace::map::fake::FakeMem;

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
        ..ReadProbe::default()
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
        ..ReadProbe::default()
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
        ..ReadProbe::default()
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

/// The 2026-10-04 macOS failure, hop by hop: the archive's record and the
/// extraction are right, the stream holds the four bytes, and the stream
/// read returns none of them.
#[test]
fn a_stream_that_gives_nothing_back_is_named_and_shown_hop_by_hop() {
    let probe = ReadProbe {
        find_index: Some(29_126),
        extract_ok: Some(true),
        entry_ok: Some(true),
        zip: Some(ZipEntry {
            method: 8,
            crc32: 0x0bad_f00d,
            compressed: 6,
            uncompressed: 4,
        }),
        extracted: Some(Extracted {
            len: 4,
            head: Some(33_408),
        }),
        stream: Some(StreamRead {
            held: Some(4),
            requested: 4,
            count: 0,
            state: 3,
        }),
        crt: Some(CrtIo {
            reads: 2,
            read_bytes: 36,
            short_reads: 0,
            seeks: 1,
            failed_seeks: 0,
        }),
    };
    assert_eq!(read_outcome(probe), "stream_read_short");
    assert_eq!(read_level(probe), "warn", "the game believes it read 0");
    let f = version_read_fields(Some("WorldInfo.pak"), Some(0), Some(0), probe);
    assert_eq!(get(&f, "zip_method"), Some(json!("deflated")));
    assert_eq!(get(&f, "zip_method_id"), Some(json!(8)));
    assert_eq!(get(&f, "zip_compressed"), Some(json!(6)));
    assert_eq!(get(&f, "zip_uncompressed"), Some(json!(4)));
    assert_eq!(get(&f, "zip_crc32"), Some(json!("0badf00d")));
    assert_eq!(get(&f, "extract_ok"), Some(json!(true)));
    assert_eq!(get(&f, "extract_len"), Some(json!(4)));
    // 33408 = 0x8280, in file order.
    assert_eq!(get(&f, "extract_bytes"), Some(json!("80 82 00 00")));
    assert_eq!(get(&f, "extract_version"), Some(json!(33_408)));
    assert_eq!(get(&f, "stream_held"), Some(json!(4)));
    assert_eq!(get(&f, "stream_read_requested"), Some(json!(4)));
    assert_eq!(get(&f, "stream_read_count"), Some(json!(0)));
    assert_eq!(get(&f, "stream_state"), Some(json!("eof|fail")));
    assert_eq!(get(&f, "crt_reads"), Some(json!(2)));
    assert_eq!(get(&f, "crt_read_bytes"), Some(json!(36)));
    assert_eq!(get(&f, "crt_seeks"), Some(json!(1)));
    assert_eq!(get(&f, "version"), Some(json!(0)));

    // The same read with a stream that works.
    let good = ReadProbe {
        stream: Some(StreamRead {
            held: Some(4),
            requested: 4,
            count: 4,
            state: 0,
        }),
        ..probe
    };
    assert_eq!(read_outcome(good), "read");
    assert_eq!(read_level(good), "info");
    let f = version_read_fields(Some("WorldInfo.pak"), Some(0), Some(33_408), good);
    assert_eq!(get(&f, "stream_state"), Some(json!("good")));
    // Hooks that are not installed add no fields.
    let bare = version_read_fields(None, None, None, ReadProbe::default());
    for key in ["zip_method", "extract_len", "stream_state", "crt_reads"] {
        assert_eq!(get(&bare, key), None, "{key}");
    }
}

#[test]
fn stream_state_bits_are_named() {
    assert_eq!(state_name(0), "good");
    assert_eq!(state_name(1), "eof");
    assert_eq!(state_name(3), "eof|fail");
    assert_eq!(state_name(6), "fail|bad");
}

#[test]
fn the_memory_file_gives_its_length_and_first_four_bytes() {
    let mut mem = FakeMem::default();
    mem.set(0x1000 + layout::MEMORY_FILE_LENGTH, 4);
    mem.set(0x1000 + layout::MEMORY_FILE_BUFFER, 0x2000);
    mem.set(0x2000, 5802);
    assert_eq!(
        extracted(&mem, 0x1000),
        Some(Extracted {
            len: 4,
            head: Some(5802)
        })
    );
    // Too short to hold a version, or no buffer: the length alone.
    mem.set(0x1000 + layout::MEMORY_FILE_LENGTH, 3);
    assert_eq!(
        extracted(&mem, 0x1000),
        Some(Extracted { len: 3, head: None })
    );
    mem.set(0x1000 + layout::MEMORY_FILE_LENGTH, 0);
    mem.set(0x1000 + layout::MEMORY_FILE_BUFFER, 0);
    assert_eq!(
        extracted(&mem, 0x1000),
        Some(Extracted { len: 0, head: None })
    );
    assert_eq!(extracted(&mem, 0x9000), None, "an unreadable memory file");
}

#[test]
fn the_archive_gives_the_directory_record_of_an_entry() {
    let mut mem = FakeMem::default();
    // Three headers; the last is MetaData.
    mem.set(0x1000 + layout::ARCHIVE_HEADERS_BEGIN, 0x3000);
    mem.set(0x1000 + layout::ARCHIVE_HEADERS_END, 0x300c);
    mem.set(0x3008, 0x4000);
    // Flags 0 in the low half, method 8 in the high half.
    mem.set(0x4000 + 0x08, 8 << 16);
    mem.set(0x4000 + layout::HEADER_CRC32, 0x1234_5678);
    mem.set(0x4000 + layout::HEADER_COMPRESSED, 6);
    mem.set(0x4000 + layout::HEADER_UNCOMPRESSED, 4);
    assert_eq!(
        zip_entry(&mem, 0x1000, 2),
        Some(ZipEntry {
            method: 8,
            crc32: 0x1234_5678,
            compressed: 6,
            uncompressed: 4,
        })
    );
    assert_eq!(zip_entry(&mem, 0x1000, 3), None, "past the end");
    assert_eq!(
        zip_entry(&mem, 0x1000, 0),
        None,
        "an unreadable header slot"
    );
    assert_eq!(zip_entry(&mem, 0x7000, 0), None, "not an archive");
    assert_eq!(method_name(0), "stored");
    assert_eq!(method_name(8), "deflated");
    assert_eq!(method_name(12), "other");
}

/// A stream object at `stream`: its vbtable, `gcount`, the `basic_ios` at
/// `+0x60` (where `std::strstream` has it) and the buffer's put area.
fn stream_at(mem: &mut FakeMem, stream: u32, written: u32, count: u32, state: u32) {
    let (vbtable, ios, buffer, cells) = (0x8000, stream + 0x60, 0x9000, 0xa000);
    mem.set(stream, vbtable);
    mem.set(vbtable + 4, 0x60);
    mem.set(stream + layout::ISTREAM_COUNT, count);
    mem.set(ios + layout::IOS_STATE, state);
    mem.set(ios + layout::IOS_STREAMBUF, buffer);
    mem.set(buffer + layout::STREAMBUF_PUT_FIRST, cells);
    mem.set(buffer + layout::STREAMBUF_PUT_NEXT, cells + 4);
    mem.set(cells, 0x5000);
    mem.set(cells + 4, 0x5000 + written);
}

#[test]
fn a_stream_gives_what_it_holds_and_how_the_read_went() {
    let mut mem = FakeMem::default();
    stream_at(&mut mem, 0x1000, 4, 0, 3);
    assert_eq!(stream_held(&mem, 0x1000), Some(4));
    assert_eq!(
        stream_read(&mem, 0x1000, 4, Some(4)),
        Some(StreamRead {
            held: Some(4),
            requested: 4,
            count: 0,
            state: 3,
        })
    );
    // A buffer nothing was written to has no put area yet.
    mem.set(0xa000, 0);
    mem.set(0xa004, 0);
    assert_eq!(stream_held(&mem, 0x1000), Some(0));
    // An offset no stream object has: this is not a stream.
    mem.set(0x8004, 0x0010_0000);
    assert_eq!(stream_held(&mem, 0x1000), None);
    assert_eq!(stream_read(&mem, 0x1000, 4, None), None);
}

#[test]
fn a_c_runtime_open_is_described_in_the_words_of_a_win32_one() {
    // _O_RDWR | _O_BINARY, _SH_DENYWR: how the cache opens an archive it has.
    let f = crt_open_fields("C:\\c\\TextStrings.pak", 0x8002, 0x20, 0, Some(5), 0);
    assert_eq!(get(&f, "via"), Some(json!("crt")));
    assert_eq!(get(&f, "opened"), Some(json!(true)));
    assert_eq!(get(&f, "write"), Some(json!(true)));
    assert_eq!(get(&f, "disposition"), Some(json!("open_existing")));
    assert_eq!(get(&f, "share_mode"), Some(json!("deny_write")));
    assert_eq!(get(&f, "binary"), Some(json!(true)));
    assert_eq!(get(&f, "fd"), Some(json!(5)));
    assert_eq!(get(&f, "error"), None);
    assert_eq!(get(&f, "suppressed"), None);
    assert_eq!(crt_open_level(0), "info");

    // _O_RDONLY without _O_BINARY, and the file is not there.
    let f = crt_open_fields("covernodes_local.pak", 0, 0x40, 2, None, 3);
    assert_eq!(get(&f, "opened"), Some(json!(false)));
    assert_eq!(get(&f, "write"), Some(json!(false)));
    assert_eq!(get(&f, "binary"), Some(json!(false)));
    assert_eq!(get(&f, "share_mode"), Some(json!("deny_none")));
    assert_eq!(get(&f, "error"), Some(json!(2)));
    assert_eq!(get(&f, "error_name"), Some(json!("no_such_file")));
    assert_eq!(get(&f, "fd"), None);
    assert_eq!(get(&f, "suppressed"), Some(json!(3)));
    assert_eq!(crt_open_level(2), "warn");

    // _O_CREAT with and without _O_TRUNC and _O_EXCL.
    assert_eq!(crt_disposition(0x0102), "open_always");
    assert_eq!(crt_disposition(0x0302), "create_always");
    assert_eq!(crt_disposition(0x0502), "create_new");
    assert_eq!(crt_disposition(0x0202), "truncate_existing");
    assert_eq!(crt_share(0x10), "deny_read_write");
    assert_eq!(crt_share(0x30), "deny_read");
    assert_eq!(errno_name(13), "access_denied");
}

#[test]
fn file_calls_count_bytes_short_reads_and_failed_seeks() {
    let mut io = CrtIo::default();
    count_read(&mut io, 30, 30);
    count_read(&mut io, 64, 6);
    count_read(&mut io, 4, -1);
    count_seek(&mut io, 1024);
    count_seek(&mut io, -1);
    assert_eq!(
        io,
        CrtIo {
            reads: 3,
            read_bytes: 36,
            short_reads: 2,
            seeks: 2,
            failed_seeks: 1,
        }
    );
}
