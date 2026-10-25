//! The sign-in Play planner against a wiremock auth server. No client is
//! launched; the relay listens on loopback and is shut down by each test.
use super::sign_in_play::{self, ClientLogin, TicketRefusal};
use super::tests::fixture;
use super::Plan;
use crate::sign_in::discovery::{self, Discovery};
use crate::sign_in::session::SignIn;
use crate::sign_in::store::{CredentialStore, MemoryStore, Stored};
use cimmeria_launcher_core::client_setup::login_servers::{self, LoginServer};
use cimmeria_launcher_core::client_setup::login_session::Links;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use url::Url;
use wiremock::matchers::{body_partial_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// A fixture install with the login folder present, and its plan.
fn install() -> (TempDir, Plan) {
    let (dir, _state, plan) = fixture();
    let login = login_servers::path(dir.path());
    std::fs::create_dir_all(login.parent().expect("login folder")).expect("folder");
    (dir, plan)
}

fn base_of(server: &MockServer) -> Url {
    Url::parse(&format!("{}/", server.uri())).expect("wiremock URI parses")
}

fn servers() -> Vec<LoginServer> {
    vec![
        LoginServer {
            name: "Cimmeria".into(),
            url: "https://play.example.org/SGWLogin".into(),
        },
        LoginServer {
            name: "Second".into(),
            url: "https://second.example.org/SGWLogin".into(),
        },
    ]
}

fn discovery_at(base: Url) -> Discovery {
    Discovery {
        auth_base: base,
        soap_relay: true,
        signup_open: false,
        providers: Vec::new(),
        links: Links::default(),
    }
}

/// A session restored from a stored entry for `base`'s host.
fn signed_in(base: Url) -> Arc<SignIn> {
    let store = Arc::new(MemoryStore::default());
    store.set(
        base.host_str().expect("host"),
        &Stored {
            refresh_token: "r0".to_owned(),
            account_name: "alice".to_owned(),
            display_name: "Alice".to_owned(),
        },
    );
    Arc::new(SignIn::restore(base, discovery::client(), store).expect("stored"))
}

async fn mount_token(server: &MockServer) {
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .and(body_partial_json(json!({ "grant_type": "refresh_token" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "result": "signed_in",
            "access_token": "a1",
            "expires_in": 900,
            "refresh_token": "r1",
            "account": { "account_name": "alice", "display_name": "Alice", "identities": [] },
        })))
        .mount(server)
        .await;
}

async fn mount_ticket(server: &MockServer, status: u16) {
    let reply = if status == 200 {
        ResponseTemplate::new(200).set_body_json(json!({
            "account_name": "alice",
            "ticket": "abcdefghijklmnopqrstuvwxyz012345",
            "expires_in": 60,
        }))
    } else {
        ResponseTemplate::new(status).set_body_json(json!({ "error": "refused" }))
    };
    Mock::given(method("POST"))
        .and(path("/auth/v1/launch-ticket"))
        .respond_with(reply)
        .mount(server)
        .await;
}

#[tokio::test]
async fn no_discovery_keeps_the_plain_list() {
    let (_dir, plan) = install();
    let list = servers();
    let (client, login) = sign_in_play::plan(None, &plan, &list, None)
        .await
        .unwrap_or_else(|e| panic!("refused: {e:?}"));
    assert_eq!(client, list);
    assert!(matches!(login, ClientLogin::Plain));
}

#[tokio::test]
async fn relay_password_mode_repoints_only_the_first_server() {
    let server = MockServer::start().await;
    let (dir, plan) = install();
    let list = servers();
    let (client, login) =
        sign_in_play::plan(None, &plan, &list, Some(discovery_at(base_of(&server))))
            .await
            .unwrap_or_else(|e| panic!("refused: {e:?}"));
    assert!(matches!(login, ClientLogin::RelayPassword { .. }));
    assert_ne!(client[0].url, list[0].url);
    assert!(client[0].url.starts_with("http://127.0.0.1:"));
    assert_eq!(client[1], list[1]);
    sign_in_play::finish(dir.path(), &list, login).await;
}

#[tokio::test]
async fn signed_in_to_another_auth_base_uses_relay_password() {
    let server = MockServer::start().await;
    let (dir, plan) = install();
    let list = servers();
    let other = signed_in(Url::parse("https://other.example.org/").expect("url"));
    let discovered = Some(discovery_at(base_of(&server)));
    let (client, login) = sign_in_play::plan(Some(other), &plan, &list, discovered)
        .await
        .unwrap_or_else(|e| panic!("refused: {e:?}"));
    assert!(matches!(login, ClientLogin::RelayPassword { .. }));
    assert_eq!(client[1], list[1]);
    sign_in_play::finish(dir.path(), &list, login).await;
}

#[tokio::test]
async fn signed_in_play_writes_a_ticket_file_with_one_server() {
    let server = MockServer::start().await;
    mount_token(&server).await;
    mount_ticket(&server, 200).await;
    let (dir, plan) = install();
    let list = servers();
    let sign_in = Some(signed_in(base_of(&server)));
    let (client, login) =
        sign_in_play::plan(sign_in, &plan, &list, Some(discovery_at(base_of(&server))))
            .await
            .unwrap_or_else(|e| panic!("refused: {e:?}"));
    assert_eq!(client.len(), 1);
    assert!(client[0].url.starts_with("http://127.0.0.1:"));
    sign_in_play::write(dir.path(), &login).expect("ticket written");
    let text = std::fs::read_to_string(login_servers::path(dir.path())).expect("file");
    assert!(text.contains("CimmeriaLogin.mode = \"ticket\""));
    assert!(text.contains(&client[0].url));
    sign_in_play::finish(dir.path(), &list, login).await;
}

#[tokio::test]
async fn ticket_failure_ends_the_launch_with_an_error() {
    for status in [401, 503] {
        let server = MockServer::start().await;
        mount_token(&server).await;
        mount_ticket(&server, status).await;
        let (_dir, plan) = install();
        let sign_in = Some(signed_in(base_of(&server)));
        let discovered = Some(discovery_at(base_of(&server)));
        let result = sign_in_play::plan(sign_in, &plan, &servers(), discovered).await;
        assert!(
            matches!(result, Err(TicketRefusal::Unavailable)),
            "status {status} must not fall back to the password"
        );
    }
}

#[tokio::test]
async fn signed_out_refresh_is_sign_in_ended() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/auth/v1/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({ "error": "invalid_grant" })))
        .mount(&server)
        .await;
    let (_dir, plan) = install();
    let sign_in = Some(signed_in(base_of(&server)));
    let discovered = Some(discovery_at(base_of(&server)));
    let result = sign_in_play::plan(sign_in, &plan, &servers(), discovered).await;
    assert!(matches!(result, Err(TicketRefusal::SignInEnded)));
    assert_eq!(TicketRefusal::SignInEnded.reason(), "sign_in_ended");
}

