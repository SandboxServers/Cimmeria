//! Rule 6 guards for the two session-end lines operators read most: the
//! disconnect teardown's `player session ended` (`session.end`) and the
//! tick-sync inactivity timeout. Each carries the names next to the IDs, so a
//! SigNoz row or a Discord post says who left without a lookup
//! (`docs/architecture/instrumentation-discipline.md` Rule 6, NT-24).
//!
//! Each assertion fails when its name field is removed from the line.

use std::time::Instant;

use super::*;
use tracing::Level;

use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};

const ACCOUNT_ID: u32 = 0x5EED_2401;
const PLAYER_ID: i32 = 0x2401;
const PLAYER_EID: u32 = 0x5EED_2402;

fn named_session() -> ConnectedClientState {
    let mut state = test_default_connected_client_state();
    state.account_id = ACCOUNT_ID;
    state.account_name = Some("sgc_login".into());
    state.active_player_id = Some(PLAYER_ID);
    state.player_name = Some("Teal'c".into());
    state.player_entity_id = Some(PLAYER_EID);
    state
}

type Maps = (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<cimmeria_entity::manager::EntityManager>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
);

fn maps(addr: SocketAddr) -> Maps {
    (
        Arc::new(Mutex::new(HashMap::from([(addr, named_session())]))),
        Arc::new(Mutex::new(cimmeria_entity::manager::EntityManager::new())),
        Arc::new(Mutex::new(HashMap::from([(PLAYER_EID, addr)]))),
    )
}

fn transport() -> Arc<dyn cimmeria_mercury::transport::Transport> {
    Arc::new(TestTransport::new())
}

/// The disconnect line names the entity, the account and the character,
/// snapshotted before the session is removed. Before NT-24 it carried
/// `player_name` as `Some("…")` (Debug of an `Option<String>`) and no
/// account name at all.
#[test]
fn session_end_line_names_the_player_and_the_account() {
    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:52401".parse().unwrap();
    let (connected, entity_manager, entity_to_addr) = maps(addr);

    destroy_client_entities(
        &connected,
        &entity_manager,
        addr,
        &None,
        &entity_to_addr,
        &transport(),
        &None,
        "client_disconnect",
    );

    let ended = capture
        .find_message(Level::INFO, "player session ended")
        .unwrap_or_else(|| panic!("no session.end line: {:#?}", capture.all()));
    assert!(ended.has_field("entity_id", &PLAYER_EID.to_string()));
    assert!(ended.has_field("entity_name", "Teal'c"), "{ended:#?}");
    assert!(ended.has_field("account_id", &ACCOUNT_ID.to_string()));
    assert!(ended.has_field("account_name", "sgc_login"), "{ended:#?}");
    assert!(ended.has_field("player_id", &PLAYER_ID.to_string()));
    assert!(ended.has_field("player_name", "Teal'c"), "{ended:#?}");

    let cleaned = capture
        .find_message(Level::INFO, "Client entities cleaned up")
        .expect("cleanup line");
    assert!(
        cleaned.has_field("account_name", "sgc_login"),
        "{cleaned:#?}"
    );
    assert!(
        cleaned.has_field("player_entity_name", "Teal'c"),
        "{cleaned:#?}"
    );
}

/// A session with no character yet (character select) leaves the character
/// names off the line rather than writing `""` or `"None"` (Rule 6:
/// absent when unresolved).
#[test]
fn session_end_at_character_select_leaves_the_character_names_off() {
    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:52402".parse().unwrap();
    let mut state = test_default_connected_client_state();
    state.account_id = ACCOUNT_ID;
    state.account_name = Some("sgc_login".into());
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
    let entity_manager = Arc::new(Mutex::new(cimmeria_entity::manager::EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));

    destroy_client_entities(
        &connected,
        &entity_manager,
        addr,
        &None,
        &entity_to_addr,
        &transport(),
        &None,
        "client_disconnect",
    );

    let cleaned = capture
        .find_message(Level::INFO, "Client entities cleaned up")
        .expect("cleanup line");
    assert!(
        cleaned.has_field("account_name", "sgc_login"),
        "{cleaned:#?}"
    );
    assert!(
        !cleaned.fields.contains_key("player_entity_name"),
        "an unresolved name is left off: {cleaned:#?}"
    );
}

/// The inactivity timeout names who went silent. The session is still in
/// the map when the line is written; the teardown runs after it.
#[tokio::test]
async fn inactivity_timeout_line_names_the_player_and_the_account() {
    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:52403".parse().unwrap();
    let (connected, entity_manager, entity_to_addr) = maps(addr);
    // Silent for longer than the 60 s reap threshold. `checked_sub` can
    // fail only on a host up for under 61 s; fail loudly rather than pass
    // without asserting anything.
    let long_ago = Instant::now()
        .checked_sub(std::time::Duration::from_secs(61))
        .expect("host uptime must exceed 61 s to backdate last_recv");

    crate::base::tick_sync::run_tick_loop(
        transport(),
        addr,
        [0u8; 32],
        Default::default(),
        Arc::new(std::sync::atomic::AtomicU32::new(0)),
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(long_ago)),
        Arc::new(std::sync::atomic::AtomicBool::new(false)),
        Arc::clone(&connected),
        entity_manager,
        None,
        entity_to_addr,
        None,
    )
    .await;

    let timeout = capture
        .find_message(Level::INFO, "client inactive")
        .unwrap_or_else(|| panic!("no timeout line: {:#?}", capture.all()));
    assert!(timeout.has_field("account_id", &ACCOUNT_ID.to_string()));
    assert!(
        timeout.has_field("account_name", "sgc_login"),
        "{timeout:#?}"
    );
    assert!(timeout.has_field("player_id", &PLAYER_ID.to_string()));
    assert!(timeout.has_field("player_name", "Teal'c"), "{timeout:#?}");
    assert!(
        capture
            .find_message(Level::INFO, "player session ended")
            .is_some_and(|e| e.has_field("disconnect_reason", "inactivity_timeout")),
        "the timeout still tears the session down"
    );
}
