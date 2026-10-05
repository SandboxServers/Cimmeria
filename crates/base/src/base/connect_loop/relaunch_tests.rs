//! Guards for a client relaunched on the address:port of its own live
//! session (`login::relaunch`, `login::eviction`).
//!
//! Colo, release v2026-10-05.1 (DA-06 lab run): a client killed and
//! relaunched within the 60 s inactivity window came back on the same
//! address:port (the SGW client binds a fixed UDP port). Its plaintext
//! `baseAppLogin` was routed to the dead session's channel, dropped as
//! "retrying baseAppLogin on an established channel", and the player sat
//! at "Logging in..." until the old channel timed out.
//!
//! Every datagram here goes through `handle_datagram`, the routing seam
//! the bug lived in, and the first session is created by a real login
//! through the same seam.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tokio::sync::mpsc;
use tracing::Level;

use cimmeria_base_session::base::plugin::BasePlugins;
use cimmeria_entity::manager::EntityManager;
use cimmeria_mercury::encryption::{EncryptionVersion, MercuryEncryption};
use cimmeria_mercury::test_transport::TestTransport;
use cimmeria_mercury::transport::Transport;

use crate::auth::PendingLogin;
use crate::base::login::relaunch::REASON_RELAUNCH_ACCOUNT_MISMATCH;
use crate::base::ConnectedClientState;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::LogCapture;

use super::handle_datagram;

const ACCOUNT_ID: u32 = 0x5EED_0A01;
const OTHER_ACCOUNT_ID: u32 = 0x5EED_0A02;
const PLAYER_ID: i32 = 0x0A01;
const PLAYER_EID: u32 = 0x5EED_0A03;
const KEY_A: [u8; 32] = [0xA1; 32];
const KEY_B: [u8; 32] = [0xB2; 32];
const TICKET_A: &str = "RELAUNCHA00000000001";
const TICKET_B: &str = "RELAUNCHB00000000002";

type Connected = Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>;

/// The client's plaintext `baseAppLogin` datagram, as the colo saw it
/// (41 bytes, the `decrypt_reject` tests pin the same shape).
fn base_app_login(ticket: &str, request_id: u32, seq: u32) -> Vec<u8> {
    assert_eq!(ticket.len(), 20);
    let mut raw = vec![0x41, 0x00];
    raw.extend_from_slice(&25u16.to_le_bytes());
    raw.extend_from_slice(&request_id.to_le_bytes());
    raw.extend_from_slice(&0u16.to_le_bytes()); // next request offset
    raw.extend_from_slice(&ACCOUNT_ID.to_le_bytes());
    raw.push(20);
    raw.extend_from_slice(ticket.as_bytes());
    raw.extend_from_slice(&1u16.to_le_bytes()); // first request offset
    raw.extend_from_slice(&seq.to_le_bytes());
    raw
}

fn hex_key(key: [u8; 32]) -> String {
    key.iter().map(|b| format!("{b:02X}")).collect()
}

fn pending(ticket: &str, account_id: u32, key: [u8; 32]) -> PendingLogin {
    PendingLogin {
        account_id,
        account_name: format!("acct{account_id:x}"),
        access_level: 0,
        ticket: ticket.to_string(),
        session_key: hex_key(key),
        client_ip: "127.0.0.1".parse().unwrap(),
        created: Instant::now(),
    }
}

struct Rig {
    addr: SocketAddr,
    transport: Arc<TestTransport>,
    pending_logins: Arc<Mutex<HashMap<String, PendingLogin>>>,
    connected: Connected,
    entity_manager: Arc<Mutex<EntityManager>>,
    entity_to_addr: Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: Option<mpsc::Sender<BaseToCellMsg>>,
    cell_rx: mpsc::Receiver<BaseToCellMsg>,
}

impl Rig {
    fn new(port: u16) -> Self {
        let (tx, rx) = mpsc::channel(16);
        Self {
            addr: SocketAddr::from(([127, 0, 0, 1], port)),
            transport: Arc::new(TestTransport::new()),
            pending_logins: Arc::new(Mutex::new(HashMap::new())),
            connected: Arc::new(Mutex::new(HashMap::new())),
            entity_manager: Arc::new(Mutex::new(EntityManager::new())),
            entity_to_addr: Arc::new(Mutex::new(HashMap::new())),
            cell_tx: Some(tx),
            cell_rx: rx,
        }
    }

