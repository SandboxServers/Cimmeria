//! A refused confirmation carries the check that failed (LT-33a).
use super::tests::{choices, Fixture};
use super::*;

#[tokio::test]
async fn a_refused_confirmation_carries_its_failed_check() {
    let f = Fixture::new();
    let preview = f.preview().unwrap();
    let handle = preview.report.preview_handle;
    std::fs::write(
        f.source.launcher_directory.join("launcher-config.json"),
        b"{}",
    )
    .unwrap();
    let worker = start_confirmation(preview, Uuid::new_v4(), handle, choices()).unwrap();
    let mut result = worker.result.clone();
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        result.wait_for(|r| r.is_some()),
    )
    .await
    .unwrap()
    .unwrap();
    {
        let outcome = worker.result.borrow();
        match outcome.as_ref() {
            Some(Err(e)) => assert_eq!(rejection::code(e), "source_changed"),
            _ => panic!("the confirmation should be refused"),
        }
    }
    let r = worker.last_rejection().expect("a recorded rejection");
    assert_eq!(r.check, "folder_changed_during_check");
    // The stop-check records one combined digest for the whole folder, not one
    // entry per file (LT-29's accepted deviation, verification.rs:119), so no
    // entry is named here.
    assert!(r.entry.is_none(), "the folder-level check names no entry");
    assert!(r.expected.is_some());
    assert!(r.found.is_some());
}
