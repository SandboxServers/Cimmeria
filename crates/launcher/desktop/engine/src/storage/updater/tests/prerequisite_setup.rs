//! A vendor installer owns mutation admission even when a launcher update is ready.
use super::*;
#[test]
fn component_setup_blocks_update_check_prepare_and_apply_without_changing_the_offer() {
    for ready in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let (config, offer) = fixture("1.1.0");
        let mut state = DesktopState::open(root.path()).unwrap();
        let snapshot = available(&mut state, &config, offer);
        let snapshot = if ready {
            prepare(&mut state, &config, snapshot, PAYLOAD)
        } else {
            snapshot
        };
        let offer = snapshot.offer.as_ref().unwrap().id;
        let before = serde_json::to_value(&snapshot).unwrap();
        state.begin_prerequisite_setup().unwrap();
        assert!(matches!(
            state.begin_launcher_update_check(
                Some(&config),
                snapshot.revision,
                snapshot.operation_revision
            ),
            Err(Error::Busy)
        ));
        assert!(matches!(
            state.begin_launcher_update_prepare(
                Some(&config),
                offer,
                snapshot.revision,
                snapshot.operation_revision
            ),
            Err(Error::Busy)
        ));
        #[cfg(windows)]
        assert!(matches!(
            state.apply_launcher_update(
                Some(&config),
                &InstalledTarget::current().unwrap(),
                offer,
                snapshot.revision,
                snapshot.operation_revision
            ),
            Err(Error::Busy)
        ));
        assert_eq!(
            serde_json::to_value(state.launcher_update_snapshot(Some(&config)).unwrap()).unwrap(),
            before
        );
        state.end_prerequisite_setup();
        assert!(state
            .update_admission(snapshot.revision, snapshot.operation_revision)
            .is_ok());
    }
}
