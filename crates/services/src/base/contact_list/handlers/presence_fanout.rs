//! `fanout_login_status` — broadcast CM 89 (onContactListEvent / LoggedInStatus)
//! to all online players who have `player_name` in any of their contact lists.

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

/// Fan out `onContactListEvent` (CM 89, eventId=LoggedInStatus) to all online
/// players who have `player_name` in any of their contact lists.
///
/// Called on player login (`data_value = DATA_ONLINE`) and logout
/// (`data_value = DATA_OFFLINE`).
///
/// # Invariant #5 (ignore-list leak)
/// This function is for the *caller's* presence fanout only — it does NOT
/// check the recipient's ignore list because the spec does not filter
/// LoggedInStatus events on the notifier's side. If that becomes a
/// requirement (e.g., muted players), add a join against the recipient's
/// Ignore list in `find_watchers` or filter here before the send.
///
/// # TODO: presence dataValues for level/death/gate events (needs runtime capture)
/// GainLevel / Death / GateTravel events (eventId 1/2/3) are deferred —
/// their data values need x64dbg confirmation.
pub(crate) async fn fanout_login_status(
    player_name: &str,
    is_online: bool,
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
                "ContactList LoginStatus fanout: no DB pool, skipping"
            );
            return;
        }
    };

    let watcher_ids = match find_watchers(pool, player_name).await {
        Ok(ids) => ids,
        Err(e) => {
            tracing::error!(
                player_name,
                "ContactList LoginStatus fanout: find_watchers failed: {e}"
            );
            return;
        }
    };

    if watcher_ids.is_empty() {
        return;
    }

    // Build a HashSet for O(1) membership tests during the connected scan below.
    let watcher_set: HashSet<i32> = watcher_ids.iter().copied().collect();

    let data_value = if is_online { DATA_ONLINE } else { DATA_OFFLINE };
    let args = build_on_contact_list_event(player_name, EVENT_LOGGED_IN_STATUS, data_value);

    // Map player_id → entity_id for online players only.
    // Locks `connected` to iterate active sessions; does NOT take entity_to_addr.
    let watcher_entity_ids: Vec<u32> = {
        let clients = match connected.lock() {
            Ok(g) => g,
            Err(_) => {
                tracing::error!(
                    player_name,
                    "ContactList LoginStatus fanout: connected lock poisoned"
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
            move |key, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_CONTACT_LIST_EVENT,
                    &args_clone,
                )
            },
        )
        .await;
    }

    tracing::debug!(
        player_name,
        is_online,
        watcher_count = watcher_ids.len(),
        "ContactList: LoginStatus fanout complete"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
