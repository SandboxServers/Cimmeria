//! Driver tests of Sign out and Cancel while the installed copy's record cannot be
//! read (CT-90). Such a request has no login server to talk to, so it must still
//! end the sign-in instead of failing with `unavailable`. The record is replaced
//! with bytes that are not JSON, which is what a half-written file looks like.

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
}
