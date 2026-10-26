//! Driver tests of the per-host session state (CT-86): one slot per auth host,
//! the offline switch resolves its own host, and sign-out forgets before it
//! tells the server.

use std::sync::atomic::{AtomicBool, Ordering};

use super::*;
use crate::client_setup::login_servers::LoginServer;
use crate::NativeCommand;

/// Points the login server at `url` (an offline or unparsed target), as the login screen does.
fn save_login_server(fx: &Fixture, name: &str, url: &str) {
    fx.state
        .lock()
        .unwrap()
        .dispatch(NativeCommand::SaveClientOptions {
            schema_version: 1,
            expected_revision: 0,
            login_servers: vec![LoginServer {
                name: name.to_owned(),
                url: url.to_owned(),
            }],
            client_patches_enabled: false,
        })
        .expect("the server list saves");
}

#[tokio::test]
async fn a_retained_session_and_a_restored_one_refresh_once() {
    let fx = Fixture::new(true, true).await;
    fx.store_entry("r1");
    fx.inspect().await.unwrap();
    let old = fx.state.lock().unwrap().sign_in().unwrap();
    // A switch away and back drops the state's session; the next inspect restores one.
    fx.state.lock().unwrap().set_sign_in(None);
    fx.inspect().await.unwrap();
    let new = fx.state.lock().unwrap().sign_in().unwrap();
    assert!(!Arc::ptr_eq(&old, &new));
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(json!({"grant_type": "refresh_token"})))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(signed_in_body("r2"))
                .set_delay(Duration::from_millis(300)),
        )
        .expect(1)
        .mount(&fx.server)
        .await;
    let (a, b) = tokio::join!(old.access_token(), new.access_token());
    assert_eq!(a.unwrap(), b.unwrap());
    assert_eq!(fx.store.get(HOST).unwrap().refresh_token, "r2");
}

#[tokio::test]
async fn offline_switch_shows_and_signs_out_only_the_new_host() {
    let fx = Fixture::new(true, true).await;
    fx.store_entry("ra");
    let online = fx.inspect().await.unwrap();
    assert!(matches!(online.state, SignInStateView::SignedIn { .. }));
    fx.store.set(
        "localhost",
        &Stored {
            refresh_token: "rb".to_owned(),
            account_name: "Player_5678".to_owned(),
            display_name: "Player 5678".to_owned(),
        },
    );
    // A closed port: the new server is offline, and its auth base is `localhost`.
    std::env::set_var(AUTH_BASE_ENV, "http://localhost:1/");
    save_login_server(&fx, "Other", "https://other.example/login");
    let status = fx.inspect().await.unwrap();
    assert!(!status.available);
    assert!(matches!(
        &status.state,
        SignInStateView::SignedIn { account_name, .. } if account_name == "Player_5678"
    ));
    fx.run(
        json!({"command": "sign_out", "schema_version": 1}),
        &SilentBrowser::default(),
    )
    .await
    .unwrap();
    assert!(fx.store.get(HOST).is_some());
    assert!(fx.store.get("localhost").is_none());
}

#[tokio::test]
async fn a_target_without_an_auth_base_drops_the_previous_session() {
    let fx = Fixture::new(true, true).await;
    fx.store_entry("ra");
    fx.inspect().await.unwrap();
    assert!(fx.has_session());
    // An IP host has no auth base, so the previous session must not carry over.
    std::env::remove_var(AUTH_BASE_ENV);
    save_login_server(&fx, "Lan", "http://203.0.113.5:8081/login");
    let status = fx.inspect().await.unwrap();
    assert!(matches!(status.state, SignInStateView::SignedOut));
    assert!(!fx.has_session());
    assert!(fx.store.get(HOST).is_some());
}

#[tokio::test]
async fn sign_out_forgets_before_it_tells_the_server() {
    let fx = Fixture::new(true, true).await;
    fx.store_entry("r0");
    fx.inspect().await.unwrap();
    let forgotten = Arc::new(AtomicBool::new(false));
    let store = fx.store.clone();
    let flag = forgotten.clone();
    Mock::given(method("POST"))
        .and(path("/auth/v1/logout"))
        .respond_with(move |_: &wiremock::Request| {
            flag.store(store.get(HOST).is_none(), Ordering::SeqCst);
            ResponseTemplate::new(204)
        })
        .expect(1)
        .mount(&fx.server)
        .await;
    fx.run(
        json!({"command": "sign_out", "schema_version": 1}),
        &SilentBrowser::default(),
    )
    .await
    .unwrap();
    assert!(forgotten.load(Ordering::SeqCst));
}
