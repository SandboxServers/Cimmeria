//! Every research refusal, through the base's request entry point: the
//! player reads why, nothing is consumed or queued, and `rejected` carries
//! the reason and the identity.

use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::session::crafting_sessions;
use crate::base::crafting::telemetry::METRIC_REJECTIONS;
use crate::base::crafting::test_verbs::VerbFixture;
use crate::cell::messages::CraftVerb;
use crate::test_support::{require_db_or_skip, LogCapture, LogCaptureGuard};
use cimmeria_observability::testing::{counter_total, install as install_meter};

const ITEM: i32 = 5481;
/// Applied science 1, the same as the item's.
const KICKER_SAME: i32 = 5668;
/// Applied science 4.
const KICKER: i32 = 5669;
/// Steel Core: neither researchable nor a kicker.
const PLAIN: i32 = 5254;
/// Applied science 4, but not flagged a kicker.
const SCIENCE_NOT_KICKER: i32 = 2925;

/// Request a research of `item` with `kickers` and assert the refusal:
/// exactly one line, `why`'s text; the inventory unchanged; nothing
/// queued; `rejected` with `why`'s reason, the identity and `fields`; one
/// more rejection counted.
async fn assert_refused(
    f: &VerbFixture,
    capture: &LogCaptureGuard,
    item: i32,
    kickers: Vec<i32>,
    why: CraftReject,
    fields: &[(&str, String)],
) {
    let labels = [("verb", "research"), ("reason", why.reason())];
    let rejections_before = counter_total(METRIC_REJECTIONS, &labels);
    let inventory = f.inventory().await;

    f.request(CraftVerb::Research {
        item_id: item,
        kickers,
    })
    .await;

    assert_eq!(f.lines(), vec![why.text()]);
    assert_eq!(f.inventory().await, inventory, "nothing is consumed");
    assert_eq!(
        crafting_sessions().pending(f.entity_id),
        0,
        "nothing queued"
    );
    assert!(counter_total(METRIC_REJECTIONS, &labels) > rejections_before);
    let rejected = capture
        .all()
        .into_iter()
        .find(|c| {
            c.target == "crafting"
                && c.has_field("event", "rejected")
                && c.has_field("player_id", &f.player_id.to_string())
        })
        .expect("rejected event");
    for (k, v) in [
        ("reason", why.reason().to_string()),
        ("verb", "research".to_string()),
        ("account_id", f.account_id.to_string()),
        ("entity_id", f.entity_id.to_string()),
    ]
    .iter()
    .chain(fields)
    {
        assert!(rejected.has_field(k, v), "{k}={v}: {rejected:#?}");
    }
}

#[tokio::test]
async fn live_db_an_item_that_is_not_researchable_is_refused() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 10).await;
    let item = f.stack(PLAIN, 15, 0).await;
    let why = CraftReject::NotResearchable {
        item_id: item,
        type_id: PLAIN,
    };
    let fields = [
        ("item_id", item.to_string()),
        ("type_id", PLAIN.to_string()),
    ];
    assert_refused(&f, &capture, item, vec![], why, &fields).await;
    f.cleanup().await;
}

#[tokio::test]
async fn live_db_a_kicker_that_is_not_a_kicker_is_refused() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 11).await;
    let item = f.stack(ITEM, 15, 0).await;
    let kicker = f.stack(SCIENCE_NOT_KICKER, 15, 1).await;
    let why = CraftReject::NotKicker {
        item_id: kicker,
        type_id: SCIENCE_NOT_KICKER,
    };
    let fields = [("item_id", kicker.to_string())];
    assert_refused(&f, &capture, item, vec![kicker], why, &fields).await;
    f.cleanup().await;
}

#[tokio::test]
async fn live_db_a_kicker_of_the_items_own_science_is_refused() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 12).await;
    let item = f.stack(ITEM, 15, 0).await;
    let kicker = f.stack(KICKER_SAME, 1, 0).await;
    let why = CraftReject::KickerSameScience {
        item_id: kicker,
        type_id: KICKER_SAME,
        applied_science_id: 1,
    };
    let fields = [("applied_science_id", "1".to_string())];
    assert_refused(&f, &capture, item, vec![kicker], why, &fields).await;
    f.cleanup().await;
}

#[tokio::test]
async fn live_db_two_kickers_of_one_science_are_refused() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 13).await;
    let item = f.stack(ITEM, 15, 0).await;
    let first = f.stack(KICKER, 1, 0).await;
    let second = f.stack(KICKER, 1, 1).await;
    let why = CraftReject::KickerDuplicateScience {
        item_id: second,
        type_id: KICKER,
        applied_science_id: 4,
    };
    let fields = [
        ("item_id", second.to_string()),
        ("applied_science_id", "4".to_string()),
    ];
    assert_refused(&f, &capture, item, vec![first, second], why, &fields).await;
    f.cleanup().await;
}

/// An item in the bank, and an item id the player does not hold.
#[tokio::test]
async fn live_db_an_item_outside_the_crafting_bags_or_gone_is_refused() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 14).await;
    let banked = f.stack(ITEM, 17, 0).await;
    let why = CraftReject::ComponentNotInCraftingBags {
        item_id: banked,
        container_id: 17,
    };
    let fields = [("container_id", "17".to_string())];
    assert_refused(&f, &capture, banked, vec![], why, &fields).await;

    let g = VerbFixture::new(&pool, 15).await;
    let gone = banked + 1_000_000;
    let why = CraftReject::ComponentMissing { item_id: gone };
    assert_refused(&g, &capture, gone, vec![], why, &[]).await;
    f.cleanup().await;
    g.cleanup().await;
}
