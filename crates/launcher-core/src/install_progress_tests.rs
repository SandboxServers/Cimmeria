use super::hash::hash_file_reporting;
use super::seed::ArchiveCheck;
use super::{Progress, ProgressSink};

#[test]
fn archive_check_codes_are_the_two_closed_log_values() {
    assert_eq!(ArchiveCheck::Sha256("x").code(), "sha256");
    assert_eq!(
        ArchiveCheck::HashedWhileCopied.code(),
        "hashed_while_copied"
    );
}

#[test]
fn hash_file_reporting_reports_every_8_mib_and_at_the_end() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nine.bin");
    let size = 9 * 1024 * 1024u64;
    std::fs::write(&path, vec![5u8; size as usize]).unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let sink = ProgressSink::from(tx);
    hash_file_reporting(&path, Some(&sink)).unwrap();
    let mut checks = Vec::new();
    while let Ok(event) = rx.try_recv() {
        match event {
            Progress::Checking { read, total } => checks.push((read, total)),
            other => panic!("unexpected {other:?}"),
        }
    }
    assert_eq!(checks.len(), 2, "{checks:?}");
    assert_eq!(checks.last(), Some(&(size, size)));
}
