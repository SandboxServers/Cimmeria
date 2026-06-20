//! Presence fan-out helpers — broadcast CM 89 (`onContactListEvent`) to all
//! online players who have a given player in any of their contact lists.
//!
//! `fanout_login_status` is a thin wrapper around the generic
//! `fanout_contact_event` for the `LoggedInStatus` case. All three new
//! presence events (GainLevel / Death / GateTravel) use `fanout_contact_event`
//! directly with the appropriate `event_id` + `data_value` pair.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::contact_list::persistence::find_watchers;
use crate::base::contact_list::wire::{
    build_on_contact_list_event, DATA_OFFLINE, DATA_ONLINE, EVENT_LOGGED_IN_STATUS,
};
use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

/// Collect entity_ids of connected clients whose `active_player_id` is in
/// `watcher_set`. Pure extraction helper; unit-tested separately.
fn collect_watcher_entity_ids(
    clients: &HashMap<SocketAddr, ConnectedClientState>,
    watcher_set: &HashSet<i32>,
) -> Vec<u32> {
    clients
        .values()
        .filter_map(|state| {
            state
                .active_player_id
                .filter(|pid| watcher_set.contains(pid))
                .and(state.player_entity_id)
        })
        .collect()
}

/// Fan out `onContactListEvent` (CM 89) to all online players who have
/// `player_name` in any of their contact lists.
///
/// Generic core used by `fanout_login_status` and the three new presence
/// events (GainLevel, Death, GateTravel). `event_id` is one of the
/// `EVENT_*` bitfield constants from `wire.rs`; `data_value` is the
/// event-specific payload (new level, 0, or world_id).
///
/// Fire-and-forget — spawn this via `tokio::spawn` at call sites that must
/// not block a critical path (gate travel, XP grant, death burst).
///
/// # Invariant #5 (ignore-list leak)
/// Does NOT filter on the recipient's ignore list — `find_watchers` returns
/// all contact-list watchers regardless. Ignore-list filtering before fanout
/// is deferred to Phase 5.
pub(crate) async fn fanout_contact_event(
    player_name: &str,
    event_id: u32,
    data_value: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p.as_ref(),
        None => {
            tracing::warn!(
                player_name,
                event_id,
                "ContactList presence fanout: no DB pool, skipping"
            );
            return;
        }
    };

    let watcher_ids = match find_watchers(pool, player_name).await {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!(
                player_name,
                event_id,
                "ContactList presence fanout: find_watchers failed: {e}"
            );
            return;
        }
    };

    if watcher_ids.is_empty() {
        return;
    }

    // Build a HashSet for O(1) membership tests during the connected scan below.
    let watcher_set: HashSet<i32> = watcher_ids.iter().copied().collect();

    let args = build_on_contact_list_event(player_name, event_id, data_value);

    // Map player_id → entity_id for online players only.
    // Locks `connected` to iterate active sessions; does NOT take entity_to_addr.
    let watcher_entity_ids: Vec<u32> = {
        let clients = match connected.lock() {
            Ok(g) => g,
            Err(_) => {
                tracing::error!(
                    player_name,
                    event_id,
                    "ContactList presence fanout: connected lock poisoned"
                );
                return;
            }
        };
        collect_watcher_entity_ids(&clients, &watcher_set)
    };

    for entity_id in watcher_entity_ids {
        let args_clone = args.clone();
        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            move |key, version, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_CONTACT_LIST_EVENT,
                    &args_clone,
                    version,
                )
            },
        )
        .await;
    }

    tracing::debug!(
        player_name,
        event_id,
        data_value,
        watcher_count = watcher_ids.len(),
        "ContactList: presence fanout complete"
    );
}

