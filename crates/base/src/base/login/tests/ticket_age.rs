//! KI-8 / PR #1246 review finding 6: Phase 3 enforces the documented 30 s
//! ticket lifetime itself. The auth reaper only sweeps every 10 s, so before
//! this a ticket stayed usable for up to about 40 s. Fails with the age
//! check removed from `handle_login`.

use tracing::Level;

use super::*;
use crate::test_support::LogCapture;

async fn login_with_ticket_aged(age: std::time::Duration, port: u16) -> bool {
    let transport = make_transport();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let mut pending = make_pending_login(0x7000_2601, 0x26);
    pending.created = Instant::now()
        .checked_sub(age)
        .expect("host uptime must exceed the ticket age");
    let ticket = pending.ticket.clone();
    pending_logins
        .lock()
        .unwrap()
        .insert(ticket.clone(), pending);

    handle_login(
        &transport,
        addr,
        1,
        &ticket,
        &pending_logins,
        &connected,
        &Arc::new(Mutex::new(EntityManager::new())),
        &None,
        &Arc::new(Mutex::new(HashMap::new())),
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
    )
    .await
    .unwrap();

    assert!(
        !pending_logins.lock().unwrap().contains_key(&ticket),
        "the ticket is gone either way: consumed or burned"
    );
    let registered = connected.lock().unwrap().contains_key(&addr);
    cancel_session(&connected, addr);
    registered
}

#[tokio::test]
async fn a_ticket_past_its_lifetime_is_refused_at_phase3() {
    let capture = LogCapture::install();

    let registered = login_with_ticket_aged(
        crate::auth::TICKET_TTL + std::time::Duration::from_secs(1),
        55601,
    )
    .await;

    assert!(!registered, "an expired ticket must not register a session");
    let row = capture
        .find_event(Level::WARN, "past its lifetime", "ticket_expired")
        .unwrap_or_else(|| panic!("no ticket_expired row; saw {:#?}", capture.all()));
    assert!(row.has_field("account_id", &0x7000_2601u32.to_string()));
}

#[tokio::test]
async fn a_ticket_inside_its_lifetime_still_logs_in() {
    let registered = login_with_ticket_aged(std::time::Duration::from_secs(5), 55602).await;
    assert!(registered);
}
