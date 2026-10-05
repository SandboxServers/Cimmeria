//! Live-DB: every alloy refusal is visible (the line, then an inventory
//! resync), consumes nothing, queues nothing, and logs `rejected` with its
//! reason, the values compared and the player's identity.

use cimmeria_observability::testing::{counter_total, install as install_meter};

use super::fixture::*;
use crate::base::crafting::telemetry::METRIC_REJECTIONS;
use crate::base::crafting::test_packets::feedback_text;
use crate::mercury::method_idx;
use crate::test_support::{require_db_or_skip, Captured, LogCapture};

/// Send the request and check the refusal's whole shape; returns the
/// `rejected` event for the rule's own fields.
async fn assert_refused(
    f: &Fixture,
    blueprint_id: i32,
    current: i32,
    lower: &[i32],
    reason: &str,
    text: &str,
) -> Captured {
    install_meter();
    let labels = [("verb", "alloying"), ("reason", reason)];
    let counted_before = counter_total(METRIC_REJECTIONS, &labels);
    let before = f.inventory().await;
    f.transport.clear();
    let capture = LogCapture::install();

    f.alloy(blueprint_id, current, lower).await;

    assert_eq!(
        f.sessions.pending(f.entity_id),
        0,
        "{reason}: nothing queued"
    );
    assert_eq!(f.inventory().await, before, "{reason}: nothing consumed");
    let calls = f.calls();
    let methods: Vec<u16> = calls.iter().map(|c| c.method).collect();
    assert_eq!(
        methods,
        vec![
            method_idx::ON_PLAYER_COMMUNICATION,
            method_idx::ON_UPDATE_ITEM
        ],
        "{reason}: the line, then the resync"
    );
    assert_eq!(feedback_text(&calls[0]), text);
    assert_eq!(
        counter_total(METRIC_REJECTIONS, &labels) - counted_before,
        1,
        "{reason}: counted once"
    );
    let rejected = capture
        .all()
        .into_iter()
        .find(|e| e.target == "crafting" && e.has_field("event", "rejected"))
        .unwrap_or_else(|| panic!("{reason}: no rejected event"));
    for (key, value) in [
        ("verb", "alloying".to_string()),
        ("reason", reason.to_string()),
        ("account_id", f.account_id.to_string()),
        ("player_id", f.player_id.to_string()),
        ("entity_id", f.entity_id.to_string()),
    ] {
        assert!(
            rejected.has_field(key, &value),
            "{key}={value}: {rejected:#?}"
        );
    }
    rejected
}

/// The tier guard, live: one tier-2 item among ten valid Normals.
#[tokio::test]
async fn live_db_an_elementary_one_tier_too_high_is_refused() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 10).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    // Nine Normals and a tier-2 item: the ten slots the page has.
    let mut lower = f.singles(NORMAL, 0, 9).await;
    let wrong = f.stack(TIER_TWO, INV_MAIN, 20, 1).await;
    lower.push(wrong);
    let e = assert_refused(
        &f,
        ALLOY,
        component,
        &lower,
        "wrong_tier",
        "Elementary components must be one tier lower than the component (tier 1). Nothing was used.",
    )
    .await;
    assert!(e.has_field("item_id", &wrong.to_string()), "{e:#?}");
    assert!(
        e.has_field("tier", "2") && e.has_field("required_tier", "1"),
        "{e:#?}"
    );
    f.cleanup().await;
}

const COUNT_LINE: &str = "The quantity of elementary components per item quality was not met: 10 Normal, 5 Good, 2 Great or 1 Fantastic. Nothing was used.";

/// One short of each quality's count, and Poor, which has no count.
#[tokio::test]
async fn live_db_each_count_one_short_is_refused() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 11).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    let cases = [
        (NORMAL, 9, "normal:9,good:0,great:0,fantastic:0"),
        (GOOD, 4, "normal:0,good:4,great:0,fantastic:0"),
        (GREAT, 1, "normal:0,good:0,great:1,fantastic:0"),
        (POOR, 10, "normal:0,good:0,great:0,fantastic:0"),
    ];
    for (type_id, quantity, counts) in cases {
        // One stack of the quantity: the count is summed stack quantity.
        let stack = f.stack(type_id, INV_MAIN, 0, quantity).await;
        let e = assert_refused(&f, ALLOY, component, &[stack], "count_not_met", COUNT_LINE).await;
        assert!(e.has_field("elementary_counts", counts), "{e:#?}");
        sqlx::query("DELETE FROM sgw_inventory WHERE item_id = $1")
            .bind(stack)
            .execute(&pool)
            .await
            .expect("clear stack");
    }
    // No elementary component at all (Fantastic's count is 1).
    assert_refused(&f, ALLOY, component, &[], "count_not_met", COUNT_LINE).await;
    f.cleanup().await;
}