/// Fan out `onContactListEvent` (CM 89, eventId=LoggedInStatus) to all online
/// players who have `player_name` in any of their contact lists.
///
/// Called on player login (`data_value = DATA_ONLINE`) and logout
/// (`data_value = DATA_OFFLINE`). Thin wrapper around `fanout_contact_event`.
pub(crate) async fn fanout_login_status(
    player_name: &str,
    is_online: bool,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let data_value = if is_online { DATA_ONLINE } else { DATA_OFFLINE };
    fanout_contact_event(
        player_name,
        EVENT_LOGGED_IN_STATUS,
        data_value,
        db_pool,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::contact_list::wire::{
        EVENT_DEATH, EVENT_GAIN_LEVEL, EVENT_GATE_TRAVEL, EVENT_LOGGED_IN_STATUS,
    };
    use crate::test_support::test_default_connected_client_state;
    use std::net::{IpAddr, Ipv4Addr};

    fn make_addr(port: u16) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
    }

    /// Build a minimal ConnectedClientState with the two fields under test.
    fn make_state(player_id: Option<i32>, entity_id: Option<u32>) -> ConnectedClientState {
        let mut s = test_default_connected_client_state();
        s.active_player_id = player_id;
        s.player_entity_id = entity_id;
        s
    }

    /// A client whose player_id is in the watcher set contributes its entity_id.
    #[test]
    fn collect_includes_matched_client() {
        let watcher_set: HashSet<i32> = [10i32, 20i32].into_iter().collect();
        let mut clients = HashMap::new();
        clients.insert(make_addr(1001), make_state(Some(10), Some(9001)));

        let entity_ids = collect_watcher_entity_ids(&clients, &watcher_set);
        assert_eq!(entity_ids, vec![9001u32]);
    }

    /// A client whose player_id is NOT in the watcher set is excluded.
    #[test]
    fn collect_excludes_non_watcher() {
        let watcher_set: HashSet<i32> = [10i32].into_iter().collect();
        let mut clients = HashMap::new();
        clients.insert(make_addr(1002), make_state(Some(99), Some(9999)));

        let entity_ids = collect_watcher_entity_ids(&clients, &watcher_set);
        assert!(
            entity_ids.is_empty(),
            "non-watcher player_id must not appear in entity_ids"
        );
    }

    /// A client with no active_player_id (not yet logged in to a character)
    /// is excluded even when an entity_id is set.
    #[test]
    fn collect_excludes_unauthenticated_session() {
        let watcher_set: HashSet<i32> = [10i32].into_iter().collect();
        let mut clients = HashMap::new();
        clients.insert(make_addr(1003), make_state(None, Some(9001)));

        let entity_ids = collect_watcher_entity_ids(&clients, &watcher_set);
        assert!(
            entity_ids.is_empty(),
            "session with no player_id must not contribute an entity_id"
        );
    }

    /// A client with a matching player_id but no entity_id contributes nothing.
    #[test]
    fn collect_excludes_session_without_entity() {
        let watcher_set: HashSet<i32> = [10i32].into_iter().collect();
        let mut clients = HashMap::new();
        clients.insert(make_addr(1004), make_state(Some(10), None));

        let entity_ids = collect_watcher_entity_ids(&clients, &watcher_set);
        assert!(
            entity_ids.is_empty(),
            "session with player_id but no entity_id must contribute nothing"
        );
    }

    /// Mixed connected map: only matching, fully-loaded sessions contribute.
    #[test]
    fn collect_selects_correct_subset_from_mixed_map() {
        let watcher_set: HashSet<i32> = [10i32, 30i32].into_iter().collect();
        let mut clients = HashMap::new();
        // Watcher with entity: included.
        clients.insert(make_addr(2001), make_state(Some(10), Some(1010)));
        // Non-watcher: excluded.
        clients.insert(make_addr(2002), make_state(Some(20), Some(2020)));
        // Watcher with entity: included.
        clients.insert(make_addr(2003), make_state(Some(30), Some(3030)));
        // Watcher without entity: excluded.
        clients.insert(make_addr(2004), make_state(Some(10), None));

        let mut entity_ids = collect_watcher_entity_ids(&clients, &watcher_set);
        entity_ids.sort_unstable();
        assert_eq!(
            entity_ids,
            vec![1010u32, 3030u32],
            "only fully-loaded watcher sessions must appear"
        );
    }

    /// Empty watcher set produces no entity_ids regardless of connected clients.
    #[test]
    fn collect_with_empty_watcher_set_returns_empty() {
        let watcher_set: HashSet<i32> = HashSet::new();
        let mut clients = HashMap::new();
        clients.insert(make_addr(3001), make_state(Some(10), Some(9001)));

        let entity_ids = collect_watcher_entity_ids(&clients, &watcher_set);
        assert!(entity_ids.is_empty());
    }

    /// Empty connected map produces no entity_ids regardless of watcher set.
    #[test]
    fn collect_with_empty_connected_map_returns_empty() {
        let watcher_set: HashSet<i32> = [10i32].into_iter().collect();
        let clients: HashMap<SocketAddr, ConnectedClientState> = HashMap::new();

        let entity_ids = collect_watcher_entity_ids(&clients, &watcher_set);
        assert!(entity_ids.is_empty());
    }

    // ── fanout_contact_event wire-arg correctness ────────────────────────────
    // These tests drive `build_on_contact_list_event` through the same path the
    // fanout takes so a refactor that passes the wrong (event_id, data_value)
    // pair fails here before the wire desync reaches the client.

    /// Helper: decode WSTRING and return (decoded_string, bytes_consumed).
    fn read_wstring(buf: &[u8]) -> (String, usize) {
        let count = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as usize;
        let mut chars = Vec::with_capacity(count);
        for i in 0..count {
            let off = 4 + i * 2;
            chars.push(u16::from_le_bytes(buf[off..off + 2].try_into().unwrap()));
        }
        (String::from_utf16(&chars).unwrap(), 4 + count * 2)
    }

    /// Helper: build a CM 89 payload and decode (event_id, data_value).
    fn decode_event(player_name: &str, event_id: u32, data_value: i32) -> (u32, i32) {
        let buf = build_on_contact_list_event(player_name, event_id, data_value);
        let (_, consumed) = read_wstring(&buf);
        let eid = u32::from_le_bytes(buf[consumed..consumed + 4].try_into().unwrap());
        let dv = i32::from_le_bytes(buf[consumed + 4..consumed + 8].try_into().unwrap());
        (eid, dv)
    }

    /// GainLevel: event_id must be EVENT_GAIN_LEVEL (2); data_value is the new
    /// level. Regression guard: a refactor using EVENT_LOGGED_IN_STATUS here
    /// would produce (1, level) and the client's `if flags == 2` branch never
    /// fires, silently dropping the notification.
    #[test]
    fn gain_level_arg_correctness() {
        let new_level = 7i32;
        let (eid, dv) = decode_event("Teal'c", EVENT_GAIN_LEVEL, new_level);
        assert_eq!(
            eid, EVENT_GAIN_LEVEL,
            "GainLevel must use event_id 2 (EVENT_GAIN_LEVEL), not {eid}"
        );
        assert_eq!(
            dv, new_level,
            "GainLevel data_value must be the new level ({new_level}), got {dv}"
        );
    }

    /// Death: event_id must be EVENT_DEATH (4); data_value is always 0
    /// (client ignores the value but shows "{Name} has died").
    #[test]
    fn death_arg_correctness() {
        let (eid, dv) = decode_event("O'Neill", EVENT_DEATH, 0);
        assert_eq!(
            eid, EVENT_DEATH,
            "Death must use event_id 4 (EVENT_DEATH), not {eid}"
        );
        assert_eq!(
            dv, 0,
            "Death data_value must be 0 (client ignores it), got {dv}"
        );
    }

    /// GateTravel: event_id must be EVENT_GATE_TRAVEL (8); data_value is the
    /// destination world_id that the client passes to getWorldInfo(value).Name.
    #[test]
    fn gate_travel_arg_correctness() {
        let world_id = 19i32; // Tollana from resources.worlds seed
        let (eid, dv) = decode_event("Carter", EVENT_GATE_TRAVEL, world_id);
        assert_eq!(
            eid, EVENT_GATE_TRAVEL,
            "GateTravel must use event_id 8 (EVENT_GATE_TRAVEL), not {eid}"
        );
        assert_eq!(
            dv, world_id,
            "GateTravel data_value must be the destination world_id ({world_id}), got {dv}"
        );
    }

    /// LoggedInStatus (via fanout_login_status path) still uses event_id 1.
    /// Regression guard for the wrapper that calls fanout_contact_event.
    #[test]
    fn logged_in_status_event_id_unchanged() {
        let (eid, dv) = decode_event("Jackson", EVENT_LOGGED_IN_STATUS, 1);
        assert_eq!(eid, EVENT_LOGGED_IN_STATUS);
        assert_eq!(dv, 1);
    }
}
