//! Login audit event types and ring buffer.
//!
//! Provides a [`LoginEvent`] struct emitted by auth handlers at every login
//! outcome, a [`LoginEventBuffer`] ring buffer for WebSocket history replay,
//! and an [`emit_login_event`] helper for constructing and broadcasting events.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::sync::broadcast;

/// Maximum number of login events kept in the ring buffer.
const BUFFER_CAPACITY: usize = 100;

/// Declares [`LoginOutcome`] and its [`LoginOutcome::ALL`] list from one
/// variant table, so a new outcome cannot be added without joining `ALL`, and
/// through it the live-DB test that inserts every outcome against the
/// `login_audit.outcome_check` constraint (#841).
macro_rules! login_outcomes {
    ($($(#[$doc:meta])* $variant:ident => $text:literal,)+) => {
        /// Every outcome the auth handlers record in a [`LoginEvent`].
        ///
        /// The strings are persisted in `login_audit.outcome`, whose CHECK
        /// constraint (`db/sgw/Audit/Tables/login_audit.sql`) must list each
        /// one; `live_db_login_audit_accepts_every_outcome` pins that.
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum LoginOutcome {
            $($(#[$doc])* $variant,)+
        }

        impl LoginOutcome {
            /// Every variant, in declaration order.
            pub const ALL: &'static [LoginOutcome] = &[$(LoginOutcome::$variant,)+];

            /// The string stored in `login_audit.outcome` and sent to the
            /// admin live feed.
            pub fn as_str(self) -> &'static str {
                match self {
                    $(LoginOutcome::$variant => $text,)+
                }
            }
        }
    };
}

login_outcomes! {
    /// The phase completed.
    Success => "success",
    /// Unknown account or wrong password.
    InvalidCredentials => "invalid_credentials",
    /// The account exists but is disabled.
    AccountDisabled => "account_disabled",
    /// The credential lookup failed, or there is no database outside dev mode.
    DbError => "db_error",
    /// The client's protocol digest does not match.
    ProtocolMismatch => "protocol_mismatch",
    /// Credentials were fine but no shard is configured.
    NoShards => "no_shards",
    /// A plaintext password arrived over plain HTTP.
    PlaintextRequiresTls => "plaintext_requires_tls",
}

/// A single login audit event.
#[derive(Clone, Debug, Serialize)]
pub struct LoginEvent {
    /// Unix timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Account name from the login request.
    pub account_name: String,
    /// Account ID (if credentials were valid).
    pub account_id: Option<u32>,
    /// Client IP address.
    pub ip_address: String,
    /// Login phase: `"credential_check"` or `"shard_selection"`.
    pub phase: String,
    /// Outcome: `"success"`, `"invalid_credentials"`, `"account_disabled"`, etc.
    pub outcome: String,
    /// Selected shard name (Phase 2 only).
    pub shard: Option<String>,
    /// Extra detail (error messages, etc.).
    pub detail: Option<String>,
}

/// Thread-safe ring buffer of recent login events.
///
/// Mirrors the [`LogBuffer`](crate::ws::broadcast_layer::LogBuffer) pattern.
#[derive(Clone)]
pub struct LoginEventBuffer {
    inner: Arc<Mutex<VecDeque<LoginEvent>>>,
}

impl Default for LoginEventBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl LoginEventBuffer {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(BUFFER_CAPACITY))),
        }
    }

    /// Push an event, evicting the oldest if at capacity.
    pub fn push(&self, event: LoginEvent) {
        if let Ok(mut buf) = self.inner.lock() {
            if buf.len() >= BUFFER_CAPACITY {
                buf.pop_front();
            }
            buf.push_back(event);
        }
    }

    /// Snapshot all buffered events (oldest first).
    pub fn snapshot(&self) -> Vec<LoginEvent> {
        self.inner
            .lock()
            .map(|buf| buf.iter().cloned().collect())
            .unwrap_or_default()
    }
}

