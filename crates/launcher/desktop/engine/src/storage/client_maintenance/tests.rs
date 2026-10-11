use super::*;

#[test]
fn scaffold_is_unavailable_and_rejects_unknown_fields_and_versions() {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(root.path()).unwrap();
    for request in [
        ClientMaintenanceCommand::Inspect { schema_version: 1 },
        ClientMaintenanceCommand::ResetCache { schema_version: 1 },
        ClientMaintenanceCommand::ResetAllClientState {
            schema_version: 1,
            confirmed: true,
        },
    ] {
        assert_eq!(
            state.client_maintenance(request),
            Err(ClientMaintenanceError::Unavailable)
        );
    }
    assert_eq!(
        state.client_maintenance(ClientMaintenanceCommand::ResetCache { schema_version: 2 }),
        Err(ClientMaintenanceError::UnsupportedSchema)
    );
    assert!(serde_json::from_str::<ClientMaintenanceCommand>(
        r#"{"command":"reset_all_client_state","schema_version":1,"confirmed":true,"path":"/other"}"#
    )
    .is_err());
    // A reset with no confirmation field is not a confirmed reset.
    assert!(serde_json::from_str::<ClientMaintenanceCommand>(
        r#"{"command":"reset_all_client_state","schema_version":1}"#
    )
    .is_err());
}
