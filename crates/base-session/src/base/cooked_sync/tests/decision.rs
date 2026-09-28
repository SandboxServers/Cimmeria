//! The decision, branch by branch.

use super::super::{resync_pending_version, VersionReply};
use super::{committed_cache, server_version};

/// `CookedDataAbilities.pak`: served, no Cimmeria overrides.
const ABILITIES: u32 = 2;
/// `CookedWorldInfo.pak`: served, with the historical CellBlock overrides.
const WORLD_INFO: u32 = 12;

#[test]
fn no_cache_echoes_the_client_version() {
    let reply = VersionReply::decide(None, ABILITIES, 42);
    assert_eq!(reply, VersionReply::NoServerData { client_version: 42 });
    assert_eq!(reply.server_version(), None);
    assert_eq!(reply.reason(), "category_not_served");
}

#[test]
fn unserved_category_echoes_the_client_version() {
    let cache = committed_cache();
    assert_eq!(
        VersionReply::decide(Some(&cache), 99, 42),
        VersionReply::NoServerData { client_version: 42 }
    );
}

#[test]
fn matching_version_is_up_to_date() {
    let cache = committed_cache();
    let served = server_version(ABILITIES);
    let reply = VersionReply::decide(Some(&cache), ABILITIES, served);
    assert_eq!(reply, VersionReply::UpToDate { version: served });
    assert_eq!(reply.outcome(), "up_to_date");
}

/// No override list: a mismatch resyncs the whole category.
#[test]
fn mismatch_without_overrides_resyncs_the_whole_category() {
    let cache = committed_cache();
    assert!(cache.overridden_elements(ABILITIES).is_empty());
    let served = server_version(ABILITIES);
    let reply = VersionReply::decide(Some(&cache), ABILITIES, served.wrapping_add(1));
    assert_eq!(
        reply,
        VersionReply::FullResync {
            client_version: served.wrapping_add(1),
            server_version: served,
            entry_count: cache.category(ABILITIES).unwrap().elements.len() as u32,
        }
    );
    assert_eq!(reply.reason(), "version_mismatch");
}

/// An override category resyncs whole too, overrides included: the entry
/// count is the category as served, not the override list.
#[test]
fn mismatch_with_overrides_resyncs_the_whole_category() {
    let cache = committed_cache();
    let overrides = cache.overridden_elements(WORLD_INFO).len();
    assert!(overrides > 0);
    let served = server_version(WORLD_INFO);
    let VersionReply::FullResync { entry_count, .. } =
        VersionReply::decide(Some(&cache), WORLD_INFO, 5959)
    else {
        panic!("a shipped client's world table must be resynced");
    };
    assert_eq!(
        entry_count as usize,
        cache.category(WORLD_INFO).unwrap().elements.len()
    );
    assert!(entry_count as usize > overrides);
    assert_ne!(served, 5959);
}

/// The placeholder a resync stamps first can never pass for the server's
/// version, so a client that disconnects mid-push resyncs next time.
#[test]
fn placeholder_version_never_matches_the_server() {
    let cache = committed_cache();
    for category_id in 1..=21u32 {
        let served = server_version(category_id);
        let pending = resync_pending_version(served);
        assert_ne!(pending, served);
        assert!(matches!(
            VersionReply::decide(Some(&cache), category_id, pending),
            VersionReply::FullResync { .. }
        ));
    }
    for v in [0, 1, u32::MAX, 0x8000_0000] {
        assert_ne!(resync_pending_version(v), v);
    }
}
