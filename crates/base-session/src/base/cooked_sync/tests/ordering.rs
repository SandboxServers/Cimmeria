//! Stream order, the held set, and in-world addressing.

use super::super::{holds_world_entry, is_held, HELD_CATEGORIES};
use super::{decode, rig, server_entries, server_version, ClientModel, Sent};

/// Categories are pushed held-first, then missions, dialogs and items, with
/// TextStrings last, whatever order the client asked in.
#[tokio::test]
async fn categories_stream_in_rank_order() {
    let rig = rig(47_701);
    for category_id in [10, 2, 4, 5, 3, 12, 16] {
        rig.request(category_id, server_version(category_id).wrapping_add(1))
            .await;
    }
    rig.pump_until_idle().await;
    let opened: Vec<u32> = rig
        .take_plaintexts()
        .iter()
        .filter_map(|pt| match decode(pt) {
            Sent::VersionInfo(v) if v.invalidate_all => Some(v.category),
            _ => None,
        })
        .collect();
    // All seven were queued before the task first ran.
    assert_eq!(opened, vec![12, 16, 3, 5, 4, 2, 10]);
}

/// Without a head start, a held category goes first and TextStrings last.
#[tokio::test]
async fn text_strings_queue_behind_everything() {
    let rig = rig(47_702);
    rig.request(12, 5959).await; // starts the task
    for category_id in [10, 3, 5] {
        rig.request(category_id, server_version(category_id).wrapping_add(1))
            .await;
    }
    rig.pump_until_idle().await;
    let opened: Vec<u32> = rig
        .take_plaintexts()
        .iter()
        .filter_map(|pt| match decode(pt) {
            Sent::VersionInfo(v) if v.invalidate_all => Some(v.category),
            _ => None,
        })
        .collect();
    assert_eq!(opened, vec![12, 3, 5, 10]);
}

/// Only the categories with no client miss path hold world entry.
#[tokio::test]
async fn only_the_held_set_holds_world_entry() {
    assert_eq!(HELD_CATEGORIES, [12, 16, 17, 18, 20, 21]);
    for c in 1..=21u32 {
        assert_eq!(is_held(c), HELD_CATEGORIES.contains(&c));
    }

    let rig = rig(47_703);
    rig.request(3, server_version(3).wrapping_add(1)).await;
    rig.request(10, server_version(10).wrapping_add(1)).await;
    assert!(rig.syncing());
    assert!(
        !holds_world_entry(&rig.connected, rig.addr),
        "missions and TextStrings stream in the world"
    );
    rig.request(12, 5959).await;
    assert!(holds_world_entry(&rig.connected, rig.addr));
}

/// Once the player is in the world, the replies go to the player as
/// SGWPlayer client method 96, and the client still converges.
#[tokio::test]
async fn in_world_replies_address_the_player() {
    let rig = rig(47_704);
    rig.enter_world(4242);
    let served = server_version(3);
    rig.request(3, served.wrapping_add(1)).await;
    rig.pump_until_idle().await;
    let plaintexts = rig.take_plaintexts();
    let replies: Vec<&Vec<u8>> = plaintexts
        .iter()
        .filter(|pt| matches!(decode(pt), Sent::VersionInfo(_)))
        .collect();
    assert_eq!(replies.len(), 2);
    for pt in replies {
        assert_eq!(pt[1], 0xBD, "extended entity method, not Account's 0x80");
        assert_eq!(&pt[4..8], &4242u32.to_le_bytes());
        assert_eq!(pt[8], 35);
    }
    let mut client = ClientModel::default();
    client.apply_all(&plaintexts);
    assert!(client.categories[&3].entries == server_entries(3));
    assert_eq!(client.categories[&3].version, served);
}
