//! Sign-in offers the server Play launches against (C18.2). Kept out of
//! `adopted_tests`, which only compiles on macOS, so these run on every host.
use super::fixtures::{self, Legacy};
use super::*;
use crate::client_setup::login_servers::LoginServer;
use std::sync::Mutex;

fn saved() -> LoginServer {
    LoginServer {
        name: "Saved".into(),
        url: "http://saved.example:8081".into(),
    }
}

#[test]
fn sign_in_offers_the_adopted_server_not_the_saved_one() {
    let legacy = Legacy::new(false, true);
    let state = Arc::new(Mutex::new(legacy.open()));
    fixtures::adopt_native(&legacy, &state);
    drop(state);
    let mut state = DesktopState::open(&legacy.state_root()).unwrap();
    let revision = state.preferences().revision;
    state
        .save_client_options(vec![saved()], true, revision)
        .unwrap();
    let (installed, _) = state.installed_for_launch().unwrap().unwrap();
    let launched = state.launch_login_servers(&installed.intent).unwrap();
    let (name, url) = fixtures::SERVERS[0];
    let offered = state
        .sign_in_login_server()
        .unwrap()
        .expect("an adopted copy offers its first reviewed server");
    assert_eq!((offered.name.as_str(), offered.url.as_str()), (name, url));
    assert_eq!(offered, launched[0]);
}

#[test]
fn sign_in_without_an_installation_uses_the_saved_list() {
    let root = tempfile::tempdir().unwrap();
    let mut state = DesktopState::open(&root.path().join("state")).unwrap();
    let revision = state.preferences().revision;
    state
        .save_client_options(vec![saved()], true, revision)
        .unwrap();
    assert!(state.installed_for_launch().unwrap().is_none());
    assert_eq!(state.sign_in_login_server().unwrap(), Some(saved()));
}
