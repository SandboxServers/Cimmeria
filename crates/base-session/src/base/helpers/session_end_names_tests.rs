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

/// TG-NET-02: a full exit (`logOff(1)`) keeps the session, so the row that
/// ends it comes later from the teardown. The `LogoffRequested` marker
/// survives to that teardown: `session.end` keeps the true mechanism
/// (`inactivity_timeout`) and adds `logoff_requested=true`, the idle time,
/// whether the character was in the world, and the world. The cleanup row
/// carries a numeric `player_id` and a bare `player_entity_id`, not
/// `"Some(n)"`. Reverting any field drops it from the row and fails here.
#[test]
fn logoff_requested_survives_to_inactivity_teardown() {
    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:52404".parse().unwrap();
    let (connected, entity_manager, entity_to_addr) = maps(addr);
    {
        let mut clients = connected.lock().unwrap();
        let c = clients.get_mut(&addr).unwrap();
        c.extensions.insert(LogoffRequested);
        c.world_name = Some("Castle_CellBlock".into());
        // The full exit already unlisted the character.
        c.listed_online = false;
        // Silent for 5 s. `checked_sub` fails only on a host up under 5 s.
        *c.last_recv.lock().unwrap() = Instant::now()
            .checked_sub(std::time::Duration::from_secs(5))
            .expect("host uptime must exceed 5 s to backdate last_recv");
    }

    destroy_client_entities(
        &connected,
        &entity_manager,
        addr,
        &None,
        &entity_to_addr,
        &transport(),
        &None,
        "inactivity_timeout",
    );

    let ended = capture
        .find_message(Level::INFO, "player session ended")
        .unwrap_or_else(|| panic!("no session.end line: {:#?}", capture.all()));
    assert!(
        ended.has_field("disconnect_reason", "inactivity_timeout"),
        "{ended:#?}"
    );
    assert!(ended.has_field("logoff_requested", "true"), "{ended:#?}");
    let idle_ms: u64 = ended
        .fields
        .get("idle_ms")
        .unwrap_or_else(|| panic!("no idle_ms: {ended:#?}"))
        .parse()
        .expect("idle_ms is a number");
    assert!(idle_ms >= 5_000, "idle_ms {idle_ms} must cover the 5 s");
    assert!(ended.has_field("in_world", "false"), "{ended:#?}");
    assert!(ended.has_field("world", "Castle_CellBlock"), "{ended:#?}");

    let cleaned = capture
        .find_message(Level::INFO, "Client entities cleaned up")
        .expect("cleanup line");
    assert!(
        cleaned.has_field("player_id", &PLAYER_ID.to_string()),
        "{cleaned:#?}"
    );
    assert!(cleaned.has_field("player_name", "Teal'c"), "{cleaned:#?}");
    assert!(
        cleaned.has_field("player_entity_id", &PLAYER_EID.to_string()),
        "{cleaned:#?}"
    );
}

/// Without the marker (a plain disconnect), `logoff_requested` is `false`
/// and a listed character reads `in_world=true`.
#[tokio::test]
async fn session_end_without_logoff_marker_reads_false() {
    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:52405".parse().unwrap();
    let (connected, entity_manager, entity_to_addr) = maps(addr);
    connected
        .lock()
        .unwrap()
        .get_mut(&addr)
        .unwrap()
        .listed_online = true;

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
    assert!(ended.has_field("logoff_requested", "false"), "{ended:#?}");
    assert!(ended.has_field("in_world", "true"), "{ended:#?}");
    assert!(
        !ended.fields.contains_key("world"),
        "no world is left off: {ended:#?}"
    );
}

/// TG-MER-07: `session.end` carries the session's Mercury channel health,
/// snapshotted before `clients.remove` drops the channel. Two reliable
/// packets the client never acked read `tx_outstanding=2` with an oldest
/// age; the lifetime counters ride along as numbers; with no clean RTT
/// sample yet, `srtt_ms` is left off. Reverting the snapshot drops the
/// fields from the row and fails here.
#[test]
fn session_end_carries_channel_health() {
    let capture = LogCapture::install();
    let addr: SocketAddr = "127.0.0.1:52406".parse().unwrap();
    let (connected, entity_manager, entity_to_addr) = maps(addr);
    {
        let clients = connected.lock().unwrap();
        let mut ch = clients[&addr].channel.lock().unwrap();
        for seq in [1u32, 2] {
            let packet =
                cimmeria_mercury::packet::Packet::new(Default::default(), seq, Default::default());
            ch.register_sent_packet(packet, Default::default()).unwrap();
        }
    }

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
    assert!(ended.has_field("tx_outstanding", "2"), "{ended:#?}");
    for numeric in [
        "oldest_unacked_age_ms",
        "last_rx_age_ms",
        "retransmits_total",
        "tx_holes",
        "tx_hole_stalls",
        "rx_stalls",
    ] {
        let raw = ended
            .fields
            .get(numeric)
            .unwrap_or_else(|| panic!("no {numeric}: {ended:#?}"));
        raw.parse::<u64>()
            .unwrap_or_else(|_| panic!("{numeric} is a number, got {raw:?}"));
    }
    assert!(ended.has_field("retransmits_total", "0"), "{ended:#?}");
    assert!(
        !ended.fields.contains_key("srtt_ms"),
        "no RTT sample yet leaves srtt_ms off: {ended:#?}"
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
    // The loop tears down only the session it owns: hand it that
    // session's own cancel flag, as `handle_login` does.
    let cancelled = Arc::clone(&connected.lock().unwrap()[&addr].cancelled);
    {
        // TG-NET-14: the character is in the world with two reliable packets
        // the client never acked, so the timeout line has state to name.
        let mut clients = connected.lock().unwrap();
        let state = clients.get_mut(&addr).unwrap();
        state.listed_online = true;
        state.world_name = Some("Castle_CellBlock".into());
        let mut ch = state.channel.lock().unwrap();
        for seq in [1u32, 2] {
            let packet =
                cimmeria_mercury::packet::Packet::new(Default::default(), seq, Default::default());
            ch.register_sent_packet(packet, Default::default()).unwrap();
        }
    }

    crate::base::tick_sync::run_tick_loop(
        transport(),
        addr,
        [0u8; 32],
        Default::default(),
        Arc::new(std::sync::atomic::AtomicU32::new(0)),
        Arc::new(Mutex::new(Vec::new())),
        Arc::new(Mutex::new(long_ago)),
        cancelled,
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
    assert!(timeout.has_field("in_world", "true"), "{timeout:#?}");
    assert!(
        timeout.has_field("world", "Castle_CellBlock"),
        "{timeout:#?}"
    );
    assert!(timeout.has_field("tx_outstanding", "2"), "{timeout:#?}");
    assert!(timeout.fields.contains_key("session_secs"), "{timeout:#?}");
    assert!(
        capture
            .find_message(Level::INFO, "player session ended")
            .is_some_and(|e| e.has_field("disconnect_reason", "inactivity_timeout")),
        "the timeout still tears the session down"
    );
}
