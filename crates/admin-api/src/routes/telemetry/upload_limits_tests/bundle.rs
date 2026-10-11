//! The bundle budgets: the hard entry cap refuses; the entry, byte and
//! line budgets truncate, newest files first.

use std::io::Write;
use std::time::Instant;

use crate::routes::telemetry::bundle::bundle_inner;
use crate::routes::telemetry::bundle_unzip::{
    declared_zip_entries, newest_first, replayable_entry, unpack_and_replay, BundleCounts,
};
use crate::routes::telemetry::dto::IngestError;
use crate::routes::telemetry::replay_tests::capture;
use crate::routes::telemetry::upload_gate::{UploadLimits, UploadPolicy, UploadState};

use super::{bundle_request, claims, run, zip_of, Env, PEER};

fn many_entries(n: usize) -> Vec<u8> {
    let names: Vec<String> = (0..n)
        .map(|i| format!("Binaries/sessions/{i:05}.log"))
        .collect();
    let entries: Vec<(&str, &[u8])> = names.iter().map(|n| (n.as_str(), &b"x\n"[..])).collect();
    zip_of(&entries)
}

fn small_entry_limits() -> UploadLimits {
    UploadLimits {
        bundle_entries: 10,
        bundle_entries_hard: 100,
        ..UploadLimits::default()
    }
}

fn budget(counts: &BundleCounts) -> Option<&'static str> {
    counts.truncation.as_ref().map(|t| t.budget)
}

/// **A zip over the hard entry cap is refused from its end record**,
/// before the archive is opened. The handler's refusal names the end
/// record; without that check the archive would be opened (every entry's
/// header read) and refused from inside, with a different `what`.
#[test]
fn a_zip_over_the_hard_entry_cap_is_refused_before_it_is_opened() {
    let zip = many_entries(150);
    assert_eq!(declared_zip_entries(&zip), Some(150));
    let _env = Env::install();
    let state = UploadState::new(small_entry_limits());
    let err = run(bundle_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        bundle_request("sess-zip-hard", &zip),
        Instant::now(),
    ))
    .unwrap_err();
    assert!(
        matches!(
            err,
            IngestError::OverBudget {
                what: "zip entries (end record)",
                limit: 100
            }
        ),
        "{err:?}"
    );
}

/// **A zip of many small entries is truncated at the entry budget**: the
/// newest are replayed and the answer says it was truncated.
#[test]
fn a_zip_with_too_many_entries_replays_the_newest() {
    let zip = many_entries(50);
    let counts = unpack_and_replay(&claims("sess-zip-entries"), &zip, &small_entry_limits())
        .expect("over the entry budget is a truncation, not a refusal");
    assert_eq!((counts.files, counts.lines), (10, 10));
    assert_eq!(budget(&counts), Some("zip entries"));
    assert_eq!(counts.truncation.unwrap().dropped_estimate, 40);

    // Through the handler too: a 200 with `truncated`.
    let _env = Env::install();
    let state = UploadState::new(small_entry_limits());
    let resp = run(bundle_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        bundle_request("sess-zip-entries", &zip),
        Instant::now(),
    ))
    .unwrap();
    assert_eq!((resp.files, resp.truncated), (10, true));
}

/// Files are replayed newest first by their own timestamps, whatever
/// their order in the archive, so a budget cuts the oldest.
#[test]
fn files_are_replayed_newest_first() {
    let mut zw = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, day) in [("a.log", 20u8), ("b.log", 5), ("c.log", 12)] {
        let when = zip::DateTime::from_date_and_time(2026, 9, day, 12, 0, 0).unwrap();
        let options = zip::write::SimpleFileOptions::default().last_modified_time(when);
        zw.start_file(name, options).unwrap();
        zw.write_all(b"x\n").unwrap();
    }
    let bytes = zw.finish().unwrap().into_inner();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let order: Vec<usize> = newest_first(&mut zip)
        .unwrap()
        .into_iter()
        .map(|(_, i)| i)
        .collect();
    assert_eq!(order, vec![0, 2, 1]);
}