#[tokio::test]
async fn live_db_two_counts_met_at_once_are_refused() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 12).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    // Counted by stack quantity: one stack of 10 Normal, one of 5 Good.
    let lower = [
        f.stack(NORMAL, INV_MAIN, 0, 10).await,
        f.stack(GOOD, INV_MAIN, 1, 5).await,
    ];
    let e = assert_refused(
        &f,
        ALLOY,
        component,
        &lower,
        "multiple_buckets",
        "Multiple categories of elementary components were met; use one quality only. Nothing was used.",
    )
    .await;
    assert!(
        e.has_field("elementary_counts", "normal:10,good:5,great:0,fantastic:0"),
        "{e:#?}"
    );
    f.cleanup().await;
}

/// The blueprint rules: a craft blueprint, an alloy blueprint not learned,
/// and a learned alloy whose discipline is not known.
#[tokio::test]
async fn live_db_blueprint_rules_are_refused() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 13).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    let lower = f.singles(NORMAL, 0, 10).await;

    let e = assert_refused(
        &f,
        CRAFT,
        component,
        &lower,
        "not_alloy",
        "That blueprint is not an alloy.",
    )
    .await;
    assert!(e.has_field("blueprint_id", &CRAFT.to_string()), "{e:#?}");

    let e = assert_refused(
        &f,
        OTHER_ALLOY,
        component,
        &lower,
        "unknown_blueprint",
        "You do not know that blueprint.",
    )
    .await;
    assert!(
        e.has_field("blueprint_id", &OTHER_ALLOY.to_string()),
        "{e:#?}"
    );

    sqlx::query("UPDATE sgw_player SET discipline_ids = '{}' WHERE player_id = $1")
        .bind(f.player_id)
        .execute(&pool)
        .await
        .expect("forget discipline");
    let e = assert_refused(
        &f,
        ALLOY,
        component,
        &lower,
        "discipline_unknown",
        "You must learn the blueprint's discipline first.",
    )
    .await;
    assert!(
        e.has_field("discipline_id", &DISCIPLINE.to_string()),
        "{e:#?}"
    );
    f.cleanup().await;
}

/// The current-tier slot must hold the blueprint's component, carried.
#[tokio::test]
async fn live_db_the_current_tier_item_must_be_the_carried_component() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 14).await;
    let lower = f.singles(NORMAL, 0, 10).await;

    let wrong = f.stack(TIER_TWO, INV_CRAFTING, 0, 1).await;
    let e = assert_refused(
        &f,
        ALLOY,
        wrong,
        &lower,
        "component_mismatch",
        "A chosen component is not the one this needs. Nothing was used.",
    )
    .await;
    assert!(
        e.has_field("design_item_type_id", &COMPONENT.to_string()),
        "{e:#?}"
    );

    let banked = f.stack(COMPONENT, INV_BANK, 0, 1).await;
    assert_refused(
        &f,
        ALLOY,
        banked,
        &lower,
        "component_not_in_crafting_bags",
        "Components must be in your backpack or crafting bag. Nothing was used.",
    )
    .await;

    assert_refused(
        &f,
        ALLOY,
        0,
        &lower,
        "component_missing",
        "A component is no longer in your inventory. Nothing was used.",
    )
    .await;

    // Another character's component, carried in its own crafting bag.
    let other = Fixture::new(&pool, 15).await;
    let theirs = other.stack(COMPONENT, INV_CRAFTING, 0, 1).await;
    let e = assert_refused(
        &f,
        ALLOY,
        theirs,
        &lower,
        "component_missing",
        "A component is no longer in your inventory. Nothing was used.",
    )
    .await;
    assert!(e.has_field("item_id", &theirs.to_string()), "{e:#?}");
    assert_eq!(other.inventory().await.len(), 1, "the owner keeps it");
    other.cleanup().await;
    f.cleanup().await;
}