    async fn deliver(&self, raw: &[u8]) {
        let transport: Arc<dyn Transport> = self.transport.clone();
        handle_datagram(
            &transport,
            self.addr,
            raw,
            &self.pending_logins,
            &self.connected,
            &None,
            &None,
            &self.entity_manager,
            &self.cell_tx,
            &self.entity_to_addr,
            EncryptionVersion::V1,
            &BasePlugins::empty(),
        )
        .await
        .unwrap();
    }

    fn issue(&self, login: PendingLogin) {
        self.pending_logins
            .lock()
            .unwrap()
            .insert(login.ticket.clone(), login);
    }

    /// Log in with ticket A through the real path, then put a character in
    /// the world on that session, as a crashed in-world player would have.
    async fn established_in_world_session(&self) -> Arc<AtomicBool> {
        self.issue(pending(TICKET_A, ACCOUNT_ID, KEY_A));
        self.deliver(&base_app_login(TICKET_A, 1, 1)).await;
        let mut clients = self.connected.lock().unwrap();
        let s = clients
            .get_mut(&self.addr)
            .expect("ticket A must register a session");
        assert_eq!(s.key, KEY_A);
        s.player_entity_id = Some(PLAYER_EID);
        s.active_player_id = Some(PLAYER_ID);
        s.player_name = Some("Vala".into());
        self.entity_to_addr
            .lock()
            .unwrap()
            .insert(PLAYER_EID, self.addr);
        self.transport.clear();
        Arc::clone(&s.cancelled)
    }

    fn session_key(&self) -> Option<[u8; 32]> {
        self.connected
            .lock()
            .unwrap()
            .get(&self.addr)
            .map(|s| s.key)
    }

    fn session_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.connected.lock().unwrap()[&self.addr].cancelled)
    }

    /// Stop the tick-sync loop of whatever session holds the address.
    fn stop(&self) {
        if let Some(s) = self.connected.lock().unwrap().get(&self.addr) {
            s.cancelled.store(true, Ordering::Relaxed);
        }
    }
}

/// The bug: an in-world session on an address, the client restarts and
/// logs in again with a new ticket from the same address:port. The new
/// login must be accepted with its own key, the old session evicted (tick
/// loop cancelled, cell told to disconnect the old player, which persists
/// it), and one takeover row written with the names. Reverting the routing
/// in `handle_datagram` drops the login as a retry and fails every check.
#[tokio::test]
async fn relaunch_with_a_new_ticket_takes_over_the_address_and_evicts_the_old_session() {
    let mut rig = Rig::new(52611);
    let old_flag = rig.established_in_world_session().await;
    let capture = LogCapture::install();

    rig.issue(pending(TICKET_B, ACCOUNT_ID, KEY_B));
    rig.deliver(&base_app_login(TICKET_B, 7, 1)).await;

    assert_eq!(
        rig.session_key(),
        Some(KEY_B),
        "the relaunched client's session must hold its own new key"
    );
    assert!(
        !Arc::ptr_eq(&old_flag, &rig.session_flag()),
        "the address must hold a new session, not the old one"
    );
    assert!(
        old_flag.load(Ordering::Relaxed),
        "the old session's tick-sync loop must be cancelled, or it keeps \
         encrypting toward the new client with the old key"
    );
    assert!(
        !rig.pending_logins.lock().unwrap().contains_key(TICKET_B),
        "the new ticket is consumed"
    );
    assert!(
        !rig.entity_to_addr.lock().unwrap().contains_key(&PLAYER_EID),
        "the old player entity no longer routes to this address"
    );

    // The old player goes through the cell's disconnect (which persists it).
    let msg = tokio::time::timeout(Duration::from_secs(2), rig.cell_rx.recv())
        .await
        .expect("the old player's DisconnectEntity must reach the cell")
        .expect("cell channel open");
    match msg {
        BaseToCellMsg::DisconnectEntity {
            entity_id,
            reply_tx,
        } => {
            assert_eq!(entity_id, PLAYER_EID);
            let _ = reply_tx.send(());
        }
        _ => panic!("expected DisconnectEntity for the old player"),
    }

    // Everything sent to the address after the takeover is under the new
    // key: the reply decrypts with KEY_B and not with KEY_A.
    let sent = rig.transport.filter_to(rig.addr);
    let reply = sent.first().expect("the new login must be answered");
    assert!(MercuryEncryption::from_session_key(KEY_B)
        .decrypt(reply)
        .is_ok());
    assert!(MercuryEncryption::from_session_key(KEY_A)
        .decrypt(reply)
        .is_err());
    assert!(
        sent.iter()
            .all(|p| MercuryEncryption::from_session_key(KEY_A)
                .decrypt(p)
                .is_err()),
        "no LOGGED_OFF under the old key goes to the address: the old client is dead \
         and the new one would drop it as corrupted"
    );

    let row = capture
        .find_event(Level::WARN, "relaunched", "relaunch_takeover")
        .unwrap_or_else(|| panic!("no takeover row; saw {:#?}", capture.all()));
    assert!(row.has_field("account_id", &ACCOUNT_ID.to_string()));
    assert!(row.has_field("account_name", &format!("acct{ACCOUNT_ID:x}")));
    assert!(row.has_field("player_id", &PLAYER_ID.to_string()));
    assert!(row.has_field("player_name", "Vala"), "{row:#?}");
    assert!(
        capture
            .find_message(Level::INFO, "player session ended")
            .is_some_and(|e| e.has_field("disconnect_reason", "relaunch_takeover")),
        "the old session ends with disconnect_reason = relaunch_takeover"
    );

    rig.stop();
}

