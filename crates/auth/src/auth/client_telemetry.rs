//! Which accounts have a client telemetry stream (TG-NET-18, owner decision
//! D-TG14).
//!
//! The launcher names the account in its dev-session mint request; the mint
//! signs the name into the token (`acct` claim) and records the stream here,
//! keyed by account name. World entry asks [`client_telemetry_for`] and logs
//! `client_telemetry = "present" | "absent"` with the stream's `session_id`,
//! the same `session_id` every replayed client row carries, so one SigNoz
//! filter joins a game session to its client telemetry.
//!
//! Minting is unauthenticated, so the account name is the launcher's claim,
//! not proof of identity: a label for finding the stream, never a privilege
//! (the same trust as the `kind` claim). A launcher that sends no account
//! name records nothing, and its player's world entry says `absent`.
//!
//! The registry is process-wide because the mint (admin API, login port) and
//! world entry (BaseApp) run in one server process. It holds at most
//! [`MAX_STREAMS`] entries: an expired stream is dropped first, then the one
//! closest to expiry, so an unauthenticated caller cannot grow it without
//! bound.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// Ceiling on recorded streams. A mint costs a quota slot per address and
/// per install, so this only matters to a caller cycling both.
pub const MAX_STREAMS: usize = 4096;

/// What world entry logs as `client_telemetry`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientTelemetry {
    /// A live (unexpired) stream names this account; `session_id` is its
    /// telemetry session, the `session_id` on its replayed rows.
    Present { session_id: String },
    /// No live stream names this account.
    Absent,
}

impl ClientTelemetry {
    /// The `client_telemetry` field value: `"present"` or `"absent"`.
    pub fn as_str(&self) -> &'static str {
        match self {
            ClientTelemetry::Present { .. } => "present",
            ClientTelemetry::Absent => "absent",
        }
    }

    /// The stream's telemetry `session_id`, when there is one.
    pub fn session_id(&self) -> Option<&str> {
        match self {
            ClientTelemetry::Present { session_id } => Some(session_id),
            ClientTelemetry::Absent => None,
        }
    }
}

#[derive(Debug, Clone)]
struct Stream {
    session_id: String,
    /// The token's `exp`, unix seconds. A refresh moves it on.
    expires_at_unix: i64,
}

fn streams() -> &'static Mutex<HashMap<String, Stream>> {
    static STREAMS: OnceLock<Mutex<HashMap<String, Stream>>> = OnceLock::new();
    STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Account names are matched without regard to ASCII case, as a player may
/// type theirs differently in the launcher and at the game's login.
fn key(account_name: &str) -> Option<String> {
    let trimmed = account_name.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_ascii_lowercase())
}

/// Record (or, on refresh, extend) the telemetry stream an account's
/// launcher minted. The latest mint for an account replaces an earlier one:
/// a relaunch starts a new stream. A blank name records nothing.
pub fn record_client_telemetry_stream(account_name: &str, session_id: &str, expires_at_unix: i64) {
    let Some(key) = key(account_name) else {
        return;
    };
    let Ok(mut map) = streams().lock() else {
        return;
    };
    if !map.contains_key(&key) && map.len() >= MAX_STREAMS {
        evict_one(&mut map);
    }
    map.insert(
        key,
        Stream {
            session_id: session_id.to_string(),
            expires_at_unix,
        },
    );
}

/// Drop one entry to make room: the one that expires first, which is an
/// expired one whenever any are.
fn evict_one(map: &mut HashMap<String, Stream>) {
    if let Some(oldest) = map
        .iter()
        .min_by_key(|(_, s)| s.expires_at_unix)
        .map(|(k, _)| k.clone())
    {
        map.remove(&oldest);
    }
}

/// Whether `account_name` has a live client telemetry stream at `now_unix`.
pub fn client_telemetry_for(account_name: Option<&str>, now_unix: i64) -> ClientTelemetry {
    let Some(key) = account_name.and_then(key) else {
        return ClientTelemetry::Absent;
    };
    let Ok(map) = streams().lock() else {
        return ClientTelemetry::Absent;
    };
    match map.get(&key) {
        Some(s) if s.expires_at_unix > now_unix => ClientTelemetry::Present {
            session_id: s.session_id.clone(),
        },
        _ => ClientTelemetry::Absent,
    }
}

/// [`client_telemetry_for`] at the current wall-clock time.
pub fn client_telemetry_now(account_name: Option<&str>) -> ClientTelemetry {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0);
    client_telemetry_for(account_name, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The registry is process-wide and tests run in parallel: every test
    // uses its own account names.

    #[test]
    fn a_recorded_stream_is_present_until_it_expires() {
        record_client_telemetry_stream("tg18-unit-live", "sid-live", 1_000);
        assert_eq!(
            client_telemetry_for(Some("tg18-unit-live"), 999),
            ClientTelemetry::Present {
                session_id: "sid-live".into()
            }
        );
        assert_eq!(
            client_telemetry_for(Some("tg18-unit-live"), 1_000),
            ClientTelemetry::Absent,
            "a stream is gone at its token's exp"
        );
    }

    #[test]
    fn an_unrecorded_or_missing_account_is_absent() {
        assert_eq!(
            client_telemetry_for(Some("tg18-unit-never-minted"), 0),
            ClientTelemetry::Absent
        );
        assert_eq!(client_telemetry_for(None, 0), ClientTelemetry::Absent);
        assert_eq!(client_telemetry_for(Some("  "), 0), ClientTelemetry::Absent);
        assert_eq!(ClientTelemetry::Absent.as_str(), "absent");
        assert_eq!(ClientTelemetry::Absent.session_id(), None);
    }

    #[test]
    fn the_join_ignores_ascii_case_and_surrounding_space() {
        record_client_telemetry_stream(" TG18-Unit-Case ", "sid-case", i64::MAX);
        let got = client_telemetry_for(Some("tg18-unit-case"), 0);
        assert_eq!(got.as_str(), "present");
        assert_eq!(got.session_id(), Some("sid-case"));
    }

    #[test]
    fn a_later_mint_replaces_the_earlier_stream() {
        record_client_telemetry_stream("tg18-unit-relaunch", "sid-first", i64::MAX);
        record_client_telemetry_stream("tg18-unit-relaunch", "sid-second", i64::MAX);
        assert_eq!(
            client_telemetry_for(Some("tg18-unit-relaunch"), 0).session_id(),
            Some("sid-second")
        );
    }

    #[test]
    fn a_blank_name_records_nothing() {
        record_client_telemetry_stream("", "sid-blank", i64::MAX);
        assert_eq!(client_telemetry_for(Some(""), 0), ClientTelemetry::Absent);
    }

    #[test]
    fn eviction_drops_the_stream_closest_to_expiry() {
        let mut map = HashMap::from([
            (
                "late".to_string(),
                Stream {
                    session_id: "a".into(),
                    expires_at_unix: 20,
                },
            ),
            (
                "early".to_string(),
                Stream {
                    session_id: "b".into(),
                    expires_at_unix: 10,
                },
            ),
        ]);
        evict_one(&mut map);
        assert!(map.contains_key("late"));
        assert!(!map.contains_key("early"));
    }
}
