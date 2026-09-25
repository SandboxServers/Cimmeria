//! Mint/refresh quotas for the dev-session endpoints.
//!
//! The endpoints are reachable by anyone who can route TCP to the
//! admin port, so the quota is what stops one caller from minting
//! tokens in a loop.
//!
//! # Why a fixed-size table instead of a `HashMap`
//!
//! Both quota keys are supplied by the caller: the peer address, and
//! the `install_id` in the request body. A map keyed on either grows
//! with whatever the caller invents, which turns the defence into its
//! own memory-exhaustion vector. A fixed slot array indexed by
//! `hash(key) % N` cannot grow, needs no eviction policy, and costs
//! one modulo per request.
//!
//! Two distinct keys that land on the same slot share one counter.
//! An earlier design reset the slot when a different key arrived, but
//! that let a caller holding two colliding keys (two IPv6 /64s, say)
//! alternate them and reset its own counter on every request. Sharing
//! is fail-closed for whoever collides, and the key hash is seeded
//! randomly per process so a collision cannot be chosen or precomputed.
//! The residual cost is an unaimable ~1-in-4096 chance that two live
//! callers share a bucket.
//!
//! That also means the per-`install_id` quota is a speed bump, not a
//! boundary: `install_id` is chosen by the caller, so an attacker
//! rotates it. The per-IP quota is the load-bearing control; the
//! per-install one catches a launcher stuck in a relaunch loop.

use std::hash::Hasher;
use std::net::IpAddr;
use std::sync::Mutex;
#[cfg(not(test))]
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Slots per table. 4096 × ~32 bytes ≈ 128 KiB, allocated once.
const SLOTS: usize = 4096;

/// Upper bound on a caller-supplied `install_id`. The launcher sends
/// a UUID v4 (36 chars); the cap exists so the value cannot bloat a
/// log line, a SigNoz field, or a hash input.
pub const MAX_INSTALL_ID_LEN: usize = 128;

#[derive(Debug, thiserror::Error)]
#[error("{scope} quota exceeded — retry in {retry_after_secs}s")]
pub struct QuotaExceeded {
    pub scope: &'static str,
    pub retry_after_secs: u64,
}

struct Slot {
    start: Instant,
    count: u32,
}

/// A fixed-window counter table. `limit` and `window` are passed per
/// call so the operator-tunable policy stays in the handler and the
/// table stays a pure counting structure.
pub struct WindowTable {
    slots: Mutex<Box<[Option<Slot>]>>,
}

impl WindowTable {
    pub fn new() -> Self {
        let mut v = Vec::with_capacity(SLOTS);
        v.resize_with(SLOTS, || None);
        Self {
            slots: Mutex::new(v.into_boxed_slice()),
        }
    }

    /// Record one request against `key`. A `limit` of 0 disables the
    /// quota entirely — the operator escape hatch.
    pub fn check_and_record(
        &self,
        key: u64,
        limit: u32,
        window: Duration,
        scope: &'static str,
        now: Instant,
    ) -> Result<(), QuotaExceeded> {
        if limit == 0 {
            return Ok(());
        }
        let mut slots = self.slots.lock().unwrap_or_else(|p| p.into_inner());
        let idx = (key % slots.len() as u64) as usize;
        let slot = &mut slots[idx];
        let live = matches!(
            slot,
            Some(s) if now.saturating_duration_since(s.start) < window
        );
        if !live {
            *slot = Some(Slot {
                start: now,
                count: 1,
            });
            return Ok(());
        }
        let s = slot.as_mut().expect("live implies Some");
        if s.count >= limit {
            let elapsed = now.saturating_duration_since(s.start);
            let remaining = window.saturating_sub(elapsed);
            return Err(QuotaExceeded {
                scope,
                // Round up so a client that honours Retry-After to the
                // second never comes back inside the same window.
                retry_after_secs: remaining.as_secs().saturating_add(1),
            });
        }
        s.count += 1;
        Ok(())
    }
}

impl Default for WindowTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Quota key for a peer address. IPv6 is folded to its /64 prefix:
/// a single host is routinely handed a whole /64, so counting full
/// addresses would let one machine mint without limit.
pub fn ip_key(addr: IpAddr) -> u64 {
    match addr {
        IpAddr::V4(v4) => hash_bytes(&v4.octets()),
        IpAddr::V6(v6) => hash_bytes(&v6.octets()[..8]),
    }
}

