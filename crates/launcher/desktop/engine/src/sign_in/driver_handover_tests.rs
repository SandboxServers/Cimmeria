//! Driver tests of one flow giving way to another: a login-server switch drops
//! the pending link choice, and a late unwind after a cancel leaves the newer
//! flow waiting.

use super::*;
use crate::client_setup::login_servers::LoginServer;
use crate::NativeCommand;

fn cancel() -> Value {
    json!({"command": "cancel", "schema_version": 1})
}

/// Waits until `browser` has been asked to open a flow, so the flow is waiting.
async fn until_opened(browser: &SilentBrowser) {
    for _ in 0..200 {
        if !browser.urls.lock().unwrap().is_empty() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!("the flow never opened the browser");
}

/// Points the login server at a second mock auth server, as if the player had
/// switched servers on the login screen. The returned server must stay alive.
async fn switch_to_second_server(fx: &Fixture) -> MockServer {
    let other = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/auth/v1/providers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "api": 1,
            "soap_relay": true,
            "signup_open": true,
            "providers": [{"id": "dev", "display_name": "Dev sign-in"}],
            "links": {"website": null, "account": null}
        })))
        .mount(&other)
        .await;
    std::env::set_var(AUTH_BASE_ENV, format!("{}/", other.uri()));
    fx.state
        .lock()
        .unwrap()
        .dispatch(NativeCommand::SaveClientOptions {
            schema_version: 1,
            expected_revision: 0,
            login_servers: vec![LoginServer {
                name: "Other".to_owned(),
                url: "https://other.example/login".to_owned(),
            }],
            client_patches_enabled: false,
        })
        .expect("the server list saves");
    other
}

#[tokio::test]
async fn pending_from_one_server_is_not_offered_on_another() {
    let fx = Fixture::new(true, true).await;
    mount_token(&fx.server, needs_link_body()).await;
    fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    let _other = switch_to_second_server(&fx).await;
    let status = fx.inspect().await.unwrap();
    assert!(status.available);
    assert!(matches!(status.state, SignInStateView::SignedOut));
}

#[tokio::test]
async fn link_after_a_server_switch_sends_nothing_to_the_first_server() {
    let fx = Fixture::new(true, true).await;
    mount_token(&fx.server, needs_link_body()).await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/link/existing"))
        .respond_with(ResponseTemplate::new(200).set_body_json(signed_in_body("r9")))
        .mount(&fx.server)
        .await;
    fx.run(begin(), &CallbackBrowser::default()).await.unwrap();
    let _other = switch_to_second_server(&fx).await;
    let reply = fx
        .run(password_link(true, "pw"), &SilentBrowser::default())
        .await;
    assert_eq!(reply, Err(SignInFailure::NoPendingLink));
    assert!(fx.requests_to("/auth/v1/link/existing").await.is_empty());
}

/// A flow that unwinds after a cancel must not clear the handle of the flow that
/// started meanwhile: that flow stays cancellable and a third one is `Busy`.
#[tokio::test]
async fn late_unwind_keeps_the_newer_flow_waiting() {
    let fx = Fixture::new(true, true).await;
    let first_browser = SilentBrowser::default();
    let second_browser = SilentBrowser::default();
    let first = fx.run(begin(), &first_browser);
    let rest = async {
        until_opened(&first_browser).await;
        let cancelled = fx.run(cancel(), &SilentBrowser::default()).await;
        let second = async {
            tokio::time::timeout(Duration::from_secs(5), fx.run(begin(), &second_browser))
                .await
                .expect("the second flow was not cancelled")
        };
        let check = async {
            until_opened(&second_browser).await;
            // A failing browser makes a flow that got past `Busy` end at once.
            let third = fx.run(begin(), &FailingBrowser).await;
            let ended = fx.run(cancel(), &SilentBrowser::default()).await;
            (third, ended)
        };
        let (second, (third, ended)) = tokio::join!(second, check);
        (cancelled, second, third, ended)
    };
    let (first, (cancelled, second, third, ended)) = tokio::join!(first, rest);
    assert_eq!(first, Err(SignInFailure::Cancelled));
    assert!(cancelled.is_ok());
    assert_eq!(third, Err(SignInFailure::Busy));
    assert!(ended.is_ok());
    assert_eq!(second, Err(SignInFailure::Cancelled));
}
