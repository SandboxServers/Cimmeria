//! Rule 6 guard for the play-character path's closing line, `World entry
//! complete`: the account and the character are named next to their IDs,
//! and the world by name (`docs/architecture/instrumentation-discipline.md`
//! Rule 6, NT-24). Each assertion fails with its name field removed.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use tracing::Level;

use super::handle_map_loaded;
use crate::base::ConnectedClientState;
use crate::test_support::{
    test_default_connected_client_state, Captured, LogCapture, LogCaptureGuard, TestTransport,
};

const PLAYER_EID: u32 = 7701;
const PLAYER_ID: i32 = 0x2402;

#[tokio::test]
async fn world_entry_complete_names_the_account_the_character_and_the_world() {
    let capture = LogCapture::install();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let addr: SocketAddr = "127.0.0.1:55611".parse().unwrap();

    let mut data = crate::base::world_entry::methods::default_player_load_data();
    data.player_id = PLAYER_ID;
    data.player_name = "Teal'c".to_string();
    data.archetype = 1;
    let mut client = test_default_connected_client_state();
    client.account_id = 0x2402;
    client.account_name = Some("sgc_login".into());
    client.active_player_id = Some(PLAYER_ID);
    client.player_name = Some("Teal'c".into());
    client.pending_player_load_data = Some(data);
    client.pending_map_loaded = Some(crate::mercury::types::WorldEntryInfo {
        player_entity_id: PLAYER_EID,
        space_id: 0x0001_0042,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "Agnos".to_string(),
        class_id: 2,
        world_stargates: Vec::new(),
    });
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::from([(addr, client)])));

    handle_map_loaded(
        &transport,
        addr,
        [0u8; 32],
        &connected,
        &None,
        &Arc::new(Mutex::new(HashMap::new())),
        &None,
    )
    .await
    .expect("map loaded");

    let done = capture
        .find_message(Level::INFO, "World entry complete")
        .unwrap_or_else(|| panic!("no completion line: {:#?}", capture.all()));
    assert!(done.has_field("account_id", "9218"), "{done:#?}");
    assert!(done.has_field("account_name", "sgc_login"), "{done:#?}");
    assert!(done.has_field("player_id", &PLAYER_ID.to_string()));
    assert!(done.has_field("player_name", "Teal'c"), "{done:#?}");
    assert!(done.has_field("world", "Agnos"), "{done:#?}");
    assert!(done.has_field("archetype_name", "Soldier"), "{done:#?}");

    let entered = capture
        .find_message(Level::INFO, "Enter world: client map loaded")
        .expect("enter-world line");
    assert!(
        entered.has_field("player_entity_name", "Teal'c"),
        "{entered:#?}"
    );
    assert!(entered.has_field("world", "Agnos"), "{entered:#?}");
}

/// Run `mapLoaded` for a session logged in as `account_name` and return the
/// `World entry complete` line.
async fn world_entry_complete_for(
    capture: &LogCaptureGuard,
    account_name: &str,
    port: u16,
) -> Captured {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let addr: SocketAddr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut client = test_default_connected_client_state();
    client.account_id = 0x2418;
    client.account_name = Some(account_name.into());
    client.active_player_id = Some(PLAYER_ID);
    client.player_name = Some("Teal'c".into());
    client.pending_player_load_data =
        Some(crate::base::world_entry::methods::default_player_load_data());
    client.pending_map_loaded = Some(crate::mercury::types::WorldEntryInfo {
        player_entity_id: PLAYER_EID,
        space_id: 0x0001_0042,
        pos: [0.0; 3],
        rot: [0.0; 3],
        world_name: "Agnos".to_string(),
        class_id: 2,
        world_stargates: Vec::new(),
    });
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::from([(addr, client)])));
    handle_map_loaded(
        &transport,
        addr,
        [0u8; 32],
        &connected,
        &None,
        &Arc::new(Mutex::new(HashMap::new())),
        &None,
    )
    .await
    .expect("map loaded");
    capture
        .all()
        .into_iter()
        .find(|e| {
            e.level == Level::INFO
                && e.message_contains("World entry complete")
                && e.has_field("account_name", account_name)
        })
        .unwrap_or_else(|| {
            panic!(
                "no completion line for {account_name}: {:#?}",
                capture.all()
            )
        })
}

/// TG-NET-18 (D-TG14): world entry says whether the account's launcher
/// minted a client telemetry stream, and names the stream's `session_id`,
/// the join key to the replayed client rows. An account with no stream says
/// `absent` and carries no session id. Removing the two fields from the
/// `World entry complete` line fails both halves.
#[tokio::test]
async fn world_entry_complete_says_whether_the_account_has_client_telemetry() {
    use cimmeria_auth::auth::client_telemetry::record_client_telemetry_stream;

    let capture = LogCapture::install();
    // The registry is process-wide: these names are this test's own.
    record_client_telemetry_stream("tg18_with_stream", "tg18-stream-sid", i64::MAX);

    let present = world_entry_complete_for(&capture, "tg18_with_stream", 55612).await;
    assert!(
        present.has_field("client_telemetry", "present"),
        "{present:#?}"
    );
    assert!(
        present.has_field("client_telemetry_session_id", "tg18-stream-sid"),
        "{present:#?}"
    );

    let absent = world_entry_complete_for(&capture, "tg18_without_stream", 55613).await;
    assert!(
        absent.has_field("client_telemetry", "absent"),
        "{absent:#?}"
    );
    assert!(
        !absent.fields.contains_key("client_telemetry_session_id"),
        "{absent:#?}"
    );
}