/// The expanded bytes of every entry together are capped: replay stops at
/// the first file that would pass the budget, and none of it is replayed.
#[test]
fn a_zip_over_the_expanded_byte_budget_is_truncated() {
    let data = [b'a'; 60];
    let zip = zip_of(&[("a.log", &data), ("b.log", &data), ("c.log", &data)]);
    let limits = UploadLimits {
        bundle_expanded_bytes: 100,
        ..UploadLimits::default()
    };
    let counts = unpack_and_replay(&claims("sess-zip-bytes"), &zip, &limits).unwrap();
    assert_eq!(counts.files, 1);
    assert_eq!(budget(&counts), Some("expanded bytes"));
}

/// Set every central-directory entry's uncompressed size to `size`, as a
/// zip that lies about its contents would.
fn lie_about_sizes(zip: &mut [u8], size: u32) {
    const CDH_SIG: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];
    let mut patched = 0;
    for pos in 0..zip.len().saturating_sub(46) {
        if zip[pos..pos + 4] == CDH_SIG {
            zip[pos + 24..pos + 28].copy_from_slice(&size.to_le_bytes());
            patched += 1;
        }
    }
    assert!(patched > 0, "no central-directory header found");
}

/// **A lying header does not get past the byte budget.** The entry claims
/// 1 byte but expands to 4,000: the declared-size check passes it, and the
/// bytes actually read stop it, so none of it is replayed.
#[test]
fn a_zip_that_lies_about_its_sizes_is_stopped_by_the_bytes_read() {
    let data = "line\n".repeat(800);
    let mut zip = zip_of(&[("a.log", data.as_bytes())]);
    lie_about_sizes(&mut zip, 1);
    let limits = UploadLimits {
        bundle_expanded_bytes: 1_000,
        ..UploadLimits::default()
    };
    let counts = unpack_and_replay(&claims("sess-zip-liar"), &zip, &limits).unwrap();
    assert_eq!((counts.files, counts.lines), (0, 0));
    assert_eq!(budget(&counts), Some("expanded bytes"));
}

/// **A lying entry stops the replay; it is not skipped.** Two entries
/// both declare small sizes: the newer one really expands to 4,000 bytes
/// against a 1,000-byte budget, the older one is honest and 10 bytes. The
/// older one is not replayed: skipping past a lying entry would let every
/// lying entry in a zip cost another full read of the budget.
#[test]
fn a_lying_entry_stops_the_replay_instead_of_being_skipped() {
    let mut zw = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, day, data) in [
        ("liar.log", 20u8, "line\n".repeat(800)),
        ("honest.log", 5, "x\n".repeat(5)),
    ] {
        let when = zip::DateTime::from_date_and_time(2026, 9, day, 12, 0, 0).unwrap();
        let options = zip::write::SimpleFileOptions::default().last_modified_time(when);
        zw.start_file(name, options).unwrap();
        zw.write_all(data.as_bytes()).unwrap();
    }
    let mut zip = zw.finish().unwrap().into_inner();
    // Only the first central-directory header (the liar's) lies.
    let cdh = zip
        .windows(4)
        .position(|w| w == [0x50, 0x4b, 0x01, 0x02])
        .unwrap();
    zip[cdh + 24..cdh + 28].copy_from_slice(&1u32.to_le_bytes());
    let limits = UploadLimits {
        bundle_expanded_bytes: 1_000,
        ..UploadLimits::default()
    };
    let counts = unpack_and_replay(&claims("sess-zip-liar-first"), &zip, &limits).unwrap();
    assert_eq!((counts.files, counts.lines), (0, 0));
    assert_eq!(budget(&counts), Some("expanded bytes"));
}

/// Replay stops at the line budget, and says how many lines it dropped.
#[test]
fn a_zip_over_the_line_budget_is_truncated() {
    let zip = zip_of(&[("a.log", b"1\n2\n3\n4\n5\n")]);
    let limits = UploadLimits {
        bundle_lines: 3,
        ..UploadLimits::default()
    };
    let counts = unpack_and_replay(&claims("sess-zip-lines"), &zip, &limits).unwrap();
    assert_eq!(counts.lines, 3);
    let t = counts.truncation.unwrap();
    assert_eq!((t.budget, t.kept, t.dropped_estimate), ("lines", 3, 2));
    let ok = unpack_and_replay(
        &claims("sess-zip-lines"),
        &zip_of(&[("a.log", b"1\n2\n\n3\n")]),
        &limits,
    )
    .unwrap();
    assert_eq!((ok.files, ok.lines, ok.truncation), (1, 3, None));
}

