//! Shared fixtures for the login tests: the `PendingLogin` handoff they seed.

use std::time::Instant;

use crate::auth::PendingLogin;

/// A Phase-3 handoff for `account_id`, keyed with `key_byte` repeated. It is a
/// password login, so it carries no launch-ticket guard.
pub(super) fn make_pending_login(account_id: u32, key_byte: u8) -> PendingLogin {
    PendingLogin {
        account_id,
        account_name: format!("acct{account_id}"),
        access_level: 0,
        test_account: false,
        ticket: "TICKET00000000000001".to_string(),
        // 32-byte key encoded as 64 hex chars, all the same byte.
        session_key: format!("{:02X}", key_byte).repeat(32),
        // Default to loopback so existing tests that don't exercise the
        // IP-binding seam still pass their addr == IP check.
        client_ip: "127.0.0.1".parse().unwrap(),
        created: Instant::now(),
        ticket_guard: None,
    }
}
