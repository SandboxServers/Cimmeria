//! Captured confirmation and recovery use the actual Windows marker locks.
use super::in_place_tests::{commit, commit_with};
use super::light_replay::tests::{patch_zip, release_with_roots};
use super::light_tests::Light;
use super::*;
use crate::test_capture::{capture, Captured};

fn identities(events: &[Captured]) -> Vec<&Captured> {
    events
        .iter()
        .filter(|e| e.field("event") == Some("applied_content_identity"))
        .collect()
}
fn assert_identity(events: &[Captured], digest: [u8; 32], overlays: &str) {
    let rows = identities(events);
    assert_eq!(rows.len(), 1);
    let digest: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(rows[0].field("manifest_sha256"), Some(digest.as_str()));
    assert_eq!(rows[0].field("phase"), Some("applied"));
    assert_eq!(rows[0].field("overlay_ids"), Some(overlays));
    assert_eq!(rows[0].field("incomplete_overlay_ids"), Some("[]"));
}

async fn in_place_identity(interrupted: bool) {
    {
        let zips = vec![patch_zip(
            "p0",
            &[],
            &[("Content/Identity.dat", b"overlay identity")],
        )];
        let release = release_with_roots(&zips, &["sgw_game"]);
        let fixture = Light::with_release(release, zips, &[]).await;
        let digest = fixture.release.digest();
        let preview = fixture.preview().unwrap();
        if !interrupted {
            let (result, events) = capture(|| commit(preview));
            result.unwrap();
            assert_identity(&events, digest, "[\"p0\"]");
        } else {
            let (result, events) = capture(|| {
                commit_with(preview, |point| {
                    if point == publication::Point::BeforePromotion {
                        Err(StorageError::Io.into())
                    } else {
                        Ok(())
                    }
                })
            });
            assert!(result.is_err());
            assert!(identities(&events).is_empty());
            let state_dir = fixture.root.path().join("state");
            let id = fixture
                .state
                .lock()
                .unwrap()
                .operations()
                .snapshot()
                .operation
                .as_ref()
                .unwrap()
                .id;
            drop(fixture.state);
            let mut state = DesktopState::open(&state_dir).unwrap();
            let revision = state.operations().snapshot().revision;
            let (refused, events) = capture(|| recover(&mut state, id, revision + 1));
            assert!(refused.is_err());
            assert!(identities(&events).is_empty());
            let (result, events) = capture(|| recover(&mut state, id, revision));
            result.unwrap();
            assert_identity(&events, digest, "[\"p0\"]");
            let revision = state.operations().snapshot().revision;
            let (result, events) = capture(|| recover(&mut state, id, revision));
            result.unwrap();
            assert!(identities(&events).is_empty());
        }
    }
}

fn copy_identity(interrupted: bool) {
    {
        let fixture = super::tests::Fixture::new();
        let preview = fixture.preview().unwrap();
        let digest = preview.release.digest();
        let handle = preview.report.preview_handle;
        let id = Uuid::new_v4();
        let (result, events) = capture(|| {
            publication::confirm(
                preview,
                id,
                handle,
                super::tests::choices(),
                CancellationToken::new(),
                &crate::install_progress::ProgressSink::latest().0,
                |point| {
                    if interrupted && point == publication::Point::BeforePromotion {
                        Err(StorageError::Io.into())
                    } else {
                        Ok(())
                    }
                },
            )
        });
        if !interrupted {
            result.unwrap();
            assert_identity(&events, digest, "[]");
        } else {
            assert!(result.is_err());
            assert!(identities(&events).is_empty());
            let state_dir = fixture.root.path().join("state");
            drop(fixture.state);
            let mut state = DesktopState::open(&state_dir).unwrap();
            let revision = state.operations().snapshot().revision;
            let (result, events) = capture(|| recover(&mut state, id, revision));
            result.unwrap();
            assert_identity(&events, digest, "[]");
            let revision = state.operations().snapshot().revision;
            let (result, events) = capture(|| recover(&mut state, id, revision));
            result.unwrap();
            assert!(identities(&events).is_empty());
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn in_place_confirmation_logs_one_retained_overlay_identity() {
    in_place_identity(false).await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn in_place_recovery_logs_once_and_failed_replayed_attempts_are_silent() {
    in_place_identity(true).await;
}
#[test]
fn copy_confirmation_logs_one_retained_identity() {
    copy_identity(false);
}
#[test]
fn copy_recovery_logs_once_and_failed_replayed_attempts_are_silent() {
    copy_identity(true);
}
