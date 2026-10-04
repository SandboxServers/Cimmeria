use super::*;
use std::io::{Cursor, Write};

fn fixture() -> (tempfile::TempDir, ExtractRequest) {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("seed with spaces.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).unwrap());
    zip.start_file(
        "Working/Binaries/payload.txt",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    zip.write_all(b"payload").unwrap();
    zip.finish().unwrap();
    let hash = Sha256::digest(std::fs::read(&archive).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let request = ExtractRequest {
        schema_version: 1,
        operation_id: Uuid::new_v4(),
        archive,
        destination: root.path().join("output ü with spaces"),
        sha256: hash,
    };
    (root, request)
}
#[test]
fn bounded_frames_preserve_next_message_and_reject_truncation() {
    let mut input = Cursor::new(b"one\ntwo\n");
    assert_eq!(read_frame(&mut input).unwrap().unwrap(), b"one\n");
    assert_eq!(read_frame(&mut input).unwrap().unwrap(), b"two\n");
    assert_eq!(read_frame(&mut input).unwrap(), None);
    for input in [vec![b'x'; MAX_FRAME + 1], b"truncated".to_vec()] {
        assert_eq!(
            read_frame(&mut Cursor::new(input)),
            Err(ExtractError::InvalidRequest)
        );
    }
}
#[test]
fn cancel_only_matches_the_current_operation() {
    let id = Uuid::new_v4();
    let frame = serde_json::json!({"schema_version":1,"operation_id":id,"cancel":true});
    let bytes = serde_json::to_vec(&frame).unwrap();
    assert!(cancellation_matches(&bytes, id));
    assert!(!cancellation_matches(&bytes, Uuid::new_v4()));
    assert!(!cancellation_matches(br#"{"cancel":true}"#, id));
}
#[test]
fn extracts_to_new_output_and_refuses_to_overwrite_a_retry() {
    let (_root, request) = fixture();
    let (progress, receiver) = ProgressSink::latest();
    extract(&request, CancellationToken::new(), progress.clone()).unwrap();
    assert_eq!(
        std::fs::read(request.destination.join("Working/Binaries/payload.txt")).unwrap(),
        b"payload"
    );
    assert!(receiver.borrow().is_some());
    assert_eq!(
        extract(&request, CancellationToken::new(), progress),
        Err(ExtractError::DestinationExists)
    );
}
#[test]
fn unauthenticated_or_cancelled_input_never_creates_output() {
    let (_root, mut request) = fixture();
    let (progress, _) = ProgressSink::latest();
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        extract(&request, cancel, progress.clone()),
        Err(ExtractError::Cancelled)
    );
    request.sha256 = "00".repeat(32);
    assert_eq!(
        extract(&request, CancellationToken::new(), progress),
        Err(ExtractError::HashMismatch)
    );
    assert!(!request.destination.exists());
}

#[cfg(windows)]
#[test]
fn verification_handle_denies_mutation_and_replacement_until_extraction_finishes() {
    let (_root, request) = fixture();
    let guard = open_archive(&request.archive).unwrap();
    assert!(std::fs::OpenOptions::new()
        .write(true)
        .open(&request.archive)
        .is_err());
    assert!(std::fs::rename(&request.archive, request.archive.with_extension("replaced")).is_err());
    // A library can still reopen the same verified file for extraction.
    assert!(std::fs::File::open(&request.archive).is_ok());
    drop(guard);
    assert!(std::fs::OpenOptions::new()
        .write(true)
        .open(&request.archive)
        .is_ok());
}

#[test]
fn terminal_error_field_is_required_and_unknown_fields_are_rejected() {
    let id = Uuid::new_v4();
    let base = serde_json::json!({"schema_version":1,"operation_id":id,"event":"finished"});
    assert!(serde_json::from_value::<WorkerEvent>(base.clone()).is_err());
    let mut valid = base;
    valid["error"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<WorkerEvent>(valid.clone()).is_ok());
    valid["unexpected"] = serde_json::json!(true);
    assert!(serde_json::from_value::<WorkerEvent>(valid).is_err());
}
