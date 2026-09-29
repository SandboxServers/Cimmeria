//! Every reverse-engineering refusal, through the base's request entry
//! point: the player reads why, nothing is consumed or queued, and
//! `rejected` carries the reason and the identity.

use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::session::crafting_sessions;
use crate::base::crafting::telemetry::METRIC_REJECTIONS;
use crate::base::crafting::test_verbs::VerbFixture;
use crate::cell::messages::CraftVerb;
use crate::test_support::{require_db_or_skip, LogCapture, LogCaptureGuard};
use cimmeria_observability::testing::{counter_total, install as install_meter};

/// Steel Core: not reverse-engineerable.
const PLAIN: i32 = 5254;
/// Ambernol Vial: flagged reverse-engineerable, made by no blueprint.
const ORPHAN: i32 = 21;

async fn assert_refused(f: &VerbFixture, capture: &LogCaptureGuard, item: i32, why: CraftReject) {
    let labels = [("verb", "reverseEngineer"), ("reason", why.reason())];
    let rejections_before = counter_total(METRIC_REJECTIONS, &labels);
    let inventory = f.inventory().await;

    f.request(CraftVerb::ReverseEngineer { item_id: item })
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
        ("verb", "reverseEngineer".to_string()),
        ("account_id", f.account_id.to_string()),
        ("entity_id", f.entity_id.to_string()),
        ("item_id", item.to_string()),
    ] {
        assert!(rejected.has_field(k, &v), "{k}={v}: {rejected:#?}");
    }
}

#[tokio::test]
async fn live_db_an_item_that_is_not_reverse_engineerable_is_refused() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 30).await;
    let item = f.stack(PLAIN, 15, 0).await;
    let why = CraftReject::NotReverseEngineerable {
        item_id: item,
        type_id: PLAIN,
    };
    assert_refused(&f, &capture, item, why).await;
    f.cleanup().await;
}

#[tokio::test]
async fn live_db_an_item_no_blueprint_makes_is_refused() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 31).await;
    let item = f.stack(ORPHAN, 1, 0).await;
    let why = CraftReject::NoBlueprintForItem {
        item_id: item,
        type_id: ORPHAN,
    };
    assert_refused(&f, &capture, item, why).await;
    f.cleanup().await;
}

#[tokio::test]
async fn live_db_an_item_in_the_bank_is_refused() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 32).await;
    let item = f.stack(5481, 17, 0).await;
    let why = CraftReject::ComponentNotInCraftingBags {
        item_id: item,
        container_id: 17,
    };
    assert_refused(&f, &capture, item, why).await;
    f.cleanup().await;
}
