//! Guards for per-ticket admission at the listener (#1119): one connection
//! per valid ticket, decided before any byte is read.
//!
//! These run over a real loopback listener ([`accept::serve`]), because the
//! behaviour under test is which sockets the accept loop closes. A client
//! that was admitted answers a `verChk`; one that was refused sees EOF.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use tokio::io::AsyncReadExt;
use tracing::Level;

use super::listener_tests::assert_no_warn_or_error;
use super::*;
use crate::minigame::session::AdmissionGuard;
use crate::test_support::LogCapture;

const LOOPBACK: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Register one pending session whose client address is loopback.
async fn register_loopback(registry: &SessionRegistry, entity_id: u32) {
    registry
        .register(
            entity_id,
            7,
            "Hack".into(),
            1,
            1,
            0,
            0,
            0,
            1,
            vec![],
            vec![],
            None,
            Some(LOOPBACK),
            1,
        )
        .await
        .expect("register loopback session");
}

/// A ticket slot for driving [`handle_connection`] without the accept loop:
/// taken from a throwaway registry that holds one loopback session.
pub(super) async fn test_permit(peer: SocketAddr) -> AdmissionGuard {
    let registry = SessionRegistry::new();
    register_loopback(&registry, 1).await;
    registry
        .try_admit(peer.ip())
        .await
        .expect("a registry with one loopback session admits the first connection")
}

/// Serve `registry` on an ephemeral loopback port.
async fn spawn_server(registry: SessionRegistry, limits: ListenerLimits) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback");
    let addr = listener.local_addr().expect("local_addr");
    let (tx, rx) = mpsc::channel(16);
    tokio::spawn(async move {
        // Keep the receiver alive as long as the server.
        let _rx = rx;
        accept::serve(listener, 9339, registry, tx, limits).await;
    });
    addr
}

/// `true` if the server answers a `verChk` (the connection was admitted),
/// `false` if it closed the socket without a byte. A refused socket is
/// closed before the server reads, so the write may itself fail.
async fn admitted(client: &mut TcpStream) -> bool {
    let _ = send_null_terminated(
        client,
        "<msg t='sys'><body action='verChk' r='0'><ver v='154'/></body></msg>",
    )
    .await;
    let mut buf = [0u8; 256];
    match tokio::time::timeout(Duration::from_secs(5), client.read(&mut buf)).await {
        Ok(Ok(n)) => n > 0,
        Ok(Err(_)) => false,
        Err(_) => panic!("neither a reply nor a close within 5 s"),
    }
}

/// Two simultaneous connects for one ticket: exactly one is admitted. Under
/// the old 8-per-IP cap both were, so a second SWF could run a second game
/// against the same ticket's victory chains.
#[tokio::test]
async fn two_connections_for_one_ticket_admit_one() {
    let registry = SessionRegistry::new();
    register_loopback(&registry, 4401).await;
    let addr = spawn_server(registry, ListenerLimits::default()).await;

    let (a, b) = tokio::join!(TcpStream::connect(addr), TcpStream::connect(addr));
    let (mut a, mut b) = (a.expect("connect"), b.expect("connect"));
    let (a_in, b_in) = tokio::join!(admitted(&mut a), admitted(&mut b));

    assert_eq!(
        [a_in, b_in].iter().filter(|x| **x).count(),
        1,
        "one ticket admits one connection (a: {a_in}, b: {b_in})",
    );
}

/// A peer that connects and never finishes the handshake gives its slot
/// back when the handshake deadline closes it, so the player can retry.
#[tokio::test]
async fn failed_handshake_frees_the_ticket_slot() {
    let registry = SessionRegistry::new();
    register_loopback(&registry, 4402).await;
    let addr = spawn_server(
        registry,
        ListenerLimits {
            handshake_timeout: Duration::from_millis(150),
            ..ListenerLimits::default()
        },
    )
    .await;

    // Holds the only slot and says nothing until the deadline closes it.
    let mut silent = TcpStream::connect(addr).await.expect("connect");
    let mut byte = [0u8; 1];
    let n = tokio::time::timeout(Duration::from_secs(5), silent.read(&mut byte))
        .await
        .expect("the handshake deadline must close the silent peer")
        .unwrap_or(0);
    assert_eq!(n, 0);

    // The slot is released as the task ends, which can trail the socket close
    // by a scheduler turn, so retry briefly.
    let mut readmitted = false;
    for _ in 0..50 {
        let mut again = TcpStream::connect(addr).await.expect("connect");
        if admitted(&mut again).await {
            readmitted = true;
            break;
        }
    }
    assert!(readmitted, "a failed handshake must free the ticket slot");
}

/// Three tickets from one address (a household) admit three connections and
/// refuse the fourth.
#[tokio::test]
async fn household_of_three_tickets_admits_three() {
    let registry = SessionRegistry::new();
    for entity_id in [4403, 4404, 4405] {
        register_loopback(&registry, entity_id).await;
    }
    let addr = spawn_server(registry, ListenerLimits::default()).await;

    let mut held = Vec::new();
    for n in 1..=3 {
        let mut client = TcpStream::connect(addr).await.expect("connect");
        assert!(admitted(&mut client).await, "connection {n} of 3 admitted");
        held.push(client);
    }
    let mut fourth = TcpStream::connect(addr).await.expect("connect");
    assert!(
        !admitted(&mut fourth).await,
        "a fourth connection has no ticket"
    );
}

/// The refusal for a peer whose tickets are all in use is INFO, never WARN
/// or ERROR (only those reach Discord), and names its reason.
#[tokio::test]
async fn admission_refusals_log_info_not_warn() {
    let capture = LogCapture::install();
    let registry = SessionRegistry::new();
    register_loopback(&registry, 4406).await;
    let addr = spawn_server(registry, ListenerLimits::default()).await;

    let mut first = TcpStream::connect(addr).await.expect("connect");
    assert!(admitted(&mut first).await);
    let mut second = TcpStream::connect(addr).await.expect("connect");
    assert!(!admitted(&mut second).await);

    let row = capture
        .find_event(Level::INFO, "no free ticket", "no_free_ticket")
        .unwrap_or_else(|| panic!("no no_free_ticket INFO row: {:#?}", capture.all()));
    assert!(row.fields.contains_key("peer"), "{row:#?}");
    assert_no_warn_or_error(&capture);
}