/// A retransmit of the login that created the channel (its ticket is
/// consumed) keeps today's behaviour: the session is untouched and the
/// datagram is logged as a retry.
#[tokio::test]
async fn retransmit_of_the_original_login_keeps_the_session() {
    let rig = Rig::new(52612);
    let old_flag = rig.established_in_world_session().await;
    let capture = LogCapture::install();

    rig.deliver(&base_app_login(TICKET_A, 1, 1)).await;
    rig.deliver(&base_app_login(TICKET_A, 1, 1)).await;

    assert_eq!(rig.session_key(), Some(KEY_A));
    assert!(Arc::ptr_eq(&old_flag, &rig.session_flag()));
    assert!(!old_flag.load(Ordering::Relaxed), "no teardown");
    assert!(
        capture
            .find_event(
                Level::WARN,
                "retrying baseAppLogin",
                "login_retry_on_channel"
            )
            .is_some(),
        "a real retransmit keeps its retry row; saw {:#?}",
        capture.all()
    );
    assert!(capture
        .find_event(Level::WARN, "", "relaunch_takeover")
        .is_none());
    assert!(
        rig.transport.filter_to(rig.addr).is_empty(),
        "a retransmit is not answered again"
    );

    rig.stop();
}

/// Garbage and a well-formed login with an unknown ticket on the
/// established channel leave the session up and do not refresh its
/// inactivity clock (a spoofer must not keep a dead session alive).
#[tokio::test]
async fn unauthenticated_datagrams_on_the_channel_neither_tear_down_nor_keep_alive() {
    let rig = Rig::new(52613);
    let old_flag = rig.established_in_world_session().await;
    let long_ago = Instant::now()
        .checked_sub(Duration::from_secs(30))
        .expect("host uptime must exceed 30 s");
    let last_recv = Arc::clone(&rig.connected.lock().unwrap()[&rig.addr].last_recv);
    *last_recv.lock().unwrap() = long_ago;

    rig.deliver(&[0x5Au8; 48]).await; // block-aligned, fails the HMAC
    rig.deliver(&base_app_login("FORGED00000000000000", 9, 1))
        .await; // never issued

    assert_eq!(rig.session_key(), Some(KEY_A));
    assert!(Arc::ptr_eq(&old_flag, &rig.session_flag()));
    assert!(!old_flag.load(Ordering::Relaxed), "no teardown");
    assert_eq!(
        *last_recv.lock().unwrap(),
        long_ago,
        "an undecryptable datagram must not refresh last_recv"
    );

    rig.stop();
}

/// A fresh, valid ticket for a *different* account on a live player's
/// address is the spoofed-source attack (any account holder can get a
/// ticket for their own account). It is refused: the live session stays,
/// the ticket is burned, and one refusal row is written.
#[tokio::test]
async fn fresh_ticket_for_another_account_is_refused_and_burned() {
    let rig = Rig::new(52614);
    let old_flag = rig.established_in_world_session().await;
    let capture = LogCapture::install();

    rig.issue(pending(TICKET_B, OTHER_ACCOUNT_ID, KEY_B));
    rig.deliver(&base_app_login(TICKET_B, 7, 1)).await;
    rig.deliver(&base_app_login(TICKET_B, 7, 1)).await;

    assert_eq!(
        rig.session_key(),
        Some(KEY_A),
        "the live session keeps its key"
    );
    assert!(Arc::ptr_eq(&old_flag, &rig.session_flag()));
    assert!(!old_flag.load(Ordering::Relaxed), "no teardown");
    assert!(
        !rig.pending_logins.lock().unwrap().contains_key(TICKET_B),
        "the refused ticket is burned"
    );
    let refusals: Vec<_> = capture
        .all()
        .into_iter()
        .filter(|c| c.has_field("reason", REASON_RELAUNCH_ACCOUNT_MISMATCH))
        .collect();
    assert_eq!(
        refusals.len(),
        1,
        "one refusal row per ticket: {refusals:#?}"
    );
    assert!(refusals[0].has_field("ticket_account_id", &OTHER_ACCOUNT_ID.to_string()));
    assert!(refusals[0].has_field("player_name", "Vala"));
    assert!(
        rig.transport.filter_to(rig.addr).is_empty(),
        "nothing answered"
    );

    rig.stop();
}