#[tokio::test]
async fn finish_scrubs_the_ticket_and_stops_the_relay() {
    let server = MockServer::start().await;
    mount_token(&server).await;
    mount_ticket(&server, 200).await;
    let (dir, plan) = install();
    let list = servers();
    let sign_in = Some(signed_in(base_of(&server)));
    let discovered = Some(discovery_at(base_of(&server)));
    let (_client, login) = sign_in_play::plan(sign_in, &plan, &list, discovered)
        .await
        .unwrap_or_else(|e| panic!("refused: {e:?}"));
    sign_in_play::write(dir.path(), &login).expect("ticket written");
    let port = match &login {
        ClientLogin::Ticket { relay, .. } => relay.soap_port(),
        _ => panic!("expected a ticket"),
    };
    sign_in_play::finish(dir.path(), &list, login).await;
    let text = std::fs::read_to_string(login_servers::path(dir.path())).expect("file");
    assert_eq!(text, login_servers::render(&list));
    assert!(tokio::net::TcpStream::connect(("127.0.0.1", port))
        .await
        .is_err());
}

#[test]
fn telemetry_keeps_the_real_servers() {
    let list = servers();
    let relayed = vec![LoginServer {
        name: list[0].name.clone(),
        url: "http://127.0.0.1:1/SGWLogin".into(),
    }];
    let (telemetry, client) = sign_in_play::lists_for(&list, relayed.clone());
    assert_eq!(telemetry, list.as_slice());
    assert_eq!(client, relayed);
}

#[test]
fn play_refusal_is_recorded_and_cleared() {
    let (_dir, mut state, _plan) = fixture();
    assert_eq!(state.play_refusal(), None);
    state.set_play_refusal(Some("ticket_unavailable"));
    assert_eq!(state.play_refusal(), Some("ticket_unavailable"));
    state.set_play_refusal(None);
    assert_eq!(state.play_refusal(), None);
}
