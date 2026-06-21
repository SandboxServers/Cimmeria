//! BaseApp-side handlers for contact-list CellToBaseMsg variants.
//!
//! Split by lifecycle phase:
//! - `header_ops`     — list-header operations (create/delete/rename/flags_update)
//! - `member_ops`     — member operations (add_members/remove_members)
//! - `presence_fanout` — login/logout broadcast to watchers
//!
//! `push_contact_lists_on_login` lives here because it is the single entry
//! point that login-path callers (`world_entry_appearance::handle_on_client_ready`)
//! import — keeping it in `mod.rs` avoids a deeper import path for a hot path.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use cimmeria_mercury::transport::Transport;
use sqlx::PgPool;

use crate::base::contact_list::persistence::{
    ensure_system_lists, load_contact_lists, ContactList,
};
use crate::base::contact_list::wire::{
    build_on_contact_list_add_members, build_on_contact_list_event, build_on_contact_list_update,
    DATA_ONLINE, EVENT_LOGGED_IN_STATUS,
};
use crate::base::helpers::send_to_witness_reliable;
use crate::base::ConnectedClientState;
use crate::mercury::{build_player_entity_method_packet, method_idx};

pub(crate) mod header_ops;
pub(crate) mod member_ops;
pub(crate) mod presence_fanout;

// Re-export the public surface so callers (cell_dispatch, login path, etc.)
// can import from `handlers::*` without knowing the split.
pub(crate) use header_ops::{handle_create, handle_delete, handle_flags_update, handle_rename};
pub(crate) use member_ops::{handle_add_members, handle_remove_members};
pub(crate) use presence_fanout::{fanout_contact_event, fanout_login_status};

/// Push all contact lists + members to the player's client on world entry.
///
/// Called from `world_entry_appearance::handle_on_client_ready` after the
/// burst bundle. Calls `ensure_system_lists` first so every character
/// always has Friends / Ignore even on first login.
///
/// Does nothing (logs warn) when `db_pool` is `None`.
pub(crate) async fn push_contact_lists_on_login(
    entity_id: u32,
    player_id: i32,
    db_pool: &Option<Arc<PgPool>>,
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    let pool = match db_pool {
        Some(p) => p.as_ref(),
        None => {
            tracing::warn!(
                entity_id,
                player_id,
                "ContactList login push: no DB pool — contact lists not sent"
            );
            return;
        }
    };

    // Ensure system lists exist (idempotent).
    if let Err(e) = ensure_system_lists(pool, player_id).await {
        tracing::error!(
            entity_id,
            player_id,
            "ContactList: ensure_system_lists failed: {e}"
        );
        return;
    }

    let lists = match load_contact_lists(pool, player_id).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(
                entity_id,
                player_id,
                "ContactList login push: load_contact_lists failed: {e}"
            );
            return;
        }
    };

    for list in &lists {
        // CM 85: header.
        let update_args = build_on_contact_list_update(list.list_id, &list.name, list.flags as u32);
        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            |key, version, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_CONTACT_LIST_UPDATE,
                    &update_args,
                    version,
                )
            },
        )
        .await;

        // CM 87: members (sent even when empty — client expects the packet).
        let members_args = build_on_contact_list_add_members(list.list_id, &list.members);
        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            |key, version, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_CONTACT_LIST_ADD_MEMBERS,
                    &members_args,
                    version,
                )
            },
        )
        .await;
    }

    tracing::debug!(
        entity_id,
        player_id,
        list_count = lists.len(),
        "ContactList: pushed {} lists to client on login",
        lists.len()
    );

    // Reverse presence sync: tell THIS player which of their contacts are
    // ALREADY online. The login fanout (`fanout_login_status`) only notifies
    // *other* players that this player just came online; without this step a
    // player who logs in second sees their already-online friends as offline.
    push_initial_contact_presence(entity_id, &lists, transport, connected, entity_to_addr).await;
}

/// Send the logging-in player a CM 89 (`onContactListEvent`, LoggedInStatus =
/// online) for every contact on their lists who is currently online.
///
/// Mirrors `fanout_login_status` but in the opposite direction: that notifies
/// watchers about one player; this notifies one player about their watched
/// contacts. Online status is read from the in-memory `connected` sessions
/// (by character name), so no DB round-trip is needed.
async fn push_initial_contact_presence(
    entity_id: u32,
    lists: &[ContactList],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Character names of all currently-online players except this one.
    let online_names = match connected.lock() {
        Ok(clients) => collect_online_names(&clients, entity_id),
        Err(_) => {
            tracing::error!(
                entity_id,
                "ContactList initial presence: connected lock poisoned"
            );
            return;
        }
    };
    if online_names.is_empty() {
        return;
    }

    let to_notify = collect_online_member_names(lists, &online_names);
    for name in &to_notify {
        let args = build_on_contact_list_event(name, EVENT_LOGGED_IN_STATUS, DATA_ONLINE);
        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            entity_id,
            |key, version, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    entity_id,
                    method_idx::ON_CONTACT_LIST_EVENT,
                    &args,
                    version,
                )
            },
        )
        .await;
    }

    if !to_notify.is_empty() {
        tracing::debug!(
            entity_id,
            online_contacts = to_notify.len(),
            "ContactList: sent initial online presence for already-online contacts"
        );
    }
}

