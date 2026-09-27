//! SS-00: `logOff` takes the character out of the online name index on both
//! variants. Return-to-character-select clears the name; a full exit keeps
//! the session until the client's disconnect reaps it, and the character
//! must not be reachable by tells or duel challenges in that window.

use super::super::*;
use crate::test_support::{test_default_connected_client_state, LogCapture, TestTransport};
use cimmeria_base_session::base::player_index::{lookup_online, NameLookup, OnlinePlayer};

async fn log_off(disconnect: u8) -> Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> {
    let addr: SocketAddr = "127.0.0.1:54700".parse().unwrap();
    let other: SocketAddr = "127.0.0.1:54701".parse().unwrap();
    let entity_id: u32 = 4242;

    let mut leaving = test_default_connected_client_state();
    leaving.player_entity_id = Some(entity_id);
    leaving.player_name = Some("Lomiada".to_string());
    leaving.active_player_id = Some(7);
    leaving.listed_online = true;
    let mut staying = test_default_connected_client_state();
    staying.player_entity_id = Some(5000);
    staying.player_name = Some("Teal".to_string());
    staying.active_player_id = Some(8);
    staying.listed_online = true;
    let connected = Arc::new(Mutex::new(HashMap::from([
        (addr, leaving),
        (other, staying),
    ])));
    assert_eq!(
        lookup_online(&connected, "Lomiada"),
        NameLookup::Found(OnlinePlayer { addr, player_id: 7 })
    );

    let transport: Arc<dyn Transport> = Arc::new(TestTransport::default());
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let (tx, _rx) = mpsc::channel::<BaseToCellMsg>(8);
    let capture = LogCapture::install();

    dispatch_sgw_player_base_method(
        sgw_player_base::LOG_OFF,
        &[disconnect],
        &Some("Lomiada".to_string()),
        addr,
        &transport,
        [0u8; 32],
        &connected,
        &entity_manager,
        &Some(tx),
        &entity_to_addr,
        &None,
    )
    .await
    .expect("logOff must not fail");

    assert_eq!(
        lookup_online(&connected, "Teal"),
        NameLookup::Found(OnlinePlayer {
            addr: other,
            player_id: 8
        }),
        "another player's listing is untouched"
    );
    // Telemetry: one DEBUG `online_index.remove` naming the logoff variant.
    let path = if disconnect != 0 {
        "logoff_full_exit"
    } else {
        "logoff_character_select"
    };
    let removed: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.target == "online_index" && c.has_field("event", "online_index.remove"))
        .collect();
    assert_eq!(removed.len(), 1);
    assert!(removed[0].has_field("path", path));
    assert!(removed[0].has_field("player_id", "7"));
    connected
}

#[tokio::test]
async fn logoff_to_character_select_unlists_the_character() {
    let connected = log_off(0).await;
    assert_eq!(lookup_online(&connected, "Lomiada"), NameLookup::NotFound);
    assert_eq!(lookup_online(&connected, "lomiada"), NameLookup::NotFound);
}

#[tokio::test]
async fn logoff_full_exit_unlists_the_character_before_the_session_is_reaped() {
    let connected = log_off(1).await;
    {
        let clients = connected.lock().unwrap();
        let s = &clients[&"127.0.0.1:54700".parse::<SocketAddr>().unwrap()];
        assert_eq!(
            s.player_name.as_deref(),
            Some("Lomiada"),
            "precondition: a full exit keeps the session and its name until the reap"
        );
    }
    assert_eq!(
        lookup_online(&connected, "Lomiada"),
        NameLookup::NotFound,
        "a character that has left the world must not resolve"
    );
}
