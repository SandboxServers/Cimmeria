use super::*;

#[test]
fn rejects_unknown_fields_and_versions() {
    let root = tempfile::tempdir().unwrap();
    let state = DesktopState::open(root.path()).unwrap();
    assert_eq!(
        state
            .begin_log_upload(
                LogUploadCommand::Inspect { schema_version: 2 },
                &LogUploader::new()
            )
            .unwrap_err(),
        LogUploadError::Storage(StorageError::UnsupportedSchema)
    );
    assert!(serde_json::from_str::<LogUploadCommand>(
        r#"{"command":"upload","schema_version":1,"path":"/other"}"#
    )
    .is_err());
    assert!(serde_json::from_str::<LogUploadCommand>(
        r#"{"command":"upload","schema_version":1,"url":"https://elsewhere"}"#
    )
    .is_err());
}

#[test]
fn errors_and_states_serialize_as_flat_codes() {
    for (error, code) in [
        (LogUploadError::CollectFailed, "collect_failed"),
        (LogUploadError::Network, "network"),
        (LogUploadError::Storage(StorageError::Busy), "busy"),
        (
            LogUploadError::Storage(StorageError::UnsupportedSchema),
            "unsupported_schema",
        ),
    ] {
        assert_eq!(
            serde_json::to_value(error).unwrap(),
            serde_json::json!(code)
        );
    }
    assert_eq!(
        serde_json::to_value(LogUploadStatus::new(LogUploadState::NothingNew)).unwrap(),
        serde_json::json!({"schema_version": 1, "state": "nothing_new"})
    );
}
