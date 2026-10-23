//! Production commit observes identity after releasing Windows owner handles.
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
        if fail {
            assert!(result.is_err());
            assert!(identities.is_empty());
        } else {
            result.unwrap();
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