/// Build and broadcast a [`LoginEvent`].
///
/// Also pushes the event into the ring buffer for late-joining WebSocket
/// clients. Silently ignores send failures (no subscribers connected).
pub fn emit_login_event(
    tx: &broadcast::Sender<LoginEvent>,
    buffer: &LoginEventBuffer,
    account_name: &str,
    account_id: Option<u32>,
    ip_address: &str,
    phase: &str,
    outcome: LoginOutcome,
    shard: Option<&str>,
    detail: Option<&str>,
) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let event = LoginEvent {
        timestamp_ms: now,
        account_name: account_name.to_string(),
        account_id,
        ip_address: ip_address.to_string(),
        phase: phase.to_string(),
        outcome: outcome.as_str().to_string(),
        shard: shard.map(|s| s.to_string()),
        detail: detail.map(|d| d.to_string()),
    };

    buffer.push(event.clone());
    let _ = tx.send(event);
}

/// Insert one [`LoginEvent`] into `login_audit`.
///
/// The server's audit writer task calls this for every broadcast event; the
/// live-DB test below calls it too, so the test exercises the production SQL.
pub async fn persist_login_event(pool: &sqlx::PgPool, event: &LoginEvent) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO login_audit (event_time, account_name, account_id, ip_address, phase, outcome, shard, detail)          VALUES (TO_TIMESTAMP($1::DOUBLE PRECISION / 1000), $2, $3, $4::INET, $5, $6, $7, $8)",
    )
    .bind(event.timestamp_ms as f64)
    .bind(&event.account_name)
    .bind(event.account_id.map(|id| id as i32))
    .bind(&event.ip_address)
    .bind(&event.phase)
    .bind(&event.outcome)
    .bind(&event.shard)
    .bind(&event.detail)
    .execute(pool)
    .await
    .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::require_db_or_skip;

    /// `account_id` sentinel for the audit live-DB test. `login_audit` has no
    /// foreign key, so no `account` row is needed; the neighbouring
    /// credential tests use the `0x7000_1B00` window.
    const TEST_ACCOUNT_ID: i32 = 0x7000_1B40;

    #[test]
    fn outcome_strings_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for o in LoginOutcome::ALL {
            assert!(seen.insert(o.as_str()), "duplicate outcome {o:?}");
        }
    }

    /// Every outcome the handlers can emit must pass the `outcome_check`
    /// constraint, or the audit writer drops the row with only a warn (#841:
    /// `plaintext_requires_tls` was missing). Driven by `LoginOutcome::ALL`,
    /// so a new variant without a schema update fails here.
    #[tokio::test]
    async fn live_db_login_audit_accepts_every_outcome() {
        let pool = require_db_or_skip!();
        let name = format!("audit-live-db-{TEST_ACCOUNT_ID}");
        let cleanup = || async {
            let _ = sqlx::query("DELETE FROM login_audit WHERE account_id = $1")
                .bind(TEST_ACCOUNT_ID)
                .execute(&pool)
                .await;
        };
        cleanup().await;

        let mut failures = Vec::new();
        for phase in ["credential_check", "shard_selection"] {
            for &outcome in LoginOutcome::ALL {
                let event = LoginEvent {
                    timestamp_ms: 1_700_000_000_000,
                    account_name: name.clone(),
                    account_id: Some(TEST_ACCOUNT_ID as u32),
                    ip_address: "127.0.0.1".to_string(),
                    phase: phase.to_string(),
                    outcome: outcome.as_str().to_string(),
                    shard: None,
                    detail: None,
                };
                if let Err(e) = persist_login_event(&pool, &event).await {
                    failures.push(format!("{phase}/{}: {e}", outcome.as_str()));
                }
            }
        }

        let (rows,): (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM login_audit WHERE account_id = $1")
                .bind(TEST_ACCOUNT_ID)
                .fetch_one(&pool)
                .await
                .unwrap();
        cleanup().await;

        assert!(failures.is_empty(), "login_audit rejected: {failures:#?}");
        assert_eq!(rows as usize, 2 * LoginOutcome::ALL.len());
    }
}
