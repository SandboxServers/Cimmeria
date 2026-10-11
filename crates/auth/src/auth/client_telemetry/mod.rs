//! Which game session has a client telemetry stream (TG-NET-18, owner
//! decision D-TG14).
//!
//! The launcher mints its telemetry token before the game logs in, so it
//! does not know the account. What it does send is its `machine_id` (a
//! truncated SHA-256 of the Windows `MachineGuid`; the raw GUID never leaves
//! the machine). The mint signs that into the token as `mid` and records the
//! stream here with the address the request came from. World entry asks
//! [`client_telemetry_for`] with the game session's address and logs what it
//! finds:
//!
//! - `present`: exactly one machine on that address has a live stream (or a
//!   stream names this account, which wins), with its `session_id`, the same
//!   `session_id` every replayed client row carries;
//! - `ambiguous`: several machines on that address (a shared NAT), with the
//!   candidate count;
//! - `absent`: streams have been recorded since boot, none matches;
//! - `unknown`: no launcher has recorded a stream since boot, so the server
//!   cannot tell; `absent` there would read as a client telemetry failure.
//!
//! Minting is unauthenticated: anyone can mint a token under any machine id
//! or account name from their own address, so `present` and its
//! `session_id` are a launcher's claim, not proof, and never a privilege.
//!
//! The registry is process-wide because the mint (admin API, login port) and
//! world entry (BaseApp) run in one server process. Each index holds at most
//! [`MAX_STREAMS`] entries: an expired stream is dropped first, then the one
//! closest to expiry, so an unauthenticated caller cannot grow it without
//! bound.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Mutex, OnceLock};

/// Ceiling on recorded streams per index. A mint costs a quota slot per
/// address and per install, so this only matters to a caller cycling both.
pub const MAX_STREAMS: usize = 4096;

/// What world entry logs as `client_telemetry`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientTelemetry {
    /// One live stream matches; `session_id` is its telemetry session (the
    /// `session_id` on its replayed rows), `machine_id` the launcher's hashed
    /// machine id when it sent one.
    Present {
        session_id: String,
        machine_id: Option<String>,
    },
    /// Several machines have live streams from this session's address.
    Ambiguous { candidates: u32 },
    /// Streams have been recorded since boot, but none matches.
    Absent,
    /// No stream has been recorded since boot, so the server cannot tell.
    Unknown,
}

impl ClientTelemetry {
    /// The `client_telemetry` field value.
    pub fn as_str(&self) -> &'static str {
        match self {
            ClientTelemetry::Present { .. } => "present",
            ClientTelemetry::Ambiguous { .. } => "ambiguous",
            ClientTelemetry::Absent => "absent",
            ClientTelemetry::Unknown => "unknown",
        }
    }

    /// The stream's telemetry `session_id`, when one matched.
    pub fn session_id(&self) -> Option<&str> {
        match self {
            ClientTelemetry::Present { session_id, .. } => Some(session_id),
            _ => None,
        }
    }

    /// The matched stream's hashed machine id, when it has one.
    pub fn machine_id(&self) -> Option<&str> {
        match self {
            ClientTelemetry::Present { machine_id, .. } => machine_id.as_deref(),
            _ => None,
        }
    }

    /// How many machines share the address, for `ambiguous`.
    pub fn candidates(&self) -> Option<u32> {
        match self {
            ClientTelemetry::Ambiguous { candidates } => Some(*candidates),
            _ => None,
        }
    }
}

/// One mint or refresh, as the registry records it.
#[derive(Debug, Clone, Copy)]
pub struct StreamRecord<'a> {
    /// The token's `sid`.
    pub session_id: &'a str,
    /// The token's `exp`, unix seconds.
    pub expires_at_unix: i64,
    /// The token's `mid` claim.
    pub machine_id: Option<&'a str>,
    /// The token's `acct` claim.
    pub account_name: Option<&'a str>,
    /// The address the mint or refresh request came from.
    pub peer_ip: Option<IpAddr>,
}

#[derive(Debug, Clone)]
struct Stream {
    session_id: String,
    machine_id: Option<String>,
    peer_ip: Option<IpAddr>,
    /// The token's `exp`, unix seconds. A refresh moves it on.
    expires_at_unix: i64,
}

impl Stream {
    fn present(&self) -> ClientTelemetry {
        ClientTelemetry::Present {
            session_id: self.session_id.clone(),
            machine_id: self.machine_id.clone(),
        }
    }
}

/// The streams by machine and by account, plus whether any has ever been
/// recorded. One lock, so a lookup never sees the flag without the maps.
#[derive(Debug, Default)]
struct Registry {
    by_machine: HashMap<String, Stream>,
    by_account: HashMap<String, Stream>,
    /// Set by the first recorded stream and never cleared: eviction or
    /// expiry does not turn `absent` back into `unknown`.
    any_recorded: bool,
}

