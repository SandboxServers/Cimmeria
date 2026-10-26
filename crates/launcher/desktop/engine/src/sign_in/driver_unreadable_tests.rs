//! Driver tests of Sign out, Cancel and Inspect while the installed copy's record
//! cannot be read (CT-90). Such a request has no login server to talk to. Sign out
//! must still end the sign-in, and Inspect and Cancel answer the sign-in the launcher
//! holds, offline-shaped, instead of failing with `unavailable`. The record is
//! replaced with bytes that are not JSON, which is what a half-written file looks like.

use super::*;

/// Signs in on the mock server, then makes the installed-content record unreadable.
async fn signed_in_with_unreadable_record(fx: &Fixture) {
    mount_token(&fx.server, signed_in_body("r9")).await;
    fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    assert!(fx.has_session());
    let record = fx._dir.path().join("state").join("installed-content.json");
    std::fs::write(record, b"{").unwrap();
}

#[tokio::test]
async fn sign_out_ends_the_sign_in_while_the_installed_copy_is_unreadable() {
    let fx = Fixture::new(true, true).await;
    signed_in_with_unreadable_record(&fx).await;
    let status = fx
        .run(
            json!({"command": "sign_out", "schema_version": 1}),
            &SilentBrowser::default(),
        )
        .await
        .unwrap();
    assert!(matches!(status.state, SignInStateView::SignedOut));
    assert!(!fx.has_session());
    assert!(fx.store.get(HOST).unwrap().is_none());
}

#[tokio::test]
async fn cancel_is_not_refused_while_the_installed_copy_is_unreadable() {
    let fx = Fixture::new(true, true).await;
    signed_in_with_unreadable_record(&fx).await;
    let status = fx
        .run(
            json!({"command": "cancel", "schema_version": 1}),
            &SilentBrowser::default(),
        )
        .await;
    assert!(status.is_ok(), "{status:?}");
    let status = status.unwrap();
    assert!(!status.available);
    assert!(matches!(status.state, SignInStateView::SignedIn { .. }));
    assert!(fx.has_session());
}

#[tokio::test]
async fn inspect_keeps_the_signed_in_panel_while_the_installed_copy_is_unreadable() {
    let fx = Fixture::new(true, true).await;
    signed_in_with_unreadable_record(&fx).await;
    let status = fx
        .run(
            json!({"command": "inspect", "schema_version": 1}),
            &SilentBrowser::default(),
        )
        .await
        .unwrap();
    assert!(!status.available);
    assert!(status.server_name.is_none());
    assert!(status.providers.is_empty());
    assert!(matches!(status.state, SignInStateView::SignedIn { .. }));
    assert!(fx.has_session());
}

#[tokio::test]
async fn inspect_while_signed_out_and_unreadable_answers_signed_out_not_unavailable() {
    let fx = Fixture::new(true, true).await;
    let record = fx._dir.path().join("state").join("installed-content.json");
    std::fs::create_dir_all(record.parent().unwrap()).unwrap();
    std::fs::write(record, b"{").unwrap();
    let status = fx
        .run(
            json!({"command": "inspect", "schema_version": 1}),
            &SilentBrowser::default(),
        )
        .await
        .unwrap();
    assert!(!status.available);
    assert!(matches!(status.state, SignInStateView::SignedOut));
}
