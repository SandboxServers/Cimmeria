use super::*;
use crate::minigame::SessionRegistry;
use crate::test_support::{test_default_connected_client_state, TestTransport};
use std::future::Future;
use std::io;
use std::net::{IpAddr, Ipv4Addr};
use std::pin::Pin;

const TEST_PARAMS: MinigamePlayerParams = MinigamePlayerParams {
    player_level: 1,
    intelligence: 0,
    tech_competency: 1,
    abilities_mask: 0,
};

fn owner_state() -> ConnectedClientState {
    let mut state = test_default_connected_client_state();
    state.player_entity_id = Some(77);
    state.active_player_id = Some(1);
    state
}

/// Run `start_minigame` for entity 77 with the given address mapping and
/// return the registry it registered into.
async fn start_for_entity(addr: Option<SocketAddr>) -> SessionRegistry {
    let transport: Arc<dyn Transport> = Arc::new(TestTransport::new());
    let connected: Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let mut entity_map = HashMap::new();
    if let Some(addr) = addr {
        connected.lock().unwrap().insert(addr, owner_state());
        entity_map.insert(77u32, addr);
    }
    let entity_to_addr = Arc::new(Mutex::new(entity_map));
    let registry = SessionRegistry::new();
    let minigame_registry = Some(registry.clone());

    start_minigame(
        77,
        1,
        "Livewire".into(),
        1,
        vec![],
        vec![],
        TEST_PARAMS,
        &transport,
        &connected,
        &entity_to_addr,
        &minigame_registry,
        "203.0.113.1",
        9339,
        &None,
    )
    .await;
    registry
}

/// The session carries the game connection's IP, so the listener admits
/// that address and no other.
#[tokio::test]
async fn start_minigame_registers_the_game_connection_ip() {
    let addr: SocketAddr = "10.0.0.5:5000".parse().unwrap();
    let registry = start_for_entity(Some(addr)).await;

    assert!(
        registry
            .expects_peer(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 5)))
            .await
    );
    assert!(
        !registry
            .expects_peer(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 6)))
            .await
    );
}

/// Without a client address nothing is registered: the entity id is not
/// held for the pending TTL by a session no peer could ever claim.
#[tokio::test]
async fn start_minigame_without_client_addr_registers_nothing() {
    let registry = start_for_entity(None).await;

    assert_eq!(registry.session_count().await, 0);
}

#[tokio::test]
async fn invalid_start_ownership_registers_nothing_and_sends_no_feedback() {
    let addr: SocketAddr = "10.0.0.5:5000".parse().unwrap();
    for (entity, player, state_entity, state_player) in [
        (77, 1, None, Some(1)),
        (77, 1, Some(88), Some(1)),
        (77, 1, Some(77), Some(2)),
        (77, 0, Some(77), Some(1)),
    ] {
        let transport = Arc::new(TestTransport::new());
        let mut state = owner_state();
        state.player_entity_id = state_entity;
        state.active_player_id = state_player;
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity, addr)])));
        let registry = SessionRegistry::new();
        start_minigame(
            entity,
            player,
            "Livewire".into(),
            1,
            vec![],
            vec![],
            TEST_PARAMS,
            &(transport.clone() as Arc<dyn Transport>),
            &connected,
            &entity_to_addr,
            &Some(registry.clone()),
            "203.0.113.1",
            9339,
            &None,
        )
        .await;
        assert_eq!(registry.session_count().await, 0);
        assert!(transport.is_empty());
    }
}

struct FailedTransport;

impl Transport for FailedTransport {
    fn send_to<'a, 'b, 'f>(
        &'a self,
        _bytes: &'b [u8],
        _addr: SocketAddr,
    ) -> Pin<Box<dyn Future<Output = io::Result<usize>> + Send + 'f>>
    where
        'a: 'f,
        'b: 'f,
        Self: 'f,
    {
        Box::pin(async { Err(io::Error::other("injected URL send failure")) })
    }

    fn local_addr(&self) -> io::Result<SocketAddr> {
        Ok("127.0.0.1:0".parse().unwrap())
    }
}

#[tokio::test]
async fn failed_url_send_releases_only_its_ticket() {
    let addr: SocketAddr = "10.0.0.5:5000".parse().unwrap();
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, owner_state())])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(77, addr)])));
    let registry = SessionRegistry::new();
    let transport: Arc<dyn Transport> = Arc::new(FailedTransport);
    start_minigame(
        77,
        1,
        "Livewire".into(),
        1,
        vec![],
        vec![],
        TEST_PARAMS,
        &transport,
        &connected,
        &entity_to_addr,
        &Some(registry.clone()),
        "203.0.113.1",
        9339,
        &None,
    )
    .await;
    assert_eq!(
        registry.session_count().await,
        0,
        "no phantom ticket after failed URL"
    );
    let fresh = registry
        .register(
            77,
            1,
            "Livewire".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![],
            vec![],
            None,
            Some(addr.ip()),
            1,
        )
        .await;
    assert!(fresh.is_some(), "a retry can claim the entity");
}

