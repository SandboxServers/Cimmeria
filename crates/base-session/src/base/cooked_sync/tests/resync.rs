//! A mismatched client ends up holding exactly the server's category.

use std::collections::BTreeMap;

use super::super::{resync_pending_version, start_resync, SyncJob};
use super::{
    decode, rig, server_entries, server_version, test_context, ClientModel, Rig, Sent, VersionInfo,
};

/// Start the resync of `category` for a client holding `client_version`,
/// driving it through the test context (zero poll interval).
fn start(rig: &Rig, category_id: u32, client_version: u32) {
    start_resync(
        test_context(rig),
        SyncJob {
            category_id,
            client_version,
            server_version: server_version(category_id),
        },
    );
}

/// Resync `category` against a client holding `stale` at `client_version`
/// and check the whole exchange: the opening reply, exactly one transfer
/// per server entry, the closing stamp, and what the client ends with.
async fn assert_resync_converges(port: u16, category_id: u32, stale: BTreeMap<u32, Vec<u8>>) {
    let rig = rig(port);
    let served = server_version(category_id);
    let server = server_entries(category_id);
    let client_version = served.wrapping_add(7);

    start(&rig, category_id, client_version);
    rig.pump_until_idle().await;
    let plaintexts = rig.take_plaintexts();
    let sent: Vec<Sent> = plaintexts.iter().map(|pt| decode(pt)).collect();

    assert_eq!(
        sent.first(),
        Some(&Sent::VersionInfo(VersionInfo {
            category: category_id,
            version: resync_pending_version(served),
            required: 0,
            invalidate_all: true,
            keys: vec![],
        })),
        "category {category_id}: opens with InvalidateAll, RequiredUpdates = 0 (so the client asks for misses) and the placeholder"
    );
    assert_eq!(
        sent.last(),
        Some(&Sent::VersionInfo(VersionInfo {
            category: category_id,
            version: served,
            required: 0,
            invalidate_all: false,
            keys: vec![],
        })),
        "category {category_id}: closes by stamping the server's version"
    );
    let transfers = sent
        .iter()
        .filter(|s| matches!(s, Sent::Fragment { head: Some(_), .. }))
        .count();
    assert_eq!(
        transfers,
        server.len(),
        "category {category_id}: exactly one transfer per entry"
    );

    let mut client = ClientModel::holding(category_id, client_version, stale);
    client.apply_all(&plaintexts);
    let held = &client.categories[&category_id];
    assert_eq!(held.version, served, "category {category_id}: version");
    assert_eq!(
        held.required_updates, 0,
        "category {category_id}: RequiredUpdates drained"
    );
    assert!(
        held.entries == server,
        "category {category_id}: the client must hold exactly the server's entries \
         (held {}, server {})",
        held.entries.len(),
        server.len()
    );
}

/// A stale copy of `category`: one entry the server lacks, one entry
/// missing, one entry with different content.
fn stale_copy(category_id: u32) -> BTreeMap<u32, Vec<u8>> {
    let mut stale = server_entries(category_id);
    stale.insert(u32::MAX - 1, b"<gone-from-the-server/>".to_vec());
    if let Some(first) = stale.keys().next().copied() {
        stale.remove(&first);
    }
    if let Some(last) = stale.keys().rev().nth(1).copied() {
        stale.insert(last, b"<changed/>".to_vec());
    }
    stale
}

/// Category 12 has Cimmeria overrides (the historical CellBlocks); a
/// shipped client's table is replaced with the server's, overrides included.
#[tokio::test]
async fn world_info_resync_converges_on_the_server_table() {
    assert_resync_converges(47_101, 12, stale_copy(12)).await;
}

/// Category 2 has no override list: before #840 this mismatch emptied the
/// client's ability table and pushed nothing.
#[tokio::test]
async fn abilities_resync_converges_on_the_server_table() {
    assert_resync_converges(47_102, 2, stale_copy(2)).await;
}

/// Missions have multi-fragment entries (up to 7.4 KB, six fragments).
#[tokio::test]
async fn multi_fragment_entries_arrive_whole() {
    assert_resync_converges(47_103, 3, stale_copy(3)).await;
}

/// `CookedCharCreation` is one 169 KB entry: 122 fragments in one transfer.
#[tokio::test]
async fn a_122_fragment_entry_arrives_whole() {
    assert_resync_converges(47_104, 7, BTreeMap::new()).await;
}

/// Category 21 ships empty: the resync still stamps the version.
#[tokio::test]
async fn an_empty_category_is_stamped() {
    assert_resync_converges(47_105, 21, stale_copy(21)).await;
}

/// Regression guard for #840: queue every client category at a mismatched
/// version and check that no category is ever invalidated without being
/// fully repopulated. The pre-#840 branch (InvalidateAll with nothing
/// pushed) fails it on the first category without an override list.
#[tokio::test]
async fn no_category_is_ever_invalidated_without_being_repopulated() {
    let rig = rig(47_106);
    for category_id in 1..=21u32 {
        rig.request(category_id, server_version(category_id).wrapping_add(1))
            .await;
    }
    rig.pump_until_idle().await;

    let plaintexts = rig.take_plaintexts();
    let mut client = ClientModel::default();
    let mut invalidated = Vec::new();
    for pt in &plaintexts {
        let sent = decode(pt);
        if let Sent::VersionInfo(v) = &sent {
            if v.invalidate_all {
                invalidated.push(v.category);
            }
        }
        client.apply(&sent);
    }
    assert_eq!(
        invalidated.len(),
        21,
        "every mismatched category is resynced once"
    );
    for category_id in invalidated {
        let held = &client.categories[&category_id];
        let server = server_entries(category_id);
        assert!(
            held.entries == server,
            "category {category_id} invalidated but left with {} of {} entries",
            held.entries.len(),
            server.len()
        );
        assert_eq!(held.version, server_version(category_id));
        assert_eq!(
            client.writes.get(&category_id).copied().unwrap_or(0) as usize,
            server.len()
        );
    }
}
