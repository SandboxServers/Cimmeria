//! Interrupted legacy copies release metadata without publishing settings or moving content.
use super::fixture::*;
use super::*;
use adoption::test_support::CopyFault;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_interrupted_legacy_copy_abandons_without_publishing_its_settings() {
    let mut f = Fixture::new(serde_json::json!({"client_patches": {"enabled": true}}));
    let _server = f.serve().await;
    let before_source = source_bytes(&f.source);
    f.begin().unwrap();
    let review = reviewed(&f.host).await;
    f.fault(CopyFault::AfterPromotion);
    f.host.adoption_command(confirmation(&review)).unwrap();
    let failed = until(
        &f.host,
        "the interrupted legacy copy",
        Duration::from_secs(30),
        |s| s.activity == Activity::Idle,
    )
    .await;
    let Some(Reconciliation::Copy {
        operation_id,
        can_recover: false,
        can_abandon: true,
        ..
    }) = failed.reconciliation
    else {
        panic!("legacy copy must only allow metadata abandonment: {failed:?}");
    };
    let binding = || {
        f.host
            .store()
            .unwrap()
            .lock()
            .unwrap()
            .effective_launch_binding()
    };
    assert_eq!(binding(), Ok(None));
    let mut retained = tree(&f.destination());
    retained.remove(&PathBuf::from(".cimmeria-install.json"));
    retained.remove(&PathBuf::from("content-ready.json"));
    assert_eq!(
        f.host
            .adoption_command(AdoptionCommand::Recover {
                schema_version: 1,
                operation_id,
                operation_revision: failed.native.operation.revision,
                confirmed: true,
            })
            .unwrap_err(),
        AdoptionError::RecoveryRequired
    );
    let abandoned = f
        .host
        .adoption_command(AdoptionCommand::Abandon {
            schema_version: 1,
            operation_id,
            operation_revision: failed.native.operation.revision,
            confirmed: true,
        })
        .unwrap();
    assert!(abandoned.reconciliation.is_none());
    assert_eq!(binding(), Ok(None));
    assert!(f.host.install_status().unwrap().uninstall.is_none());
    assert_eq!(tree(&f.destination()), retained);
    assert_eq!(source_bytes(&f.source), before_source);
}