/// A duplicate start cannot launch a second SWF, but must tell the
/// player why the click did not work. The content replay guard separately
/// pins that the terminal's interaction bit remains until victory.
#[tokio::test]
async fn duplicate_start_sends_feedback_without_replacing_ticket() {
    let addr: SocketAddr = "10.0.0.5:5000".parse().unwrap();
    let transport = Arc::new(TestTransport::new());
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, owner_state())])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(77u32, addr)])));
    let registry = SessionRegistry::new();
    let configured = Some(registry.clone());
    start_minigame(
        77,
        1,
        "Livewire".into(),
        1,
        vec![],
        vec![],
        TEST_PARAMS,
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entity_to_addr,
        &configured,
        "203.0.113.1",
        9339,
        &None,
    )
    .await;
    let first = transport.drain();
    assert_eq!(first.len(), 1, "first click launches Livewire");
    let plain = cimmeria_mercury::encryption::MercuryEncryption::from_session_key([0; 32])
        .decrypt(&first[0].1)
        .unwrap();
    let packet = cimmeria_mercury::packet::parse_incoming(&plain).unwrap();
    let marker = [b'h', 0, b't', 0, b't', 0, b'p', 0];
    let start = packet
        .body
        .windows(marker.len())
        .position(|w| w == marker)
        .unwrap();
    let len = u32::from_le_bytes(packet.body[start - 4..start].try_into().unwrap()) as usize;
    let units: Vec<u16> = packet.body[start..start + len * 2]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect();
    let url = String::from_utf16(&units).unwrap();
    let ticket = url.rsplit('/').next().unwrap();
    assert!(registry
        .authenticate(77, ticket, "Livewire")
        .await
        .is_some());
    let utf16: Vec<u16> = url.encode_utf16().collect();
    let mut expected_args = (utf16.len() as u32).to_le_bytes().to_vec();
    for unit in utf16 {
        expected_args.extend_from_slice(&unit.to_le_bytes());
    }
    let expected_start = build_player_entity_method_packet(
        &[0; 32],
        0,
        &[],
        77,
        crate::cell::dispatch::CLIENT_MG_ON_START_MINIGAME,
        &expected_args,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    assert_eq!(first[0].1, expected_start, "onStartMinigame wire bytes");
    start_minigame(
        77,
        1,
        "Livewire".into(),
        1,
        vec![],
        vec![],
        TEST_PARAMS,
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entity_to_addr,
        &configured,
        "203.0.113.1",
        9339,
        &None,
    )
    .await;
    assert_eq!(registry.session_count().await, 1);
    assert!(registry
        .authenticate(77, ticket, "Livewire")
        .await
        .is_some());
    let sent = transport.filter_to(addr);
    assert_eq!(sent.len(), 1, "refused click emits exactly one packet");
    let args = serialize_on_player_communication(
        "SYSTEM",
        0,
        CHAN_FEEDBACK,
        "Minigame could not start. Close any open minigame and try again.",
    );
    let expected = build_player_entity_method_packet(
        &[0u8; 32],
        1,
        &[],
        77,
        crate::mercury::method_idx::ON_PLAYER_COMMUNICATION,
        &args,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    assert_eq!(sent[0], expected, "refused click uses CHAN_FEEDBACK");
}

#[tokio::test]
async fn stale_result_does_not_end_a_reopened_board_even_if_fin_won() {
    let addr: SocketAddr = "10.0.0.5:5000".parse().unwrap();
    let transport = Arc::new(TestTransport::new());
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, owner_state())])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(77u32, addr)])));
    let (cell_tx, mut cell_rx) = mpsc::channel(2);
    let registry = SessionRegistry::new();
    let instance = connected.lock().unwrap()[&addr].session_instance_id;
    let old_ticket = registry
        .register(
            77,
            1,
            "Livewire".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![],
            vec![],
            None,
            Some(addr.ip()),
            instance,
        )
        .await
        .unwrap();
    assert!(registry.cancel(77).await);
    let new_ticket = registry
        .register(
            77,
            1,
            "Livewire".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![],
            vec![],
            None,
            Some(addr.ip()),
            instance,
        )
        .await
        .unwrap();
    assert_ne!(old_ticket, new_ticket);

    minigame_result(
        &old_ticket,
        &Some(registry.clone()),
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entity_to_addr,
        &Some(cell_tx.clone()),
    )
    .await;
    assert_eq!(
        transport.send_count_to(addr),
        0,
        "late onEndMinigame must not close the next board"
    );
    assert!(
        cell_rx.try_recv().is_err(),
        "old ticket cannot forward to Cell"
    );
    let claimed = registry
        .authenticate_and_claim(77, &new_ticket, "Livewire")
        .await
        .unwrap();
    assert!(registry.deposit_result(&claimed, 1, true).await);
    minigame_result(
        &new_ticket,
        &Some(registry),
        &(transport.clone() as Arc<dyn Transport>),
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
    )
    .await;
    assert_eq!(
        transport.send_count_to(addr),
        1,
        "current result still ends its board"
    );
    let expected = build_player_entity_method_packet(
        &[0u8; 32],
        0,
        &[],
        77,
        crate::cell::dispatch::CLIENT_MG_ON_END_MINIGAME,
        &[],
        cimmeria_mercury::encryption::EncryptionVersion::V1,
    );
    assert_eq!(
        transport.filter_to(addr),
        [expected],
        "onEndMinigame wire bytes"
    );
}
