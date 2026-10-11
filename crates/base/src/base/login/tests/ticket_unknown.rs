//! Refusal context for an unknown ticket: the WARN names the peer address and
//! the closed `reason = "ticket_unknown"`, so a SigNoz row says which client
//! presented the stale ticket. Fails with `addr` or `reason` removed.

use tracing::Level;

use super::*;
use crate::test_support::LogCapture;

#[tokio::test]
async fn handle_login_unknown_ticket_names_addr_and_reason() {
    let capture = LogCapture::install();
    let transport = make_transport();
    let addr: SocketAddr = "127.0.0.1:55582".parse().unwrap();
    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));

    handle_login(
        &transport,
        addr,
        1,
        "no-such-ticket",
        &pending_logins,
        &connected,
        &entity_manager,
        &None,
        &entity_to_addr,
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
    )
    .await
    .expect("an unknown ticket is refused, not an error");

    let line = capture
        .find_message(Level::WARN, "Unknown or already-consumed ticket")
        .unwrap_or_else(|| panic!("no refusal line: {:#?}", capture.all()));
    assert!(line.has_field("addr", "127.0.0.1:55582"), "{line:#?}");
    assert!(line.has_field("reason", "ticket_unknown"), "{line:#?}");
}
