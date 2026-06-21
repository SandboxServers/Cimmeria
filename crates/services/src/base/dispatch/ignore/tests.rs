//! Unit tests for the base-side ignore enforcement: the tell filter in
//! `handle_send_player_communication`. These need no DB — the tell guard runs
//! before any cell routing and reads only the in-memory `ignore_set`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;

use crate::base::dispatch::chat::handle_send_player_communication;
use crate::base::ConnectedClientState;
use crate::cell::chat::CHAN_TELL;
use crate::cell::messages::BaseToCellMsg;
use crate::test_support::test_default_connected_client_state;

/// A connected session with a name + entity id (in-world).
fn session(name: &str, entity_id: u32) -> ConnectedClientState {
    let mut s = test_default_connected_client_state();
    s.player_name = Some(name.to_string());
    s.player_entity_id = Some(entity_id);
    s
}

/// Build a `sendPlayerCommunication(UINT8 channel, WSTRING target, WSTRING text)`
/// payload for a tell to `target`.
fn tell_payload(target: &str, text: &str) -> Vec<u8> {
    let mut p = vec![CHAN_TELL];
    let t16: Vec<u16> = target.encode_utf16().collect();
    p.extend_from_slice(&(t16.len() as u32).to_le_bytes());
    for c in t16 {
        p.extend_from_slice(&c.to_le_bytes());
    }
    let x16: Vec<u16> = text.encode_utf16().collect();
    p.extend_from_slice(&(x16.len() as u32).to_le_bytes());
    for c in x16 {
        p.extend_from_slice(&c.to_le_bytes());
    }
    p
}

/// Drive a tell from `sender_addr` and report whether any `BaseToCellMsg`
/// was emitted (the tell guard returns early — dropping means no cell msg).
async fn tell_routed(
    sender_addr: SocketAddr,
    sender_name: &str,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    target: &str,
) -> bool {
    let (tx, mut rx) = mpsc::channel::<BaseToCellMsg>(8);
    let payload = tell_payload(target, "hi there");
    handle_send_player_communication(
        &payload,
        &Some(sender_name.to_string()),
        sender_addr,
        connected,
        &Some(tx),
    )
    .await;
    rx.try_recv().is_ok()
}

#[tokio::test]
async fn tell_dropped_when_sender_ignores_target() {
    let a: SocketAddr = "127.0.0.1:7001".parse().unwrap();
    let b: SocketAddr = "127.0.0.1:7002".parse().unwrap();
    let mut alice = session("Alice", 10);
    alice.ignore_set.insert("Bob".to_string());
    let map = HashMap::from([(a, alice), (b, session("Bob", 20))]);
    let connected = Arc::new(Mutex::new(map));

    // Alice tells Bob → dropped (Alice ignores Bob).
    assert!(
        !tell_routed(a, "Alice", &connected, "Bob").await,
        "tell from Alice to ignored Bob must be dropped"
    );
}

#[tokio::test]
async fn tell_dropped_when_target_ignores_sender() {
    let a: SocketAddr = "127.0.0.1:7003".parse().unwrap();
    let b: SocketAddr = "127.0.0.1:7004".parse().unwrap();
    let mut bob = session("Bob", 20);
    bob.ignore_set.insert("Alice".to_string());
    let map = HashMap::from([(a, session("Alice", 10)), (b, bob)]);
    let connected = Arc::new(Mutex::new(map));

    // Alice tells Bob, but Bob ignores Alice → dropped (symmetric).
    assert!(
        !tell_routed(a, "Alice", &connected, "Bob").await,
        "tell to a target who ignores the sender must be dropped"
    );
}

#[tokio::test]
async fn tell_to_third_party_not_dropped() {
    let a: SocketAddr = "127.0.0.1:7005".parse().unwrap();
    let c: SocketAddr = "127.0.0.1:7006".parse().unwrap();
    let mut alice = session("Alice", 10);
    alice.ignore_set.insert("Bob".to_string()); // ignores Bob, not Carol
    let map = HashMap::from([(a, alice), (c, session("Carol", 30))]);
    let connected = Arc::new(Mutex::new(map));

    // Alice tells Carol (not ignored either way) → routed to cell.
    assert!(
        tell_routed(a, "Alice", &connected, "Carol").await,
        "tell to a non-ignored third party must be routed"
    );
}

/// Live-DB: `resync_ignore_after_member_change` reloads the authoritative Ignore
/// membership, updates the in-memory `ignore_set`, and pushes
/// `UpdateIgnoreList` to the cell. This is the path both `chatIgnore` and the
/// contact-list-UI member edit funnel through. Self-skips without `DATABASE_URL`.
#[tokio::test]
async fn resync_reloads_ignore_list_and_pushes_to_cell() {
    use crate::base::contact_list::persistence::{add_members, ensure_system_lists};
    use crate::test_support::require_db_or_skip;

    let pool = require_db_or_skip!();
    let player_id: i32 = 0x7000_2C01;
    let entity_id: u32 = 720_001;
    let addr: SocketAddr = "127.0.0.1:7100".parse().unwrap();

    // Clean + seed: Ignore list (301) with one member.
    let _ = sqlx::query("DELETE FROM sgw_contact_list WHERE player_id = $1")
        .bind(player_id)
        .execute(&pool)
        .await;
    let (_friends, ignore_id) = ensure_system_lists(&pool, player_id)
        .await
        .expect("ensure system lists");
    add_members(&pool, player_id, ignore_id, &["Griefer".to_string()])
        .await
        .expect("add ignore member");

    let mut me = session("Me", entity_id);
    me.ignore_set.clear();
    let connected = Arc::new(Mutex::new(HashMap::from([(addr, me)])));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(entity_id, addr)])));
    let (tx, mut rx) = mpsc::channel::<BaseToCellMsg>(8);
    let db_opt = Some(Arc::new(pool.clone()));

    super::resync_ignore_after_member_change(
        entity_id,
        player_id,
        ignore_id,
        &db_opt,
        &connected,
        &entity_to_addr,
        &Some(tx),
    )
    .await;

    // In-memory cache reflects the DB Ignore membership.
    assert!(
        connected
            .lock()
            .unwrap()
            .get(&addr)
            .unwrap()
            .ignore_set
            .contains("Griefer"),
        "resync must update the session ignore_set from the DB"
    );
    // Cell received the seed for its AoI filter.
    match rx.try_recv() {
        Ok(BaseToCellMsg::UpdateIgnoreList {
            entity_id: eid,
            ignore_names,
        }) => {
            assert_eq!(eid, entity_id);
            assert!(ignore_names.contains("Griefer"));
        }
        _ => panic!("resync must push UpdateIgnoreList to the cell"),
    }

    let _ = sqlx::query("DELETE FROM sgw_contact_list WHERE player_id = $1")
        .bind(player_id)
        .execute(&pool)
        .await;
}
