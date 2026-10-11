use super::*;

#[test]
fn scaffold_is_unavailable_and_rejects_unknown_fields_and_versions() {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    for request in [
        LogUploadCommand::Inspect { schema_version: 1 },
        LogUploadCommand::Upload { schema_version: 1 },
    ] {
        assert_eq!(state.log_upload(request), Err(LogUploadError::Unavailable));
    }
    assert_eq!(
        state.log_upload(LogUploadCommand::Inspect { schema_version: 2 }),
        Err(LogUploadError::Storage(StorageError::UnsupportedSchema))
    );
    assert!(serde_json::from_str::<LogUploadCommand>(
        r#"{"command":"upload","schema_version":1,"path":"/other"}"#
    )
    .is_err());
}

#[test]
fn errors_serialize_as_the_flat_codes_the_bridge_accepts() {
    for (error, code) in [
        (LogUploadError::Unavailable, "unavailable"),
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
}
