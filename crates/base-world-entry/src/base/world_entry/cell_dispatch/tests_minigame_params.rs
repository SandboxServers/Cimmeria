//! MG-02 / Contract C3: `StartMinigame` registers the cell-supplied player params.

use super::*;
use crate::test_support::{LogCapture, TestTransport};

/// MG-02 / Contract C3: the player's level and Intelligence ride the
/// `StartMinigame` message into the registered session, so the SWF's `joinOK`
/// reports them instead of the old hard-coded 1 and 0.
/// Revert: register with literals again and the session holds 1 and 0.
#[tokio::test]
async fn start_minigame_registers_player_params_and_failure_chains() {
    use crate::minigame::SessionRegistry;
    use crate::test_support::test_default_connected_client_state;

    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let addr: SocketAddr = "10.0.0.5:5000".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::from([(
        addr,
        test_default_connected_client_state(),
    )])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(10u32, addr)])));
    let registry = SessionRegistry::new();

    handle_cell_message(
        CellToBaseMsg::StartMinigame {
            entity_id: 10,
            player_id: 1,
            game_name: "Livewire".into(),
            difficulty: 3,
            on_victory_chains: vec![100],
            on_failure_chains: vec![200],
            player_level: 17,
            intelligence: 9,
            tech_competency: 1,
            abilities_mask: 0,
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &Some(registry.clone()),
        "127.0.0.1",
        7777,
    )
    .await;

    let session = registry
        .peek_session(10)
        .await
        .expect("the start must register a session");
    assert_eq!(session.player_level, 17);
    assert_eq!(session.intelligence, 9);
    assert_eq!(session.difficulty, 3);
    assert_eq!(session.on_victory_chains, vec![100]);
    assert_eq!(session.on_failure_chains, vec![200]);
}



/// A disabled minigame server (`None` registry) is a WARN naming the game,
/// not a silent no-op.
#[tokio::test]
async fn start_minigame_with_disabled_server_logs_warn() {
    let capture = LogCapture::install();
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));

    handle_cell_message(
        CellToBaseMsg::StartMinigame {
            entity_id: 10,
            player_id: 1,
            game_name: "Livewire".into(),
            difficulty: 1,
            on_victory_chains: vec![],
            on_failure_chains: vec![],
            player_level: 1,
            intelligence: 0,
            tech_competency: 1,
            abilities_mask: 0,
        },
        &transport,
        &connected,
        &entity_to_addr,
        &None,
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    let event = capture
        .find_event(
            tracing::Level::WARN,
            "minigame server is disabled",
            "minigame_server_disabled",
        )
        .expect("disabled minigame server must log a WARN");
    assert!(event.has_field("game_name", "Livewire"), "{event:#?}");
}