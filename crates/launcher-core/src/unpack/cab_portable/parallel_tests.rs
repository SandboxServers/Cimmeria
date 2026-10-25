//! Parallel folder expansion: same bytes at any worker count, reports from
//! several threads, the same first error, and cancellation. Each test names
//! the line whose revert it fails.

use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::ThreadId;

use tokio_util::sync::CancellationToken;

use super::super::super::install_progress::{ProgressReporter, ProgressSink};
use super::super::test_fixtures::sink;
use super::super::UnpackSink;
use super::fixture::{write_cab_set, CabOptions, CabSetInfo};
use super::format;
use super::schedule::assign;
use super::{expand_chain_with, UnpackError};

/// Text-like bytes with long repeats, so MSZIP history reaches across blocks.
fn text(seed: u8, len: usize) -> Vec<u8> {
    format!("[Engine.Core{seed}] Value=0x{seed:02X} ; the same line again\r\n")
        .bytes()
        .cycle()
        .take(len)
        .collect()
}

/// Six files, 1 to 3 blocks each. With one folder per file and four blocks
/// per cabinet, the fourth folder is split across the second and third
/// cabinets: one folder continues across a boundary.
fn six_files() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("f0.txt", text(10, 30_000)),
        (
            "f1.bin",
            crate::unpack::test_fixtures::incompressible(11, 70_000),
        ),
        ("f2.txt", text(12, 20_000)),
        ("f3.txt", text(13, 50_000)),
        ("f4.txt", text(14, 40_000)),
        ("f5.txt", text(15, 10_000)),
    ]
}

fn write_six(dir: &Path, opts: CabOptions) -> (Vec<(&'static str, Vec<u8>)>, CabSetInfo) {
    let files = six_files();
    let info = write_cab_set(dir, &files, opts);
    (files, info)
}

/// Reporter that records which thread each progress event came from.
#[derive(Default)]
struct ThreadLog(Mutex<Vec<ThreadId>>);

impl ProgressReporter for ThreadLog {
    fn report(&self, _value: crate::install::Progress) {
        let mut ids = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        ids.push(std::thread::current().id());
    }
}

fn logged_sink(log: &Arc<ThreadLog>) -> UnpackSink {
    UnpackSink {
        progress: ProgressSink::Checkpoint(log.clone()),
        label: "test".into(),
        cancel: CancellationToken::new(),
    }
}

fn spans_a_boundary(dir: &Path, names: &[String]) -> bool {
    names.iter().any(|name| {
        let file = std::fs::File::open(dir.join(name)).unwrap();
        format::parse(&file)
            .unwrap()
            .files
            .iter()
            .any(|f| f.continues_to_next())
    })
}

// Guards `expand_chain_with`'s worker threads: revert the parallel run to a
// single worker and the bytes must still match, so this checks contents and
// the file listing, not the thread count (that is the next test's job).
#[test]
fn every_worker_count_writes_the_same_files() {
    let dir = tempfile::tempdir().unwrap();
    let (files, info) = write_six(
        dir.path(),
        CabOptions {
            files_per_folder: 1,
            blocks_per_cab: 4,
            ..CabOptions::default()
        },
    );
    assert_eq!(info.cabinets.len(), 3, "{:?}", info.cabinets);
    assert!(spans_a_boundary(dir.path(), &info.cabinets));
    for workers in [1, 4] {
        let dest = dir.path().join(format!("out-{workers}"));
        let (sink, _rx) = sink();
        expand_chain_with(
            dir.path(),
            &info.cabinets,
            &dest,
            files.len(),
            &sink,
            workers,
        )
        .unwrap();
        for (name, data) in &files {
            let written = std::fs::read(dest.join(name)).unwrap();
            assert_eq!(&written, data, "{name} with {workers} workers");
        }
        assert_eq!(std::fs::read_dir(&dest).unwrap().count(), files.len());
    }
}

// Guards the thread spawn in `schedule::run`: revert it to a sequential loop
// and every report comes from one thread, so this fails.
#[test]
fn folders_report_from_several_threads() {
    let dir = tempfile::tempdir().unwrap();
    let (files, info) = write_six(
        dir.path(),
        CabOptions {
            files_per_folder: 1,
            blocks_per_cab: 4,
            ..CabOptions::default()
        },
    );
    let log = Arc::new(ThreadLog::default());
    let dest = dir.path().join("out");
    expand_chain_with(
        dir.path(),
        &info.cabinets,
        &dest,
        files.len(),
        &logged_sink(&log),
        4,
    )
    .unwrap();
    let ids = log.0.lock().unwrap_or_else(PoisonError::into_inner);
    assert_eq!(ids.len(), files.len(), "one report per file");
    let threads: HashSet<&ThreadId> = ids.iter().collect();
    assert!(
        threads.len() > 1,
        "reports came from {} thread",
        threads.len()
    );
}

// Guards the error path in `schedule::Stop::fail`: a corrupt folder must
// give the same error on one worker and on four.
#[test]
fn a_corrupt_folder_fails_the_same_way_on_any_worker_count() {
    let dir = tempfile::tempdir().unwrap();
    let (files, info) = write_six(
        dir.path(),
        CabOptions {
            files_per_folder: 1,
            blocks_per_cab: 1000,
            ..CabOptions::default()
        },
    );
    assert_eq!(info.cabinets.len(), 1);
    // Break the CK signature of the first block of folder 3 (index 2).
    let cab = dir.path().join(&info.cabinets[0]);
    let parsed = format::parse(&std::fs::File::open(&cab).unwrap()).unwrap();
    let block = usize::try_from(parsed.folders[2].data_offset).unwrap() + 8;
    let mut bytes = std::fs::read(&cab).unwrap();
    assert_eq!(&bytes[block..block + 2], b"CK");
    bytes[block..block + 2].copy_from_slice(b"XX");
    std::fs::write(&cab, &bytes).unwrap();

    let mut messages = Vec::new();
    for workers in [1, 4] {
        let dest = dir.path().join(format!("out-{workers}"));
        let (sink, _rx) = sink();
        let err = expand_chain_with(
            dir.path(),
            &info.cabinets,
            &dest,
            files.len(),
            &sink,
            workers,
        )
        .unwrap_err();
        assert!(matches!(err, UnpackError::Cab(_)), "{err}");
        assert!(err.to_string().contains("CK signature"), "{err}");
        messages.push(err.to_string());
    }
    assert_eq!(messages[0], messages[1]);
}

// Guards the cancel check before any output: a cancelled token must stop the
// run before a file is created.
#[test]
fn a_cancelled_token_stops_before_any_file() {
    let dir = tempfile::tempdir().unwrap();
    let (files, info) = write_six(dir.path(), CabOptions::default());
    let (sink, _rx) = sink();
    sink.cancel.cancel();
    let dest = dir.path().join("out");
    let result = expand_chain_with(dir.path(), &info.cabinets, &dest, files.len(), &sink, 4);
    assert!(matches!(result, Err(UnpackError::Cancelled)));
    assert!(!dest.exists());
}

// Guards the assignment order: revert it to plan order or to round-robin and
// these lanes change.
#[test]
fn folders_go_largest_first_to_the_least_loaded_lane() {
    assert_eq!(assign(&[5, 3, 3, 2], 2), vec![vec![0, 3], vec![1, 2]]);
    // Ties keep plan order, and go to the lower lane.
    assert_eq!(assign(&[4, 4, 4], 2), vec![vec![0, 2], vec![1]]);
}
