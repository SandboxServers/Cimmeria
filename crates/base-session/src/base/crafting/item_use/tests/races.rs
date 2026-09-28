//! Replays and races: a use is decided under the item's row lock, so a
//! repeated, concurrent or foreign use never teaches twice or consumes
//! another player's item.

use cimmeria_entity::inventory::INV_CRAFTING;

use super::*;
use crate::test_support::require_db_or_skip;

/// Replay: the same use twice teaches once and consumes once. The second is
/// refused because the item is gone, and writes nothing.
#[tokio::test]
async fn live_db_replayed_use_teaches_once() {
    let pool = require_db_or_skip!();
    let slot = Slot::new(8);
    player(&pool, slot, &[], None).await;
    give(
        &pool,
        slot.item,
        slot.player_id,
        GOAULD_GUIDE,
        INV_CRAFTING,
        1,
    )
    .await;
    let session = OneSession::new(slot.entity(), 55800);

    let first = use_item(Some(&pool), &session, slot, slot.item).await;
    let after_first = crafting(&pool, slot.player_id).await;
    let second = use_item(Some(&pool), &session, slot, slot.item).await;
    let after_second = crafting(&pool, slot.player_id).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, slot).await;

    assert!(first.is_some() && second.is_none());
    assert_eq!(after_first.1[2], (3, 2));
    assert_eq!(after_second, after_first, "the replay changed nothing");
    assert_eq!(sent.len(), 2, "138, then one refusal line");
    assert_eq!(
        sent[1],
        refusal(
            slot.entity(),
            1,
            "That item is no longer in your inventory."
        )
    );
}

/// Two uses of a one-item stack at once: one teaches and deletes the row,
/// the other waits on the row lock, finds it gone and is refused.
#[tokio::test]
async fn live_db_concurrent_uses_of_one_item_teach_once() {
    let pool = require_db_or_skip!();
    let slot = Slot::new(9);
    player(&pool, slot, &[], None).await;
    give(
        &pool,
        slot.item,
        slot.player_id,
        GOAULD_GUIDE,
        INV_CRAFTING,
        1,
    )
    .await;
    let a = OneSession::new(slot.entity(), 55801);
    let b = OneSession::new(slot.entity(), 55802);

    let (x, y) = tokio::join!(
        use_item(Some(&pool), &a, slot, slot.item),
        use_item(Some(&pool), &b, slot, slot.item)
    );

    let (_, levels) = crafting(&pool, slot.player_id).await;
    let left = instance(&pool, slot.item).await;
    let outbox = outbox_rows(&pool, slot).await;
    cleanup(&pool, slot).await;
    assert_eq!(
        usize::from(x.is_some()) + usize::from(y.is_some()),
        1,
        "exactly one use committed"
    );
    assert_eq!(levels[2], (3, 2), "raised once");
    assert_eq!(left, None);
    assert_eq!(outbox, vec!["inventory_item_removed".to_string()]);
}

/// An instance id of another character's item is refused as missing: the
/// owner keeps it and the user learns nothing. The locked read names the
/// owner, so an item traded away after the caller's ownership check fares
/// the same.
#[tokio::test]
async fn live_db_another_characters_item_is_refused() {
    let pool = require_db_or_skip!();
    let user = Slot::new(10);
    let owner = Slot::new(11);
    player(&pool, user, &[], None).await;
    player(&pool, owner, &[], None).await;
    give(
        &pool,
        owner.item,
        owner.player_id,
        STEEL_PLATING,
        INV_CRAFTING,
        1,
    )
    .await;
    let session = OneSession::new(user.entity(), 55803);

    let consumed = use_item(Some(&pool), &session, user, owner.item).await;

    let user_after = crafting(&pool, user.player_id).await;
    let owner_after = crafting(&pool, owner.player_id).await;
    let left = instance(&pool, owner.item).await;
    let sent = session.typed.filter_to(session.addr);
    cleanup(&pool, user).await;
    cleanup(&pool, owner).await;
    assert!(consumed.is_none());
    assert!(user_after.0.is_empty() && owner_after.0.is_empty());
    assert_eq!(left, Some((owner.player_id, 1)), "the owner keeps it");
    assert_eq!(
        sent,
        vec![refusal(
            user.entity(),
            0,
            "That item is no longer in your inventory."
        )]
    );
}
