//! Production commit observes identity after releasing Windows owner handles, and
//! failed preparations and commits log their cause at WARN (PD-17).
use super::*;
use crate::test_capture::{capture, Captured};
use std::cell::RefCell;

thread_local! {
    static CAPTURE: RefCell<Option<tracing::dispatcher::DefaultGuard>> = const { RefCell::new(None) };
}

#[test]
fn production_update_commit_logs_target_once_after_owner_unlock_and_failure_logs_none() {
    for fail in [false, true] {
        let preparation_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (_root, state, prepared, _server) =
            preparation_runtime.block_on(super::journey_tests::fixture());
        let ((result, plan), events) = capture(|| {
            let dispatch = tracing::dispatcher::get_default(Clone::clone);
            // Tokio's blocking worker needs the same capture subscriber. Keep its
            // guard for that thread's entire lifetime, including terminal logs.
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .on_thread_start(move || {
                    CAPTURE.with(|slot| {
                        *slot.borrow_mut() = Some(tracing::dispatcher::set_default(&dispatch))
                    });
                })
                .on_thread_stop(|| {
                    CAPTURE.with(|slot| {
                        slot.borrow_mut().take();
                    })
                })
                .build()
                .unwrap();
            runtime.block_on(async {
                let plan = prepared.plan.clone();
                assert_ne!(plan.previous.manifest_digest, plan.target.manifest_digest);
                if fail {
                    state
                        .lock()
                        .unwrap()
                        .operations_mut()
                        .unwrap()
                        .request_cancel(plan.id)
                        .unwrap();
                }
                let result = commit::start(state.clone(), prepared)
                    .unwrap()
                    .await
                    .unwrap();
                (result, plan)
            })
        });
        let identities: Vec<&Captured> = events
            .iter()
            .filter(|e| e.field("event") == Some("applied_content_identity"))
            .collect();
        let committed: Vec<&Captured> = events
            .iter()
            .filter(|e| e.field("event") == Some("update_committed"))
            .collect();
        assert_eq!(committed.len(), 1);
        if fail {
            assert!(result.is_err());
            assert!(identities.is_empty());
            assert_eq!(committed[0].level, tracing::Level::WARN);
            assert_eq!(committed[0].field("outcome"), Some("cancelled"));
        } else {
            result.unwrap();
            assert_eq!(committed[0].level, tracing::Level::INFO);
            assert_eq!(identities.len(), 1);
            let digest: String = plan
                .target
                .manifest_digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert_eq!(
                identities[0].field("manifest_sha256"),
                Some(digest.as_str())
            );
            assert_eq!(identities[0].field("phase"), Some("applied"));
            assert_eq!(identities[0].field("overlay_ids"), Some("[]"));
            assert_eq!(identities[0].field("incomplete_overlay_ids"), Some("[]"));
        }
    }
}

#[test]
fn a_failed_seed_download_logs_update_prepare_failed_at_the_install_stage() {
    let setup = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let (_root, state, plan, server) = setup.block_on(async {
        let (root, state, installation, server) = install_worker::tests::prepared_fixture().await;
        // No seed is mounted, so the seed download answers 404.
        server.reset().await;
        let mut archive =
            zip::ZipWriter::new_append(std::io::Cursor::new(install_worker::tests::archive(true)))
                .unwrap();
        archive
            .start_file("version-two.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut archive, b"new signed content").unwrap();
        let target = install_worker::tests::verified(&archive.finish().unwrap().into_inner());
        let plan = {
            let mut owner = state.lock().unwrap();
            let installed = owner.installed_content().unwrap().unwrap();
            let revision = owner.operations().snapshot().revision;
            owner
                .admit_update(Request {
                    id: Uuid::new_v4(),
                    operation_revision: revision,
                    installation_id: installation,
                    expected_current: installed.current_release,
                    target: &target,
                    confirmed: true,
                })
                .unwrap()
                .plan
        };
        (root, state, plan, server)
    });
    let url = format!("{}/manifest.json", server.uri());
    let (result, events) = capture(|| {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            preparation::start(state.clone(), plan.id, reqwest::Client::new(), url)
                .unwrap()
                .result
                .await
                .unwrap()
                .map(|_| ())
        })
    });
    assert_eq!(result, Err(preparation::Failure::Failed));
    let failed: Vec<&Captured> = events
        .iter()
        .filter(|e| e.field("event") == Some("update_prepare_failed"))
        .collect();
    assert_eq!(failed.len(), 1, "{events:?}");
    assert_eq!(failed[0].target, "launcher.game_update");
    assert_eq!(failed[0].level, tracing::Level::WARN);
    assert_eq!(failed[0].field("stage"), Some("install"));
    assert_eq!(failed[0].field("error"), None);
}
