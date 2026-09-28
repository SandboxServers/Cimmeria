//! Named instances are tied to their design: an instance of another
//! design is refused, and an exact consumption takes only the instance
//! named.

use super::*;
use crate::base::crafting::test_packets::feedback_text;
use crate::mercury::method_idx;
use crate::test_support::{require_db_or_skip, LogCapture};

/// A request that names a carried item of another design cannot have the
/// component design consumed in its place: nothing is used, the player
/// reads why, and the refusal names both designs.
#[tokio::test]
async fn live_db_a_named_instance_of_another_design_is_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 16).await;
    let unrelated = f.stack(FILLER, INV_CRAFTING, 0, 1).await;
    let component = f.stack(COMPONENT, INV_CRAFTING, 1, 2).await;

    let result = f
        .apply(&CraftTransaction {
            named_items: vec![NamedItem::new(unrelated, COMPONENT)],
            consume: vec![(COMPONENT, 1)],
            ..CraftTransaction::default()
        })
        .await;

    assert_eq!(
        result,
        Err(CraftReject::ComponentMismatch {
            item_id: unrelated,
            expected_design_id: COMPONENT,
            type_id: FILLER,
        })
    );
    assert_eq!(
        f.row(component).await,
        Some((2, INV_CRAFTING)),
        "nothing used"
    );
    assert_eq!(f.row(unrelated).await, Some((1, INV_CRAFTING)));
    let calls = f.calls();
    assert_eq!(calls[0].method, method_idx::ON_PLAYER_COMMUNICATION);
    assert_eq!(
        feedback_text(&calls[0]),
        "A chosen component is not the one this needs. Nothing was used."
    );
    let e = capture
        .find_event(
            tracing::Level::INFO,
            "crafting request rejected",
            "component_mismatch",
        )
        .expect("rejected component_mismatch");
    assert!(e.has_field("item_id", &unrelated.to_string()), "{e:#?}");
    assert!(e.has_field("design_id", &COMPONENT.to_string()), "{e:#?}");
    assert!(e.has_field("type_id", &FILLER.to_string()), "{e:#?}");
    f.cleanup().await;
}

/// An exact consumption takes from the named instance only, even when
/// consumption by design would start with another stack (the crafting
/// bag comes first).
#[tokio::test]
async fn live_db_an_exact_consumption_takes_only_the_named_instance() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 17).await;
    let staged = f.stack(COMPONENT, INV_CRAFTING, 0, 2).await;
    let named = f.stack(COMPONENT, INV_MAIN, 0, 2).await;

    let applied = f
        .apply(&CraftTransaction {
            named_items: vec![NamedItem::new(named, COMPONENT)],
            consume_named: vec![(named, 1)],
            ..CraftTransaction::default()
        })
        .await
        .expect("exact consumption commits");

    assert_eq!(f.row(named).await, Some((1, INV_MAIN)), "the named stack");
    assert_eq!(f.row(staged).await, Some((2, INV_CRAFTING)), "untouched");
    assert_eq!(applied.consumed_field(), format!("{named}:{COMPONENT}:2→1"));
    f.cleanup().await;
}

/// Asking for more than the named instance holds refuses the whole
/// transaction, even when other stacks of the design would cover it.
#[tokio::test]
async fn live_db_an_exact_consumption_larger_than_the_instance_is_refused() {
    let pool = require_db_or_skip!();
    assert_seed_shape(&pool).await;
    let f = Fixture::new(&pool, 18).await;
    let other = f.stack(COMPONENT, INV_CRAFTING, 0, 2).await;
    let named = f.stack(COMPONENT, INV_MAIN, 0, 1).await;

    let result = f
        .apply(&CraftTransaction {
            named_items: vec![NamedItem::new(named, COMPONENT)],
            consume_named: vec![(named, 2)],
            ..CraftTransaction::default()
        })
        .await;

    assert_eq!(
        result,
        Err(CraftReject::NotEnoughComponents {
            design_id: COMPONENT,
            needed: 2,
            available: 1,
        })
    );
    assert_eq!(f.row(named).await, Some((1, INV_MAIN)));
    assert_eq!(f.row(other).await, Some((2, INV_CRAFTING)));
    f.cleanup().await;
}
