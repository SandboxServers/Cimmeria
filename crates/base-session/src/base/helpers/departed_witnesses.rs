//! Witnesses whose session just ended, so an `entity_to_addr` miss for them
//! is the teardown race and not a delivery failure.
//!
//! Every teardown (`logOff` to character select or full exit, and
//! `destroy_client_entities` for disconnects, duplicate logins and
//! inactivity timeouts) unmaps the player's entity id from `entity_to_addr`
//! at once, before the cell has seen `DisconnectEntity`. Whatever the cell
//! queued for that witness before then still arrives: a tick's position
//! relays for every NPC in its AoI, and the entity creates and leaves of the
//! same tick. Each one misses the map. On the colo (2026-09-29) one logOff
//! from a Castle_CellBlock instance produced 23 `entity_to_addr_miss` WARNs
//! in half a millisecond (witness 1, `entity_count_in_map` 0), and a
//! duplicate-login eviction another 20; the Discord errors channel posted
//! every one.
//!
//! Those sends have nobody to reach, so the helpers log them at DEBUG with
//! `reason = "witness_session_ended"`. A miss for a witness that did NOT
//! just end its session still WARNs with `reason = "entity_to_addr_miss"`:
//! that is a live player the server cannot reach (the #838 invisible-entity
//! class).
//!
//! The record is process-wide, like the user-channel registry, because the
//! send helpers are handed only the two session maps. It is keyed by entity
//! id and expires after [`DEPARTED_WITNESS_TTL`]. An expired or reused id
//! falls back to the WARN, the safe side: a reused id is mapped again by
//! its new session, so it never misses while that session is live.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

/// How long after its session ends a witness's misses count as the
/// teardown race. The backlog the cell can have queued is one or two ticks
/// (100 ms each) under normal load; 30 s leaves room for a stalled cell.
pub const DEPARTED_WITNESS_TTL: Duration = Duration::from_secs(30);

static DEPARTED: LazyLock<Mutex<HashMap<u32, Instant>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Unmap a player whose session is ending and record it as departed. The
/// two teardown paths call this instead of removing the mapping directly,
/// so the in-flight sends that follow log at DEBUG.
pub fn unmap_departed_witness(entity_to_addr: &Mutex<HashMap<u32, SocketAddr>>, entity_id: u32) {
    note_witness_departed(entity_id);
    if let Ok(mut m) = entity_to_addr.lock() {
        m.remove(&entity_id);
    }
}

/// Record `entity_id` as a witness whose session has just ended.
pub fn note_witness_departed(entity_id: u32) {
    let now = Instant::now();
    if let Ok(mut m) = DEPARTED.lock() {
        // Pruned here so the map holds at most the last TTL's teardowns.
        m.retain(|_, at| now.duration_since(*at) < DEPARTED_WITNESS_TTL);
        m.insert(entity_id, now);
    }
}

/// `true` when `entity_id`'s session ended within [`DEPARTED_WITNESS_TTL`].
pub fn witness_recently_departed(entity_id: u32) -> bool {
    DEPARTED.lock().is_ok_and(|m| {
        m.get(&entity_id)
            .is_some_and(|at| at.elapsed() < DEPARTED_WITNESS_TTL)
    })
}

/// The target these lines always had (they were emitted from `helpers`
/// itself), kept so SigNoz queries on the scope keep matching.
const HELPERS_TARGET: &str = "cimmeria_base_session::base::helpers";

/// Which witness-send helper missed, for the log line.
#[derive(Debug, Clone, Copy)]
pub(super) enum AddrMissPath {
    Unreliable,
    Reliable,
    Bundle,
}

/// The witness's character name for a miss line (Rule 6), from the session
/// that still claims the entity. The map miss is what brought us here, so
/// this scans the sessions; it runs only on the miss path. A departed
/// witness's session is usually gone already, and the name is left off.
fn witness_name(
    connected: &Mutex<HashMap<SocketAddr, super::ConnectedClientState>>,
    witness_id: u32,
) -> Option<&'static str> {
    let clients = connected.lock().ok()?;
    let c = clients
        .values()
        .find(|c| c.player_entity_id == Some(witness_id))?;
    cimmeria_entity::name_intern::intern_opt(c.player_name.as_deref())
}

/// Log an `entity_to_addr` miss: DEBUG for a witness whose session just
/// ended, WARN otherwise. The WARN messages and fields are the ones the
/// helpers always emitted (pinned by the negative-logging guards).
pub(super) fn log_addr_miss(
    witness_id: u32,
    map_size: usize,
    path: AddrMissPath,
    connected: &Mutex<HashMap<SocketAddr, super::ConnectedClientState>>,
) {
    if witness_recently_departed(witness_id) {
        tracing::debug!(
            target: HELPERS_TARGET,
            witness_id,
            witness_name = witness_name(connected, witness_id),
            reason = "witness_session_ended",
            entity_count_in_map = map_size,
            path = ?path,
            "AoI: witness's session has ended -- in-flight packet dropped"
        );
        return;
    }
    match path {
        // Entity gone from the address map while its session is live is a
        // player-visible bug (the witness sees stale state).
        AddrMissPath::Unreliable => tracing::warn!(
            target: HELPERS_TARGET,
            witness_id,
            witness_name = witness_name(connected, witness_id),
            reason = "entity_to_addr_miss",
            entity_count_in_map = map_size,
            "AoI: no client addr for witness -- packet dropped"
        ),
        // Dropping a reliable AoI packet means the client never sees a
        // state change (entity create/destroy, method call): the biggest
        // blind spot for the world-entry spawn-glitch class.
        AddrMissPath::Reliable => tracing::warn!(
            target: HELPERS_TARGET,
            witness_id,
            witness_name = witness_name(connected, witness_id),
            reason = "entity_to_addr_miss",
            entity_count_in_map = map_size,
            "AoI reliable: no client addr for witness -- packet dropped"
        ),
        // A bundle is a whole batch of AoI messages.
        AddrMissPath::Bundle => tracing::warn!(
            target: HELPERS_TARGET,
            witness_id,
            witness_name = witness_name(connected, witness_id),
            reason = "entity_to_addr_miss",
            entity_count_in_map = map_size,
            "AoI bundle: no client addr for witness -- bundle dropped"
        ),
    }
}
