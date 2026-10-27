//! Unit tests for the launch-ticket guard at the SOAP handoffs (CT-78): a
//! ticket revoked during UserAuth issues no SID, and a SID whose ticket was
//! revoked after Phase 1 gets no BaseApp ticket. Developer mode with no
//! database, so `ticket_account` answers without SQL.

use std::sync::Arc;
use std::time::Instant;

use crate::auth::credentials::sha1_upper_hex;
use crate::auth::handlers_tests::{
    phase1_body, post_server_selection, post_user_auth_with, test_handler_state,
};
use crate::auth::test_hooks::set_after_ticket_lookup;
use crate::auth::HandlerState;

const ACCOUNT: u32 = 7;
const NAME: &str = "Player_1234";
const LAUNCH: &str = "launch-guard";
const SELECTION_REFUSED: &str = "Your logon session has expired. Please log in again.";

/// Mint a launch ticket for account 7 and return its SHA-1 credential.
fn minted_hash(state: &HandlerState) -> String {
    let minted = state
        .launch_tickets
        .mint(ACCOUNT, NAME, LAUNCH, Instant::now());
    sha1_upper_hex(&minted.ticket)
}

/// The SID value from a `Set-Cookie` header.
fn sid_of(cookie: Option<String>) -> String {
    let cookie = cookie.expect("login sets a SID cookie");
    cookie
        .strip_prefix("SID=")
        .and_then(|rest| rest.split(';').next())
        .expect("SID cookie")
        .to_string()
}

/// **Handoff guard.** A launch ticket revoked after its lookup but before
/// the SID insert issues no SID: the login is refused and no session exists.
/// Without the check the SID is inserted with a live session.
#[tokio::test]
async fn userauth_refuses_when_the_ticket_is_revoked_during_the_lookup() {
    let state = test_handler_state();
    let hash = minted_hash(&state);
    let tickets = Arc::clone(&state.launch_tickets);
    let sessions = Arc::clone(&state.sessions);
    set_after_ticket_lookup(move || {
        tickets.revoke_account(ACCOUNT);
    });

    let body = phase1_body(NAME, "SGW_BETA", &hash);
    let (cookie, xml) = post_user_auth_with(state, &body, true).await;
    assert!(
        xml.contains("SGWLoginError") && cookie.is_none(),
        "got: {xml}"
    );
    assert!(sessions.lock().unwrap().is_empty());
}

/// **Handoff guard.** A SID issued from a live ticket is refused at Phase 2
/// once the ticket is revoked: error 15 and no BaseApp ticket (no
/// `pending_logins` entry). Without the check the SID becomes a ticket.
#[tokio::test]
async fn server_selection_refuses_a_sid_whose_ticket_was_revoked() {
    let state = test_handler_state();
    let hash = minted_hash(&state);
    let body = phase1_body(NAME, "SGW_BETA", &hash);
    let (cookie, xml) = post_user_auth_with(Arc::clone(&state), &body, true).await;
    assert!(xml.contains("SGWLoginSuccess"), "got: {xml}");
    let sid = sid_of(cookie);

    state.launch_tickets.revoke_account(ACCOUNT);
    let xml = post_server_selection(Arc::clone(&state), Some(&sid), "TestShard").await;
    assert!(xml.contains(SELECTION_REFUSED), "got: {xml}");
    assert!(state.pending_logins.lock().unwrap().is_empty());
}

/// **Handoff guard.** A SID from a launch ticket that is still live is
/// selected normally. Guards against refusing every ticket login.
#[tokio::test]
async fn server_selection_with_a_live_ticket_guard_still_succeeds() {
    let state = test_handler_state();
    let hash = minted_hash(&state);
    let body = phase1_body(NAME, "SGW_BETA", &hash);
    let (cookie, xml) = post_user_auth_with(Arc::clone(&state), &body, true).await;
    assert!(xml.contains("SGWLoginSuccess"), "got: {xml}");
    let sid = sid_of(cookie);

    let xml = post_server_selection(Arc::clone(&state), Some(&sid), "TestShard").await;
    assert!(!xml.contains(SELECTION_REFUSED), "got: {xml}");
    assert_eq!(state.pending_logins.lock().unwrap().len(), 1);
}

/// **Default guard.** A password login's session carries no ticket guard,
/// so no later revocation can refuse it.
#[tokio::test]
async fn password_login_sessions_carry_no_guard() {
    let state = test_handler_state();
    let body = phase1_body(
        "anyone",
        "SGW_BETA",
        "A94A8FE5CCB19BA61C4C0873D391E987982FBBD3",
    );
    let (cookie, xml) = post_user_auth_with(Arc::clone(&state), &body, false).await;
    assert!(xml.contains("SGWLoginSuccess"), "got: {xml}");
    let sid = sid_of(cookie);

    let sessions = state.sessions.lock().unwrap();
    let record = sessions.get(&sid).expect("password login stores a session");
    assert!(record.ticket_guard.is_none());
}
