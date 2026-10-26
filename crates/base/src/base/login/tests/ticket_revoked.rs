//! CT-79 / F05: Phase 3 refuses a ticket whose launch ticket was revoked after
//! the SOAP handoff, so the BaseApp ticket is burned and no client connects.
//! Fails with the guard check removed from `handle_login_with_issuer`.

use tracing::Level;

use super::*;
use crate::auth::LaunchTicketStore;
use crate::test_support::LogCapture;

const ACCOUNT_ID: u32 = 79;

/// Runs Phase 3 for a handoff minted under `store`'s guard. `revoke` flips the
/// account's guard before the login runs. Returns whether the client was
/// registered and whether the ticket is still pending.
async fn login_under_guard(store: &LaunchTicketStore, revoke: bool, port: u16) -> (bool, bool) {
    let transport = make_transport();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let mut pending = make_pending_login(ACCOUNT_ID, 0x79);
    pending.ticket_guard = Some(store.guard(ACCOUNT_ID));
    let ticket = pending.ticket.clone();
    pending_logins
        .lock()
        .unwrap()
        .insert(ticket.clone(), pending);
    if revoke {
        store.revoke_account(ACCOUNT_ID);
    }

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

    let still_pending = pending_logins.lock().unwrap().contains_key(&ticket);
    let registered = connected.lock().unwrap().contains_key(&addr);
    cancel_session(&connected, addr);
    (registered, still_pending)
}

#[tokio::test]
async fn a_pending_login_from_a_revoked_launch_ticket_is_burned() {
    let capture = LogCapture::install();
    let store = LaunchTicketStore::default();

    let (registered, still_pending) = login_under_guard(&store, true, 55611).await;

    assert!(!still_pending, "the revoked ticket must be burned");
    assert!(
        !registered,
        "a revoked launch ticket must not connect a client"
    );
    let row = capture
        .find_event(Level::WARN, "revoked launch ticket", "ticket_revoked")
        .unwrap_or_else(|| panic!("no ticket_revoked row; saw {:#?}", capture.all()));
    assert!(row.has_field("account_id", &ACCOUNT_ID.to_string()));
}

#[tokio::test]
async fn a_pending_login_with_a_live_guard_is_admitted() {
    let store = LaunchTicketStore::default();

    let (registered, still_pending) = login_under_guard(&store, false, 55612).await;

    assert!(!still_pending, "the consumed ticket leaves the pending map");
    assert!(
        registered,
        "a ticket whose launch ticket is live must log in"
    );
}