impl Registry {
    fn record(&mut self, rec: &StreamRecord<'_>) {
        let machine = rec.machine_id.and_then(non_blank);
        let account = rec.account_name.and_then(key);
        if machine.is_none() && account.is_none() {
            return;
        }
        self.any_recorded = true;
        let stream = Stream {
            session_id: rec.session_id.to_string(),
            machine_id: machine.clone(),
            peer_ip: rec.peer_ip.map(|ip| ip.to_canonical()),
            expires_at_unix: rec.expires_at_unix,
        };
        if let Some(m) = machine {
            insert_newest(&mut self.by_machine, m, stream.clone());
        }
        if let Some(a) = account {
            insert_newest(&mut self.by_account, a, stream);
        }
    }

    fn lookup(&self, account: Option<&str>, peer_ip: Option<IpAddr>, now: i64) -> ClientTelemetry {
        if !self.any_recorded {
            return ClientTelemetry::Unknown;
        }
        // A stream that names the account is the strongest claim there is.
        if let Some(s) = account
            .and_then(key)
            .and_then(|k| self.by_account.get(&k))
            .filter(|s| s.expires_at_unix > now)
        {
            return s.present();
        }
        let Some(ip) = peer_ip.map(|ip| ip.to_canonical()) else {
            return ClientTelemetry::Absent;
        };
        let mut live = self
            .by_machine
            .values()
            .filter(|s| s.expires_at_unix > now && s.peer_ip == Some(ip));
        match (live.next(), live.count()) {
            (None, _) => ClientTelemetry::Absent,
            (Some(s), 0) => s.present(),
            (Some(_), rest) => ClientTelemetry::Ambiguous {
                candidates: u32::try_from(rest + 1).unwrap_or(u32::MAX),
            },
        }
    }
}

/// Insert unless a stale refresh would overwrite a newer stream. A refresh
/// of an older stream can land after a newer mint for the same machine (a
/// relaunch while the old launcher's refresh was in flight); the stream
/// that lives longer is the newer one, so keep it.
fn insert_newest(map: &mut HashMap<String, Stream>, key: String, stream: Stream) {
    if let Some(existing) = map.get(&key) {
        if existing.expires_at_unix > stream.expires_at_unix
            && existing.session_id != stream.session_id
        {
            return;
        }
    }
    if !map.contains_key(&key) && map.len() >= MAX_STREAMS {
        evict_one(map);
    }
    map.insert(key, stream);
}

fn registry() -> &'static Mutex<Registry> {
    static REGISTRY: OnceLock<Mutex<Registry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Mutex::new(Registry::default()))
}

fn non_blank(s: &str) -> Option<String> {
    let trimmed = s.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Account names are matched without regard to ASCII case, as a player may
/// type theirs differently in the launcher and at the game's login.
fn key(account_name: &str) -> Option<String> {
    non_blank(account_name).map(|s| s.to_ascii_lowercase())
}

/// Record (or, on refresh, extend) a launcher's telemetry stream. A later
/// mint for a machine or account replaces an earlier one (a relaunch starts
/// a new stream), unless the earlier one outlives it: a stale refresh never
/// overwrites a newer stream. A record with neither a machine id nor an
/// account records nothing.
pub fn record_client_telemetry_stream(rec: StreamRecord<'_>) {
    let mut reg = match registry().lock() {
        Ok(reg) => reg,
        Err(e) => {
            tracing::warn!(
                error = %e,
                session_id = rec.session_id, // nt:id-only telemetry session UUID; it names nothing
                "Client telemetry stream not recorded: registry lock poisoned"
            );
            return;
        }
    };
    reg.record(&rec);
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

/// Whether the game session logged in as `account_name` from `peer_ip` has
/// a live client telemetry stream at `now_unix`.
pub fn client_telemetry_for(
    account_name: Option<&str>,
    peer_ip: Option<IpAddr>,
    now_unix: i64,
) -> ClientTelemetry {
    let reg = match registry().lock() {
        Ok(reg) => reg,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Client telemetry lookup failed: registry lock poisoned"
            );
            return ClientTelemetry::Unknown;
        }
    };
    reg.lookup(account_name, peer_ip, now_unix)
}

/// [`client_telemetry_for`] at the current wall-clock time.
pub fn client_telemetry_now(
    account_name: Option<&str>,
    peer_ip: Option<IpAddr>,
) -> ClientTelemetry {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0);
    client_telemetry_for(account_name, peer_ip, now)
}

#[cfg(test)]
mod tests;
