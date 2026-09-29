//! Reverse engineering against the seeded database, with pinned rolls.
//!
//! The item is the seed's 5481 "Crafted Pistol of the Whale": flagged
//! reverse-engineerable, tech competency 20, made only by blueprint 1
//! (discipline 21), whose sets are 1: 2× 5188, 1× 5189, 1× 5335;
//! 2: 2× 5224, 1× 5225; 3: 4× 5256. Every component goes to the crafting
//! bag (`{17,15}`).

use crate::base::crafting::reverse_engineer::reverse_engineer_job;
use crate::base::crafting::session::{crafting_sessions, drop_player_inductions, DropReason};
use crate::base::crafting::test_verbs::{engine, run_all, submit, VerbFixture};
use crate::cell::messages::CraftVerb;
use crate::test_support::{require_db_or_skip, LogCapture};

const ITEM: i32 = 5481;
const INV_MAIN: i32 = 1;
const INV_CRAFTING: i32 = 15;

/// Set 3's only component, four per recipe.
const SET3_COMPONENT: i32 = 5256;

async fn reverse_engineer(f: &VerbFixture, item: i32, samples: Vec<f64>) {
    let job = reverse_engineer_job(f.entity_id, f.player_id, item, &f.ctx())
        .await
        .expect("request accepted");
    let (sessions, scheduler) = engine(samples);
    submit(&sessions, f, Box::new(job)).await;
    assert_eq!(run_all(&sessions, &scheduler, &f.env).await, 1);
}

/// With no known discipline (bias 1/20), rolls that would recover 3 of set
/// 3's four units at full expertise recover the floor of one. Exactly the
/// named instance is used: another 5481, earlier in consumption-by-design
/// order, is untouched.
#[tokio::test]
async fn low_expertise_recovers_the_minimum_from_exactly_the_named_item() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 20).await;
    // By design, consumption would take the crafting bag's stack first.
    let other = f.stack(ITEM, INV_CRAFTING, 0).await;
    let named = f.stack(ITEM, INV_MAIN, 0).await;

    // Blueprint 1 (only candidate), set 3 (0.9 of 3), component roll 0.99.
    reverse_engineer(&f, named, vec![0.0, 0.9, 0.99]).await;

    assert!(!f.holds(named).await, "the named instance is used");
    assert!(f.holds(other).await, "the other 5481 is not");
    assert_eq!(f.units_of(SET3_COMPONENT).await, (1, vec![INV_CRAFTING]));
    let lines = f.lines();
    assert_eq!(
        lines,
        vec!["Reverse engineering complete: recovered 1 component.".to_string()]
    );

    let completed = capture
        .all()
        .into_iter()
        .find(|c| {
            c.target == "crafting"
                && c.has_field("event", "completed")
                && c.has_field("player_id", &f.player_id.to_string())
        })
        .expect("completed event");
    for (k, v) in [
        ("verb", "reverseEngineer".to_string()),
        ("account_id", f.account_id.to_string()),
        ("entity_id", f.entity_id.to_string()),
        ("item_id", named.to_string()),
        ("blueprint_id", "1".to_string()),
        ("component_set_id", "3".to_string()),
        ("bias", "0.05".to_string()),
        ("rolls", format!("{SET3_COMPONENT}:0.9900:1/4")),
        ("consumed", format!("{named}:{ITEM}:1→0")),
    ] {
        assert!(completed.has_field(k, &v), "{k}={v}: {completed:#?}");
    }
    f.cleanup().await;
}

/// At expertise 20 in blueprint 1's discipline (the product's tech
/// competency) the same rolls recover floor(0.99 × 4) = 3.
#[tokio::test]
async fn high_expertise_recovers_more_from_the_same_rolls() {
    let pool = require_db_or_skip!();
    let f = VerbFixture::new(&pool, 21).await;
    f.know(21, 20).await;
    let item = f.stack(ITEM, INV_CRAFTING, 0).await;

    reverse_engineer(&f, item, vec![0.0, 0.9, 0.99]).await;

    assert!(!f.holds(item).await);
    assert_eq!(f.units_of(SET3_COMPONENT).await, (3, vec![INV_CRAFTING]));
    assert_eq!(
        f.lines(),
        vec!["Reverse engineering complete: recovered 3 components.".to_string()]
    );
    f.cleanup().await;
}

/// The reverse-engineering page sends ten requests in one burst: all ten
/// are accepted, run one after another, and each uses its own item.
#[tokio::test]
async fn a_burst_of_ten_completes_ten_times() {
    let pool = require_db_or_skip!();
    let f = VerbFixture::new(&pool, 22).await;
    f.know(21, 20).await;
    let mut items = Vec::new();
    for slot in 0..10 {
        items.push(f.stack(ITEM, INV_CRAFTING, slot).await);
    }
    let (sessions, scheduler) = engine(vec![0.0, 0.9, 0.99]);
    for &item in &items {
        let job = reverse_engineer_job(f.entity_id, f.player_id, item, &f.ctx())
            .await
            .expect("request accepted");
        submit(&sessions, &f, Box::new(job)).await;
    }
    assert_eq!(sessions.pending(f.entity_id), 10);

    assert_eq!(run_all(&sessions, &scheduler, &f.env).await, 10);

    for item in items {
        assert!(!f.holds(item).await, "{item} used");
    }
    assert_eq!(f.units_of(SET3_COMPONENT).await.0, 30);
    assert_eq!(f.lines().len(), 10, "one result line per item");
    f.cleanup().await;
}

/// Through the request entry point: queued, nothing used yet, and a logout
/// drops it with the item intact.
#[tokio::test]
async fn a_reverse_engineer_request_queues_and_consumes_nothing_yet() {
    let pool = require_db_or_skip!();
    let f = VerbFixture::new(&pool, 23).await;
    let item = f.stack(ITEM, INV_CRAFTING, 0).await;

    f.request(CraftVerb::ReverseEngineer { item_id: item })
        .await;

    assert_eq!(crafting_sessions().pending(f.entity_id), 1);
    assert!(f.lines().is_empty(), "no refusal");
    drop_player_inductions(f.entity_id, DropReason::Logout, "test");
    assert!(f.holds(item).await);
    f.cleanup().await;
}
