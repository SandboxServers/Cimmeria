//! Disconnect and relog mid-push, and holding world entry.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::super::super::resources::ResourceCache;
use super::super::{defer_until_synced, resync_pending_version, DeferredAction, VersionReply};
use super::{decode, rig, server_entries, server_version, ClientModel, Sent};
use crate::test_support::{test_default_connected_client_state, LogCapture};

const MISSIONS: u32 = 3;
const WORLD_INFO: u32 = 12;

fn flag_action(flag: &Arc<AtomicBool>) -> DeferredAction {
    let flag = Arc::clone(flag);
    Box::new(move || {
        Box::pin(async move {
            flag.store(true, Ordering::SeqCst);
        })
    })
}

/// A client that disconnects part-way keeps the placeholder version, so its
/// next login is a mismatch and resyncs; the push stops with a WARN, the
/// closing stamp never goes out, and held world entry is dropped.
#[tokio::test]
async fn disconnect_mid_push_abandons_and_the_next_login_resyncs() {
    let rig = rig(47_301);
    let capture = LogCapture::install();
    let served = server_version(MISSIONS);
    rig.request(MISSIONS, served.wrapping_add(1)).await;
    let entered = Arc::new(AtomicBool::new(false));
    assert!(defer_until_synced(&rig.connected, rig.addr, flag_action(&entered)).is_ok());
    rig.idle_turns(200).await;
    rig.ack_all();
    rig.idle_turns(200).await;

    rig.connected.lock().unwrap().remove(&rig.addr);
    rig.idle_turns(200).await;

    assert!(!rig.syncing());
    assert!(
        !entered.load(Ordering::SeqCst),
        "world entry is dropped with the session"
    );
    let plaintexts = rig.take_plaintexts();
    assert!(
        plaintexts.len() < 100,
        "the push stopped ({} packets)",
        plaintexts.len()
    );
    let stamped = plaintexts
        .iter()
        .any(|pt| matches!(decode(pt), Sent::VersionInfo(v) if v.version == served));
    assert!(
        !stamped,
        "the server's version must not be stamped on a partial push"
    );

    let mut client =
        ClientModel::holding(MISSIONS, served.wrapping_add(1), server_entries(MISSIONS));
    client.apply_all(&plaintexts);
    let held = client.categories[&MISSIONS].version;
    assert_eq!(held, resync_pending_version(served));
    let cache: Arc<ResourceCache> = rig.cache.clone();
    assert!(matches!(
        VersionReply::decide(Some(&cache), MISSIONS, held),
        VersionReply::FullResync { .. }
    ));

    let warn = capture
        .all()
        .into_iter()
        .find(|e| {
            e.level == tracing::Level::WARN
                && e.has_field("event", "cooked_data.sync_finish")
                && e.has_field("outcome", "abandoned")
        })
        .unwrap_or_else(|| panic!("no abandoned WARN; saw {:#?}", capture.all()));
    assert!(warn.has_field("reason", "session_gone"));
    assert!(warn.has_field("category_id", &MISSIONS.to_string()));
}

/// A relog from the same address mid-push: the old session's push stops,
/// and the new session's request (at the placeholder version the first push
/// stamped) resyncs to the server's table.
#[tokio::test]
async fn relog_mid_push_resyncs_for_the_new_session() {
    let rig = rig(47_302);
    let served = server_version(WORLD_INFO);
    rig.request(WORLD_INFO, 5959).await;
    rig.idle_turns(200).await;
    let first = rig.take_plaintexts();
    let mut client = ClientModel::holding(WORLD_INFO, 5959, server_entries(WORLD_INFO));
    client.apply_all(&first);
    let held = client.categories[&WORLD_INFO].version;
    assert_eq!(held, resync_pending_version(served), "stalled mid-push");

    // New session at the same address.
    rig.connected
        .lock()
        .unwrap()
        .insert(rig.addr, test_default_connected_client_state());
    rig.idle_turns(200).await;
    rig.request(WORLD_INFO, held).await;
    rig.pump_until_idle().await;

    client.apply_all(&rig.take_plaintexts());
    let state = &client.categories[&WORLD_INFO];
    assert_eq!(state.version, served);
    assert!(state.entries == server_entries(WORLD_INFO));
}

/// A second request for a category already being pushed adds nothing.
#[tokio::test]
async fn a_repeated_request_during_the_push_is_ignored() {
    let rig = rig(47_303);
    rig.request(WORLD_INFO, 5959).await;
    rig.request(WORLD_INFO, 5959).await;
    rig.pump_until_idle().await;
    let invalidations = rig
        .take_plaintexts()
        .iter()
        .filter(|pt| matches!(decode(pt), Sent::VersionInfo(v) if v.invalidate_all))
        .count();
    assert_eq!(invalidations, 1);
}

/// World entry waits for the push and runs once it is done.
#[tokio::test]
async fn held_world_entry_runs_after_the_push() {
    let rig = rig(47_304);
    rig.request(WORLD_INFO, 5959).await;
    let entered = Arc::new(AtomicBool::new(false));
    assert!(defer_until_synced(&rig.connected, rig.addr, flag_action(&entered)).is_ok());
    rig.idle_turns(50).await;
    assert!(!entered.load(Ordering::SeqCst), "held while the push runs");
    rig.pump_until_idle().await;
    rig.idle_turns(50).await;
    assert!(
        entered.load(Ordering::SeqCst),
        "released when the push finishes"
    );
}

/// With no push running, nothing is held.
#[tokio::test]
async fn nothing_is_held_without_a_push() {
    let rig = rig(47_305);
    rig.request(WORLD_INFO, server_version(WORLD_INFO)).await;
    let entered = Arc::new(AtomicBool::new(false));
    let handed_back = defer_until_synced(&rig.connected, rig.addr, flag_action(&entered));
    assert!(handed_back.is_err(), "the caller runs it straight away");
}
