//! Tests for the `handle_cell_message` dispatch.

use super::*;

fn empty_maps() -> (
    Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    (
        Arc::new(Mutex::new(HashMap::new())),
        Arc::new(Mutex::new(HashMap::new())),
    )
}

#[tokio::test]
async fn minigame_result_forwards_to_cell_service() {
    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let (connected, entity_to_addr) = empty_maps();
    let (cell_tx, mut cell_rx) = mpsc::channel(1);

    handle_cell_message(
        CellToBaseMsg::MinigameResult {
            entity_id: 10,
            result_code: 2,
            on_victory_chains: vec![100, 200],
        },
        &socket,
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    match cell_rx.try_recv().expect("minigame result forwarded") {
        BaseToCellMsg::MinigameResult {
            entity_id,
            result_code,
            on_victory_chains,
        } => {
            assert_eq!(entity_id, 10);
            assert_eq!(result_code, 2);
            assert_eq!(on_victory_chains, vec![100, 200]);
        }
        // BaseToCellMsg deliberately omits Debug (oneshot::Sender),
        // so we can't print the variant — name the expected one
        // and let test output point at this line.
        _ => panic!("expected BaseToCellMsg::MinigameResult"),
    }
}

#[tokio::test]
async fn invalid_bandolier_ammo_update_drops_before_side_effects() {
    let socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
    let (connected, entity_to_addr) = empty_maps();
    let (cell_tx, mut cell_rx) = mpsc::channel(1);

    handle_cell_message(
        CellToBaseMsg::BandolierAmmoUpdate {
            player_id: 10,
            slot_id: -1,
            expected_item_id: 42,
            current_ammo: 17,
            cur_ammo_type: 1,
        },
        &socket,
        &connected,
        &entity_to_addr,
        &Some(cell_tx),
        &None,
        &None,
        "127.0.0.1",
        7777,
    )
    .await;

    assert!(
        cell_rx.try_recv().is_err(),
        "invalid payload must not forward"
    );
    assert!(connected.lock().unwrap().is_empty());
    assert!(entity_to_addr.lock().unwrap().is_empty());
}