/// **A large entry of short lines stops at the line budget** and counts
/// what it dropped: 2,000,000 one-byte lines (4 MB) with a budget of
/// 1,000 replay 1,000 and report 1,999,000 dropped. The lines are walked
/// lazily, never collected: collecting a 64 MiB entry of `a\n` would
/// allocate about half a gigabyte of slices.
#[test]
fn a_large_entry_of_short_lines_stops_at_the_line_budget() {
    let data = "a\n".repeat(2_000_000);
    let zip = zip_of(&[("big.log", data.as_bytes())]);
    let limits = UploadLimits {
        bundle_lines: 1_000,
        ..UploadLimits::default()
    };
    let counts = unpack_and_replay(&claims("sess-zip-short-lines"), &zip, &limits).unwrap();
    assert_eq!(counts.lines, 1_000);
    let t = counts.truncation.unwrap();
    assert_eq!(
        (t.budget, t.kept, t.dropped_estimate),
        ("lines", 1_000, 1_999_000)
    );
}

/// A file over what is left of the byte budget is skipped, not the end of
/// the replay: a smaller, older file after it still fits and is replayed.
#[test]
fn a_file_over_the_byte_budget_is_skipped_and_smaller_ones_still_replay() {
    let mut zw = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, day, len) in [("new-big.log", 20u8, 500usize), ("old-small.log", 5, 10)] {
        let when = zip::DateTime::from_date_and_time(2026, 9, day, 12, 0, 0).unwrap();
        let options = zip::write::SimpleFileOptions::default().last_modified_time(when);
        zw.start_file(name, options).unwrap();
        zw.write_all("x\n".repeat(len / 2).as_bytes()).unwrap();
    }
    let zip = zw.finish().unwrap().into_inner();
    let limits = UploadLimits {
        bundle_expanded_bytes: 100,
        ..UploadLimits::default()
    };
    let counts = unpack_and_replay(&claims("sess-zip-skip"), &zip, &limits).unwrap();
    assert_eq!((counts.files, counts.lines), (1, 5));
    let t = counts.truncation.unwrap();
    assert_eq!((t.budget, t.dropped_estimate), ("expanded bytes", 500));
}

/// **Only logs are replayed.** Which entries count as logs: the client's
/// debug log and `*.log` files, whatever the folder or separator.
#[test]
fn replayable_entry_allows_only_logs() {
    for name in [
        "Binaries/SGWDebugLog.log",
        "sgwdebuglog",
        "Binaries/sessions/2026-10/session.log",
        "a\\b\\X.LOG",
    ] {
        assert!(replayable_entry(name), "{name}");
    }
    for name in [
        "Binaries/sessions/2026-10/abc-keys.txt",
        "Binaries/sessions/current-session.json",
        "notes.txt",
    ] {
        assert!(!replayable_entry(name), "{name}");
    }
}

/// **A key dump is never replayed.** The session folder holds a log and a
/// key dump: the log's line replays, the dump is counted in
/// `skipped_not_log` and its bytes reach no captured row.
#[test]
fn bundle_replay_skips_key_dumps() {
    let zip = zip_of(&[
        ("sessions/x/session.log", b"hello\n"),
        ("sessions/x/s-keys.txt", b"SECRETLINE\n"),
    ]);
    let mut counts = None;
    let rows = capture(|| {
        counts = Some(
            unpack_and_replay(&claims("sess-zip-keys"), &zip, &UploadLimits::default()).unwrap(),
        );
    });
    let counts = counts.unwrap();
    assert_eq!(
        (counts.files, counts.lines, counts.skipped_not_log),
        (1, 1, 1)
    );
    assert!(
        rows.iter()
            .all(|r| r.fields.values().all(|v| !v.contains("SECRETLINE"))),
        "{rows:#?}"
    );
    assert!(
        rows.iter().any(|r| r.fields.values().any(|v| v == "hello")),
        "{rows:#?}"
    );
}

/// A bundle within every budget is accepted through the handler.
#[test]
fn a_small_bundle_is_accepted() {
    let _env = Env::install();
    let state = UploadState::new(UploadLimits::default());
    let zip = zip_of(&[("Binaries/sgwdebuglog", b"hello\nworld\n")]);
    let resp = run(bundle_inner(
        &state,
        &UploadPolicy::defaults(),
        PEER,
        bundle_request("sess-bundle-ok", &zip),
        Instant::now(),
    ))
    .unwrap();
    assert_eq!((resp.files, resp.lines, resp.truncated), (1, 2, false));
}