pub fn install_key(install_id: &str) -> u64 {
    hash_bytes(install_id.as_bytes())
}

/// SipHash keyed with a per-process random seed, so a caller cannot
/// pick two keys that collide in a table (see the module docs).
#[cfg(not(test))]
fn hash_bytes(bytes: &[u8]) -> u64 {
    use std::hash::BuildHasher;
    static SEED: OnceLock<std::collections::hash_map::RandomState> = OnceLock::new();
    let mut h = SEED.get_or_init(Default::default).build_hasher();
    h.write(bytes);
    h.finish()
}

/// Tests hash with fixed keys so that whether two test addresses share
/// a slot is decided once, not by a 1-in-4096 roll on every run.
#[cfg(test)]
fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    h.write(bytes);
    h.finish()
}

/// Reject an `install_id` that could not have come from the launcher
/// before it reaches a token claim, a log field, or a hash input.
pub fn validate_install_id(install_id: &str) -> Result<(), &'static str> {
    if install_id.is_empty() {
        return Err("must not be empty");
    }
    if install_id.len() > MAX_INSTALL_ID_LEN {
        return Err("exceeds 128 bytes");
    }
    if !install_id
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("must be ASCII alphanumeric, '-' or '_'");
    }
    Ok(())
}

/// Upper bound on the free-form launcher metadata fields.
pub const MAX_METADATA_LEN: usize = 256;