/// Character names of all currently-online players except `exclude_entity`
/// (the logging-in player themself). Pure helper over the connected-session
/// map; unit-tested separately. A session counts as online once it has a
/// `player_entity_id` (i.e. world entry reached).
fn collect_online_names(
    clients: &HashMap<SocketAddr, ConnectedClientState>,
    exclude_entity: u32,
) -> HashSet<String> {
    clients
        .values()
        .filter(|s| s.player_entity_id.is_some() && s.player_entity_id != Some(exclude_entity))
        .filter_map(|s| s.player_name.clone())
        .collect()
}

/// Distinct contact-member names (across all of `lists`) that appear in
/// `online_names`. Pure helper, unit-tested separately. Deduplicates a name
/// that is present in more than one list so the client gets one event per
/// online contact.
fn collect_online_member_names(
    lists: &[ContactList],
    online_names: &HashSet<String>,
) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for list in lists {
        for member in &list.members {
            if online_names.contains(member) && seen.insert(member.as_str()) {
                out.push(member.clone());
            }
        }
    }
    out
}

#[cfg(test)]
mod presence_login_tests {
    use super::*;

    fn list(name: &str, members: &[&str]) -> ContactList {
        ContactList {
            list_id: 1,
            name: name.to_string(),
            flags: 0,
            members: members.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn only_online_members_are_notified() {
        let lists = vec![
            list("Friends", &["Friendly", "Offliner"]),
            list("Ignore", &["BadGuy"]),
        ];
        let online: HashSet<String> = ["Friendly", "BadGuy", "Stranger"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let got = collect_online_member_names(&lists, &online);
        // Friendly (online friend) + BadGuy (online, on Ignore) — symmetric with
        // the watcher fanout, which also ignores list type. Offliner excluded.
        assert_eq!(got, vec!["Friendly".to_string(), "BadGuy".to_string()]);
    }

    #[test]
    fn duplicate_member_across_lists_notified_once() {
        let lists = vec![list("Friends", &["Dup"]), list("Squad", &["Dup"])];
        let online: HashSet<String> = ["Dup"].iter().map(|s| s.to_string()).collect();
        assert_eq!(
            collect_online_member_names(&lists, &online),
            vec!["Dup".to_string()]
        );
    }

    #[test]
    fn no_online_contacts_yields_empty() {
        let lists = vec![list("Friends", &["A", "B"])];
        let online: HashSet<String> = ["C"].iter().map(|s| s.to_string()).collect();
        assert!(collect_online_member_names(&lists, &online).is_empty());
    }

    // ── collect_online_names + full reverse-sync send path ──────────────────

    use crate::test_support::{test_default_connected_client_state, TestTransport};
    use cimmeria_mercury::transport::Transport;

    /// A session that has reached world entry (online) with a name + entity id.
    fn online_session(entity_id: u32, name: &str) -> ConnectedClientState {
        let mut s = test_default_connected_client_state();
        s.player_entity_id = Some(entity_id);
        s.player_name = Some(name.to_string());
        s
    }

    #[test]
    fn collect_online_names_excludes_self_and_pre_world_entry() {
        let a: SocketAddr = "127.0.0.1:1".parse().unwrap();
        let b: SocketAddr = "127.0.0.1:2".parse().unwrap();
        let c: SocketAddr = "127.0.0.1:3".parse().unwrap();
        let mut map = HashMap::new();
        map.insert(a, online_session(10, "Me")); // self — excluded
        map.insert(b, online_session(20, "Friendly")); // online other
        let mut not_ready = test_default_connected_client_state(); // no entity id yet
        not_ready.player_name = Some("Loading".to_string());
        map.insert(c, not_ready);

        let got = collect_online_names(&map, 10);
        assert_eq!(got.len(), 1);
        assert!(got.contains("Friendly"));
        assert!(!got.contains("Me"));
        assert!(!got.contains("Loading"));
    }

    #[tokio::test]
    async fn initial_presence_sends_event_for_online_contact() {
        let me_addr: SocketAddr = "127.0.0.1:5001".parse().unwrap();
        let friend_addr: SocketAddr = "127.0.0.1:5002".parse().unwrap();

        let mut me = test_default_connected_client_state();
        me.player_entity_id = Some(10);
        let mut map = HashMap::new();
        map.insert(me_addr, me);
        map.insert(friend_addr, online_session(20, "Friendly"));
        let connected = Arc::new(Mutex::new(map));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([
            (10u32, me_addr),
            (20u32, friend_addr),
        ])));

        let tt = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = tt.clone();

        let lists = vec![list("Friends", &["Friendly", "Offliner"])];
        push_initial_contact_presence(10, &lists, &transport, &connected, &entity_to_addr).await;

        // The logging-in player gets exactly one CM 89 (for the online Friendly);
        // nothing is sent to the friend in this reverse sync. Reverting the
        // push_initial_contact_presence call makes `to_me` empty.
        assert_eq!(
            tt.filter_to(me_addr).len(),
            1,
            "logging-in player must get one onContactListEvent for the online contact"
        );
        assert!(tt.filter_to(friend_addr).is_empty());
    }

    #[tokio::test]
    async fn initial_presence_silent_when_no_contact_online() {
        let me_addr: SocketAddr = "127.0.0.1:5003".parse().unwrap();
        let mut me = test_default_connected_client_state();
        me.player_entity_id = Some(10);
        let connected = Arc::new(Mutex::new(HashMap::from([(me_addr, me)])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(10u32, me_addr)])));

        let tt = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = tt.clone();

        let lists = vec![list("Friends", &["Friendly"])];
        push_initial_contact_presence(10, &lists, &transport, &connected, &entity_to_addr).await;

        assert!(
            tt.drain().is_empty(),
            "no events when none of the player's contacts are online"
        );
    }
}
