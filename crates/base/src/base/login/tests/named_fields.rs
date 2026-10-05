//! Rule 6 guard for the Phase 3 login line: the account is named next to its
//! ID, so a SigNoz row or a Discord post says who logged in without a lookup
//! (`docs/architecture/instrumentation-discipline.md` Rule 6, NT-24). Fails
//! with `account_name` removed from the line.

use tracing::Level;

use super::*;
use crate::test_support::LogCapture;

#[tokio::test]
async fn phase3_login_line_names_the_account() {
    let capture = LogCapture::install();
    let transport = make_transport();
    let addr: SocketAddr = "127.0.0.1:55581".parse().unwrap();
    let pending_logins = Arc::new(Mutex::new(HashMap::new()));
    let connected = Arc::new(Mutex::new(HashMap::new()));
    let entity_manager = Arc::new(Mutex::new(EntityManager::new()));
    let entity_to_addr = Arc::new(Mutex::new(HashMap::new()));
    let pending = make_pending_login(0x7000_2401, 0x24);
    let ticket = pending.ticket.clone();
    let account_name = pending.account_name.clone();
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
        &entity_manager,
        &None,
        &entity_to_addr,
        &None,
        cimmeria_mercury::encryption::EncryptionVersion::V1,
        &cimmeria_base_session::base::plugin::BasePlugins::empty(),
    )
    .await
    .expect("Phase 3 handoff");
    cancel_session(&connected, addr);

    let line = capture
        .find_message(Level::INFO, "Phase 3 authenticated")
        .unwrap_or_else(|| panic!("no Phase 3 line: {:#?}", capture.all()));
    assert!(line.has_field("account_id", &0x7000_2401u32.to_string()));
    assert!(line.has_field("account_name", &account_name), "{line:#?}");
}
