//! Duplicate-login rows say whether the displaced session was alive
//! (TG-NET-03). A session idle past `STALE_IDLE_MS` is a crashed client that
//! relaunched, logged at INFO; a session still receiving packets is a real
//! concurrent login, logged at WARN. Reverting the liveness fields or the
//! level split fails these tests.

use std::time::Duration;

use tracing::Level;

use super::*;
use crate::test_support::LogCapture;

const ACCOUNT_ID: u32 = 0xC0FF_EE77;
const MESSAGE: &str = "Duplicate login -- evicting old session";

/// Log a first login on `addr_a`, optionally backdate its `last_recv` by
/// `idle`, then log a second login for the same account on `addr_b`.
async fn second_login_after(idle: Option<Duration>) {
    let transport = make_transport();
    let addr_a: SocketAddr = "127.0.0.1:55571".parse().unwrap();
    let addr_b: SocketAddr = "127.0.0.1:55572".parse().unwrap();
    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let cell_tx = None;

    let mut p_a = make_pending_login(ACCOUNT_ID, 0x01);
    p_a.ticket = "TICKETA000000000071A".to_string();
    let ticket_a = p_a.ticket.clone();
    pending_logins.lock().unwrap().insert(ticket_a.clone(), p_a);
    handle_login(
        &transport,
        addr_a,
        1,
        &ticket_a,
        &pending_logins,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
    )
    .await
    .expect("first login");
    if let Some(idle) = idle {
        let map = connected.lock().unwrap();
        let old = map.get(&addr_a).expect("session A registered");
        *old.last_recv.lock().unwrap() = Instant::now() - idle;
    }

    let mut p_b = make_pending_login(ACCOUNT_ID, 0x02);
    p_b.ticket = "TICKETB000000000072B".to_string();
    let ticket_b = p_b.ticket.clone();
    pending_logins.lock().unwrap().insert(ticket_b.clone(), p_b);
    handle_login(
        &transport,
        addr_b,
        2,
        &ticket_b,
        &pending_logins,
        &connected,
        &entity_manager,
        &cell_tx,
        &entity_to_addr,
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
    )
    .await
    .expect("second login");
    cancel_session(&connected, addr_b);
}

#[tokio::test]
async fn stale_duplicate_login_is_info() {
    let capture = LogCapture::install();
    second_login_after(Some(Duration::from_secs(20))).await;

    let line = capture
        .find_message(Level::INFO, MESSAGE)
        .unwrap_or_else(|| panic!("no INFO eviction row: {:#?}", capture.all()));
    let idle_ms: u64 = line
        .fields
        .get("old_idle_ms")
        .expect("old_idle_ms field")
        .parse()
        .expect("old_idle_ms is numeric");
    assert!(idle_ms >= 20_000, "{line:#?}");
    assert!(line.has_field("reason", "duplicate_login"), "{line:#?}");
    assert!(line.fields.contains_key("old_in_world"), "{line:#?}");
    assert!(line.fields.contains_key("old_session_secs"), "{line:#?}");
}

#[tokio::test]
async fn fresh_duplicate_login_is_warn() {
    let capture = LogCapture::install();
    second_login_after(None).await;

    assert!(
        capture.find_message(Level::WARN, MESSAGE).is_some(),
        "no WARN eviction row: {:#?}",
        capture.all()
    );
    assert!(
        capture.find_message(Level::INFO, MESSAGE).is_none(),
        "a live session must not log at INFO"
    );
}