/// PR #1246 review, finding 1: an attacker spoofs the victim's
/// address:port while it is free and registers there with a ticket for
/// their own account. That ticket was issued (over TCP, unspoofable) to the
/// attacker's real IP, so the session registers with a mismatched ticket IP.
/// The victim's own login, whose ticket was issued to this address's IP,
/// must evict the squatter instead of being refused and burned. Another
/// account whose ticket is also mismatched must not. Fails with rule 4
/// removed from `relaunch::address_claim`.
#[tokio::test]
async fn a_victims_login_evicts_a_squatter_that_registered_with_a_foreign_ticket_ip() {
    let rig = Rig::new(52615);
    let mut squat = pending(TICKET_B, OTHER_ACCOUNT_ID, KEY_B);
    squat.client_ip = "198.51.100.66".parse().unwrap();
    rig.issue(squat);
    rig.deliver(&base_app_login(TICKET_B, 3, 1)).await;
    assert_eq!(rig.session_key(), Some(KEY_B), "the squatter registered");
    let squatter_flag = rig.session_flag();

    // A third account, its ticket also issued elsewhere: refused.
    const THIRD_TICKET: &str = "RELAUNCHC00000000003";
    let mut third = pending(THIRD_TICKET, 0x5EED_0A09, [0xC3; 32]);
    third.client_ip = "203.0.113.9".parse().unwrap();
    rig.issue(third);
    rig.deliver(&base_app_login(THIRD_TICKET, 4, 1)).await;
    assert_eq!(
        rig.session_key(),
        Some(KEY_B),
        "a foreign-IP ticket cannot reclaim the address"
    );

    let capture = LogCapture::install();
    rig.issue(pending(TICKET_A, ACCOUNT_ID, KEY_A)); // issued to 127.0.0.1
    rig.deliver(&base_app_login(TICKET_A, 5, 1)).await;

    assert_eq!(
        rig.session_key(),
        Some(KEY_A),
        "the victim's login must take its address back"
    );
    assert!(
        squatter_flag.load(Ordering::Relaxed),
        "the squatter is torn down"
    );
    assert!(
        !rig.pending_logins.lock().unwrap().contains_key(TICKET_A),
        "the victim's ticket is consumed by its own login"
    );
    let row = capture
        .find_event(Level::WARN, "reclaims the address", "address_reclaimed")
        .unwrap_or_else(|| panic!("no reclaim row; saw {:#?}", capture.all()));
    assert!(row.has_field("account_id", &OTHER_ACCOUNT_ID.to_string()));
    assert!(row.has_field("ticket_account_id", &ACCOUNT_ID.to_string()));
    assert!(
        capture
            .find_message(Level::INFO, "Client entities cleaned up")
            .is_some_and(|e| e.has_field("disconnect_reason", "address_reclaimed")),
        "the squatter's teardown is labelled address_reclaimed"
    );

    rig.stop();
}

/// PR #1246 review, finding 5: a poisoned `connected` lock refuses the
/// login rather than reading as "no session here".
#[test]
fn a_poisoned_connected_lock_refuses_the_takeover() {
    use crate::base::login::relaunch::{address_claim, AddressClaim};

    let connected: Connected = Arc::new(Mutex::new(HashMap::new()));
    let poison = Arc::clone(&connected);
    let _ = std::thread::spawn(move || {
        let _guard = poison.lock().unwrap();
        panic!("poison the connected map");
    })
    .join();
    assert!(connected.is_poisoned());

    let claim = address_claim(
        &connected,
        SocketAddr::from(([127, 0, 0, 1], 52616)),
        &pending(TICKET_B, ACCOUNT_ID, KEY_B),
    );
    assert_eq!(claim, AddressClaim::Refused);
}
