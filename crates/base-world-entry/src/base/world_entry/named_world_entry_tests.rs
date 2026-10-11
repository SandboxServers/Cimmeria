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

/// Run `mapLoaded` for a session logged in as `account_name` from `addr` and
/// return the `World entry complete` line.
async fn world_entry_complete_for(
    capture: &LogCaptureGuard,
    account_name: &str,
    addr: SocketAddr,
) -> Captured {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
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

/// TG-NET-18 (D-TG14): world entry says whether the session's launcher
/// minted a client telemetry stream, joined on the launcher's machine id and
/// this session's address (an account match wins), and names the stream's
/// `session_id`, the join key to the replayed client rows. Each of the four
/// values is checked with the fields it carries and the ones it must not;
/// removing a field from the `World entry complete` line fails its step.
///
/// One test, in order, because `unknown` means nothing has been recorded in
/// this process yet: no other test in this crate records a stream.
#[tokio::test]
async fn world_entry_complete_says_whether_the_session_has_client_telemetry() {
    use cimmeria_auth::auth::client_telemetry::{record_client_telemetry_stream, StreamRecord};

    let capture = LogCapture::install();
    let at = |last: u8| SocketAddr::from(([10, 18, 0, last], 55612));
    let record = |sid, mid, acct, last| {
        record_client_telemetry_stream(StreamRecord {
            session_id: sid,
            expires_at_unix: i64::MAX,
            machine_id: Some(mid),
            account_name: acct,
            peer_ip: Some(at(last).ip()),
        })
    };
    let no_join_fields = |row: &Captured| {
        for f in [
            "client_telemetry_session_id",
            "client_machine_id",
            "client_telemetry_candidates",
        ] {
            assert!(!row.fields.contains_key(f), "{f}: {row:#?}");
        }
    };

    // No launcher has minted since boot.
    let unknown = world_entry_complete_for(&capture, "tg18_before_any", at(1)).await;
    assert!(
        unknown.has_field("client_telemetry", "unknown"),
        "{unknown:#?}"
    );
    no_join_fields(&unknown);

    // One machine on the session's address.
    record("tg18-sid-a", "tg18machine0000a", None, 2);
    let present = world_entry_complete_for(&capture, "tg18_one_machine", at(2)).await;
    assert!(
        present.has_field("client_telemetry", "present"),
        "{present:#?}"
    );
    assert!(
        present.has_field("client_telemetry_session_id", "tg18-sid-a"),
        "{present:#?}"
    );
    assert!(
        present.has_field("client_machine_id", "tg18machine0000a"),
        "{present:#?}"
    );

    // Two machines behind one NAT.
    record("tg18-sid-b", "tg18machine0000b", None, 3);
    record("tg18-sid-c", "tg18machine0000c", None, 3);
    let ambiguous = world_entry_complete_for(&capture, "tg18_behind_nat", at(3)).await;
    assert!(
        ambiguous.has_field("client_telemetry", "ambiguous"),
        "{ambiguous:#?}"
    );
    assert!(
        ambiguous.has_field("client_telemetry_candidates", "2"),
        "{ambiguous:#?}"
    );
    assert!(
        !ambiguous.fields.contains_key("client_telemetry_session_id"),
        "{ambiguous:#?}"
    );

    // Launchers have minted, none from this address.
    let absent = world_entry_complete_for(&capture, "tg18_no_stream", at(4)).await;
    assert!(
        absent.has_field("client_telemetry", "absent"),
        "{absent:#?}"
    );
    no_join_fields(&absent);

    // A stream naming the account wins over the ambiguous address.
    record("tg18-sid-d", "tg18machine0000d", Some("tg18_named_acct"), 9);
    let named = world_entry_complete_for(&capture, "tg18_named_acct", at(3)).await;
    assert!(
        named.has_field("client_telemetry_session_id", "tg18-sid-d"),
        "{named:#?}"
    );
}