/// Check a free-form launcher metadata field (`machine_id`, `branch`,
/// `git_sha`, `launcher_version`). These reach `%`-Display in the
/// mint log line, and the plain `fmt` sinks do not escape them, so an
/// embedded newline would forge a log line. Anything printable is
/// allowed — branch names carry `/` and `.`.
pub fn validate_metadata(value: &str) -> Result<(), &'static str> {
    if value.len() > MAX_METADATA_LEN {
        return Err("exceeds 256 bytes");
    }
    if value.chars().any(char::is_control) {
        return Err("must not contain control characters");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv6Addr;

    const WINDOW: Duration = Duration::from_secs(60);

    fn table() -> WindowTable {
        WindowTable::new()
    }

    // The Nth request inside the window is refused, the first N-1 are
    // not. Pins the boundary exactly rather than "some request fails".
    #[test]
    fn nth_request_in_window_is_refused() {
        let t = table();
        let now = Instant::now();
        for i in 0..3 {
            t.check_and_record(7, 3, WINDOW, "mint/ip", now)
                .unwrap_or_else(|e| panic!("request {i} should pass, got {e}"));
        }
        let err = t
            .check_and_record(7, 3, WINDOW, "mint/ip", now)
            .unwrap_err();
        assert_eq!(err.scope, "mint/ip");
    }

    // Retry-After points past the end of the window, never at 0 — a
    // client that sleeps for exactly the advertised value must land in
    // the next window.
    #[test]
    fn retry_after_outlasts_the_remaining_window() {
        let t = table();
        let now = Instant::now();
        t.check_and_record(7, 1, WINDOW, "mint/ip", now).unwrap();
        let at = now + Duration::from_secs(20);
        let err = t.check_and_record(7, 1, WINDOW, "mint/ip", at).unwrap_err();
        assert_eq!(
            err.retry_after_secs, 41,
            "40s remain, rounded up past the edge"
        );
        t.check_and_record(7, 1, WINDOW, "mint/ip", at + Duration::from_secs(41))
            .expect("the advertised retry time must actually be allowed");
    }

    // Crossing the window boundary resets the counter.
    #[test]
    fn window_expiry_restores_the_allowance() {
        let t = table();
        let now = Instant::now();
        t.check_and_record(7, 1, WINDOW, "mint/ip", now).unwrap();
        assert!(t.check_and_record(7, 1, WINDOW, "mint/ip", now).is_err());
        t.check_and_record(7, 1, WINDOW, "mint/ip", now + WINDOW)
            .expect("a new window starts with a full allowance");
    }

    // Keys in different slots are counted independently, so one caller
    // exhausting its allowance never refuses another.
    #[test]
    fn distinct_keys_do_not_share_an_allowance() {
        let t = table();
        let now = Instant::now();
        t.check_and_record(7, 1, WINDOW, "mint/ip", now).unwrap();
        t.check_and_record(8, 1, WINDOW, "mint/ip", now)
            .expect("a different key has its own counter");
    }

    // Two keys landing on the same slot share its counter. The old
    // reset-on-mismatch rule let a caller alternate two colliding keys
    // to reset its own counter forever; sharing makes the collision
    // fail closed for the colliding caller instead.
    #[test]
    fn slot_collision_shares_the_counter() {
        let t = table();
        let now = Instant::now();
        let a = 7u64;
        let b = a + SLOTS as u64; // same slot index
        t.check_and_record(a, 2, WINDOW, "mint/ip", now).unwrap();
        t.check_and_record(b, 2, WINDOW, "mint/ip", now).unwrap();
        assert!(
            t.check_and_record(a, 2, WINDOW, "mint/ip", now).is_err(),
            "alternating colliding keys must not reset the slot"
        );
        assert!(t.check_and_record(b, 2, WINDOW, "mint/ip", now).is_err());
    }

    // The key hash is seeded, so equal inputs still agree within one
    // process (the quota depends on it) while the seed stays hidden.
    #[test]
    fn keys_are_stable_within_the_process() {
        assert_eq!(install_key("abc"), install_key("abc"));
        assert_ne!(install_key("abc"), install_key("abd"));
    }

    // limit == 0 is the documented "disabled" value, not "refuse
    // everything" — an operator setting it must not break minting.
    #[test]
    fn zero_limit_disables_the_quota() {
        let t = table();
        let now = Instant::now();
        for _ in 0..1000 {
            t.check_and_record(7, 0, WINDOW, "mint/ip", now).unwrap();
        }
    }

    // A whole IPv6 /64 counts as one caller; hosts are routinely
    // handed an entire /64, so per-address counting would be free to
    // bypass.
    #[test]
    fn ipv6_addresses_in_one_slash64_share_a_key() {
        let a: IpAddr = "2001:db8:1:2::1".parse::<Ipv6Addr>().unwrap().into();
        let b: IpAddr = "2001:db8:1:2:ffff:ffff:ffff:ffff"
            .parse::<Ipv6Addr>()
            .unwrap()
            .into();
        let other: IpAddr = "2001:db8:1:3::1".parse::<Ipv6Addr>().unwrap().into();
        assert_eq!(ip_key(a), ip_key(b));
        assert_ne!(ip_key(a), ip_key(other));
    }

    #[test]
    fn ipv4_addresses_key_independently() {
        let a: IpAddr = "203.0.113.1".parse().unwrap();
        let b: IpAddr = "203.0.113.2".parse().unwrap();
        assert_ne!(ip_key(a), ip_key(b));
    }

    #[test]
    fn validate_metadata_accepts_launcher_values_and_rejects_line_breaks() {
        for ok in [
            "",
            "main",
            "feature/foo.bar",
            "0123456abcdef",
            "0.1.0",
            "DESKTOP-1",
        ] {
            validate_metadata(ok).unwrap_or_else(|e| panic!("{ok:?} should pass: {e}"));
        }
        assert!(validate_metadata("main\nlevel=error msg=forged").is_err());
        assert!(validate_metadata("a\rb").is_err());
        assert!(validate_metadata("x\u{1b}[31m").is_err());
        assert!(validate_metadata(&"a".repeat(MAX_METADATA_LEN + 1)).is_err());
    }

    // The launcher's own install_id shape (UUID v4) must pass.
    #[test]
    fn validate_install_id_accepts_launcher_uuid() {
        validate_install_id("3f2504e0-4f89-41d3-9a0c-0305e82c3301").unwrap();
    }

    #[test]
    fn validate_install_id_rejects_empty_oversized_and_non_token_chars() {
        assert!(validate_install_id("").is_err());
        assert!(validate_install_id(&"a".repeat(MAX_INSTALL_ID_LEN + 1)).is_err());
        validate_install_id(&"a".repeat(MAX_INSTALL_ID_LEN))
            .expect("exactly at the cap is allowed");
        // Newline would forge extra lines in any text log sink.
        assert!(validate_install_id("abc\ndef").is_err());
        assert!(validate_install_id("abc def").is_err());
        assert!(validate_install_id("../../etc/passwd").is_err());
    }
}
