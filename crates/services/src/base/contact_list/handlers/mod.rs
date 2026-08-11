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
    ensure_system_lists, load_contact_lists, ContactList, IGNORE_LIST_FLAGS,
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

    // Seed the in-memory ignore_set from the 'Ignore' list (flags=301) so the
    // base-side tell filter and the cell-side AoI seed have it immediately.
    // The caller (`handle_on_client_ready`) reads ignore_set right after this
    // to push `BaseToCellMsg::UpdateIgnoreList` to the cell.
    {
        let ignore_names = ignore_names_from_lists(&lists);
        let addr = entity_to_addr
            .lock()
            .ok()
            .and_then(|m| m.get(&entity_id).copied());
        if let Some(addr) = addr {
            if let Ok(mut clients) = connected.lock() {
                if let Some(c) = clients.get_mut(&addr) {
                    c.ignore_set = ignore_names;
                }
            }
        }
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
    let member_names: Vec<String> = lists
        .iter()
        .flat_map(|l| l.members.iter().cloned())
        .collect();
    notify_online_contacts(
        entity_id,
        &member_names,
        transport,
        connected,
        entity_to_addr,
    )
    .await;
}

/// Send `recipient_entity_id` a CM 89 (`onContactListEvent`, LoggedInStatus =
/// online) for each name in `candidate_names` that belongs to a currently
/// online player.
///
/// Shared by the login reverse-sync (all contact members) and add-members (the
/// just-added names), so a contact who is already online lights up immediately
/// — whether you logged in after them *or* just added them.
///
/// Mirrors `fanout_login_status` in the opposite direction: that notifies
/// watchers about one player; this notifies one player about their contacts.
/// Online status is read from the in-memory `connected` sessions (by character
/// name), so no DB round-trip is needed.
pub(crate) async fn notify_online_contacts(
    recipient_entity_id: u32,
    candidate_names: &[String],
    transport: &Arc<dyn Transport>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
) {
    // Character names of all currently-online players except the recipient.
    let online_names = match connected.lock() {
        Ok(clients) => collect_online_names(&clients, recipient_entity_id),
        Err(_) => {
            tracing::error!(
                recipient_entity_id,
                "ContactList presence: connected lock poisoned"
            );
            return;
        }
    };
    if online_names.is_empty() {
        return;
    }

    let to_notify = dedup_online(candidate_names, &online_names);
    for name in &to_notify {
        let args = build_on_contact_list_event(name, EVENT_LOGGED_IN_STATUS, DATA_ONLINE);
        send_to_witness_reliable(
            transport,
            connected,
            entity_to_addr,
            recipient_entity_id,
            |key, version, seq, acks| {
                build_player_entity_method_packet(
                    key,
                    seq,
                    acks,
                    recipient_entity_id,
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
            recipient_entity_id,
            online_contacts = to_notify.len(),
            "ContactList: sent online presence for already-online contacts"
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

/// Distinct `candidate_names` that appear in `online_names`, preserving
/// first-seen order. Pure helper, unit-tested separately. Deduplicates a name
/// that appears more than once (e.g. present in two of the player's lists) so
/// the client gets one event per online contact.
fn dedup_online(candidate_names: &[String], online_names: &HashSet<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for name in candidate_names {
        if online_names.contains(name) && seen.insert(name.as_str()) {
            out.push(name.clone());
        }
    }
    out
}

/// Flatten the member names of the player's 'Ignore' list(s) into a set. Pure
/// helper, unit-tested; seeds the in-memory `ignore_set` at login.
fn ignore_names_from_lists(lists: &[ContactList]) -> HashSet<String> {
    lists
        .iter()
        .filter(|l| l.flags == IGNORE_LIST_FLAGS)
        .flat_map(|l| l.members.iter().cloned())
        .collect()
}

#[cfg(test)]
mod presence_login_tests {
    use super::*;

    #[test]
    fn ignore_names_from_lists_picks_only_flag_301() {
        let lists = vec![
            ContactList {
                list_id: 1,
                name: "Friends".into(),
                flags: 300,
                members: vec!["Pal".into()],
            },
            ContactList {
                list_id: 2,
                name: "Ignore".into(),
                flags: IGNORE_LIST_FLAGS,
                members: vec!["BadGuy".into(), "Jerk".into()],
            },
        ];
        let got = ignore_names_from_lists(&lists);
        assert_eq!(got.len(), 2);
        assert!(got.contains("BadGuy") && got.contains("Jerk"));
        assert!(!got.contains("Pal"), "Friends (300) must be excluded");
    }

    fn names(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }
    fn online_set(v: &[&str]) -> HashSet<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn dedup_online_keeps_only_online_in_order() {
        let online = online_set(&["Friendly", "BadGuy", "Stranger"]);
        // Candidate order preserved; Offliner dropped (not online).
        let got = dedup_online(&names(&["Friendly", "Offliner", "BadGuy"]), &online);
        assert_eq!(got, names(&["Friendly", "BadGuy"]));
    }

    #[test]
    fn dedup_online_deduplicates_repeats() {
        let online = online_set(&["Dup"]);
        // Same name appearing twice (e.g. flattened from two lists) → one event.
        assert_eq!(
            dedup_online(&names(&["Dup", "Dup"]), &online),
            names(&["Dup"])
        );
    }

    #[test]
    fn dedup_online_empty_when_none_online() {
        let online = online_set(&["C"]);
        assert!(dedup_online(&names(&["A", "B"]), &online).is_empty());
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
    async fn notify_online_contacts_sends_for_online_contact() {
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

        notify_online_contacts(
            10,
            &names(&["Friendly", "Offliner"]),
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        // The recipient gets exactly one CM 89 (for the online Friendly); nothing
        // is sent to the friend in this reverse sync. Reverting the
        // notify_online_contacts call makes `to_me` empty.
        assert_eq!(
            tt.filter_to(me_addr).len(),
            1,
            "logging-in player must get one onContactListEvent for the online contact"
        );
        assert!(tt.filter_to(friend_addr).is_empty());
    }

    #[tokio::test]
    async fn notify_online_contacts_silent_when_none_online() {
        let me_addr: SocketAddr = "127.0.0.1:5003".parse().unwrap();
        let mut me = test_default_connected_client_state();
        me.player_entity_id = Some(10);
        let connected = Arc::new(Mutex::new(HashMap::from([(me_addr, me)])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(10u32, me_addr)])));

        let tt = Arc::new(TestTransport::new());
        let transport: Arc<dyn Transport> = tt.clone();

        notify_online_contacts(
            10,
            &names(&["Friendly"]),
            &transport,
            &connected,
            &entity_to_addr,
        )
        .await;

        assert!(
            tt.drain().is_empty(),
            "no events when none of the player's contacts are online"
        );
    }
}
