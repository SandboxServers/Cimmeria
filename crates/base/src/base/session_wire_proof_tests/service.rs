//! Real Base service login and restart proof fixtures.

use super::*;
use crate::auth::PendingLogin;
use crate::base::service::BaseService;
use cimmeria_common::ServerConfig;
use tokio::net::UdpSocket;

fn issue_login(pending: &Arc<Mutex<HashMap<String, PendingLogin>>>, account: u32) -> String {
    let ticket = format!("TICKET{account:014}");
    pending.lock().unwrap().insert(
        ticket.clone(),
        PendingLogin {
            account_id: account,
            account_name: format!("account{account}"),
            access_level: 0,
            test_account: false,
            ticket: ticket.clone(),
            session_key: "42".repeat(32),
            client_ip: "127.0.0.1".parse().unwrap(),
            created: Instant::now(),
            ticket_guard: None,
        },
    );
    ticket
}

fn login_datagram(ticket: &str, account: u32) -> Vec<u8> {
    let mut raw = vec![0x41, 0x00];
    raw.extend_from_slice(&25u16.to_le_bytes());
    raw.extend_from_slice(&1u32.to_le_bytes());
    raw.extend_from_slice(&0u16.to_le_bytes());
    raw.extend_from_slice(&account.to_le_bytes());
    raw.push(ticket.len() as u8);
    raw.extend_from_slice(ticket.as_bytes());
    raw.extend_from_slice(&1u16.to_le_bytes());
    raw.extend_from_slice(&1u32.to_le_bytes());
    raw
}

async fn await_sessions(service: &BaseService, count: usize) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if service.connected_clients.lock().unwrap().len() == count {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("real Base receive loop did not register sessions");
}

#[tokio::test]
async fn session_wire_proof_service_restart_and_overlapping_phase3_logins() {
    let config = ServerConfig {
        base_port: 0,
        ..ServerConfig::loopback()
    };
    let pending = Arc::new(Mutex::new(HashMap::new()));
    let mut first_service = BaseService::new(&config);
    first_service.set_pending_logins(pending.clone());
    first_service.start().await.unwrap();
    let first_server = first_service
        .proof_transport
        .as_ref()
        .unwrap()
        .local_addr()
        .unwrap();
    let a_client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let b_client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let a_peer = a_client.local_addr().unwrap();
    let b_peer = b_client.local_addr().unwrap();
    let a_ticket = issue_login(&pending, 7001);
    let b_ticket = issue_login(&pending, 7002);
    let a_login = login_datagram(&a_ticket, 7001);
    let b_login = login_datagram(&b_ticket, 7002);
    let (a_sent, b_sent) = tokio::join!(
        a_client.send_to(&a_login, first_server),
        b_client.send_to(&b_login, first_server),
    );
    a_sent.unwrap();
    b_sent.unwrap();
    await_sessions(&first_service, 2).await;
    let (a, b) = {
        let sessions = first_service.connected_clients.lock().unwrap();
        (
            sessions[&a_peer].wire_send_origin().unwrap(),
            sessions[&b_peer].wire_send_origin().unwrap(),
        )
    };
    assert_eq!(a.server_boot_id, b.server_boot_id);
    assert_ne!(a.session_generation, b.session_generation);
    assert_eq!(a.player_entity_id, None);
    let proof = first_service.proof_transport.as_ref().unwrap();
    proof
        .send_to_with_origin(b"active", a_peer, &a)
        .await
        .unwrap();
    let snapshot = proof.snapshot();
    assert_eq!(snapshot.gaps.pre_phase3_exempt, 4);
    assert!(snapshot
        .rows
        .iter()
        .any(|row| row.session_generation == a.session_generation));

    for session in first_service.connected_clients.lock().unwrap().values() {
        session.cancelled.store(true, Ordering::Relaxed);
    }
    first_service.stop().await;

    let restart_pending = Arc::new(Mutex::new(HashMap::new()));
    let mut restarted = BaseService::new(&config);
    restarted.set_pending_logins(restart_pending.clone());
    restarted.start().await.unwrap();
    let second_server = restarted
        .proof_transport
        .as_ref()
        .unwrap()
        .local_addr()
        .unwrap();
    let restart_ticket = issue_login(&restart_pending, 7003);
    a_client
        .send_to(&login_datagram(&restart_ticket, 7003), second_server)
        .await
        .unwrap();
    await_sessions(&restarted, 1).await;
    let restart_origin = restarted.connected_clients.lock().unwrap()[&a_peer]
        .wire_send_origin()
        .unwrap();
    assert_ne!(a.server_boot_id, restart_origin.server_boot_id);
    assert_ne!(a.session_generation, restart_origin.session_generation);
    restarted.connected_clients.lock().unwrap()[&a_peer]
        .cancelled
        .store(true, Ordering::Relaxed);
}
