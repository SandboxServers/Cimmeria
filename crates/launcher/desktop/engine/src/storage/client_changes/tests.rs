use super::*;

#[test]
fn scaffold_is_unavailable_and_rejects_unknown_fields_and_versions() {
    let root = tempfile::tempdir().unwrap();
    let state = DesktopState::open(root.path()).unwrap();
    assert_eq!(
        state.client_changes(ClientChangesCommand::Inspect { schema_version: 1 }),
        Err(ClientChangesError::Unavailable)
    );
    assert_eq!(
        state.client_changes(ClientChangesCommand::Inspect { schema_version: 2 }),
        Err(ClientChangesError::UnsupportedSchema)
    );
    assert!(serde_json::from_str::<ClientChangesCommand>(
        r#"{"command":"inspect","schema_version":1,"path":"/other"}"#
    )
    .is_err());
}
