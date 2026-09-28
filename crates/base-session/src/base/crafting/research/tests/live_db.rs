//! Research against the seeded database, with pinned rolls: what is
//! consumed, what is gained, what the client is sent, and what is logged.
//!
//! The item is the seed's 5481 "Crafted Pistol of the Whale": researchable,
//! applied science 1, tech competency 20, disciplines 21 and 22, made by
//! blueprint 1 (discipline 21). Kicker 5669 is applied science 4.

use crate::base::crafting::research::research_job;
use crate::base::crafting::session::{crafting_sessions, drop_player_inductions, DropReason};
use crate::base::crafting::telemetry::METRIC_REQUESTS;
use crate::base::crafting::test_verbs::{engine, run_all, submit, VerbFixture};
use crate::cell::messages::CraftVerb;
use crate::mercury::method_idx;
use crate::test_support::{require_db_or_skip, Captured, LogCapture, LogCaptureGuard};
use cimmeria_observability::testing::{counter_total, install as install_meter};
use cimmeria_wire::cell::client_methods::being::ON_TIMER_UPDATE;

const ITEM: i32 = 5481;
const KICKER: i32 = 5669;
const INV_MAIN: i32 = 1;
const INV_CRAFTING: i32 = 15;

/// The item in the crafting bag and one kicker in the main bag.
async fn item_and_kicker(f: &VerbFixture) -> (i32, i32) {
    (
        f.stack(ITEM, INV_CRAFTING, 0).await,
        f.stack(KICKER, INV_MAIN, 0).await,
    )
}

/// Validate the request and run its job to completion on an engine whose
/// rolls are `samples`.
async fn research(f: &VerbFixture, item: i32, kickers: &[i32], samples: Vec<f64>) {
    let job = research_job(f.entity_id, f.player_id, item, kickers, &f.ctx())
        .await
        .expect("request accepted");
    let (sessions, scheduler) = engine(samples);
    submit(&sessions, f, Box::new(job)).await;
    assert_eq!(run_all(&sessions, &scheduler, &f.env).await, 1);
}

fn event(capture: &LogCaptureGuard, f: &VerbFixture, name: &str) -> Captured {
    let found = capture
        .all()
        .into_iter()
        .find(|c| {
            c.target == "crafting"
                && c.has_field("event", name)
                && c.has_field("player_id", &f.player_id.to_string())
        })
        .unwrap_or_else(|| panic!("no {name} event"));
    for (k, v) in [
        ("account_id", f.account_id.to_string()),
        ("entity_id", f.entity_id.to_string()),
        ("verb", "research".to_string()),
    ] {
        assert!(found.has_field(k, &v), "{name}: {k}={v}: {found:#?}");
    }
    found
}

fn args_of(f: &VerbFixture, method: u16) -> Vec<Vec<u8>> {
    f.calls()
        .into_iter()
        .filter(|c| c.method == method)
        .map(|c| c.args)
        .collect()
}

/// Expertise 10 in 21 (the only eligible discipline), one kicker: chance
/// 100 − 10 + 5 = 95, roll 50. The item and the kicker are consumed, 21
/// rises by 5, blueprint 1 is taught, and the client gets 136, the whole
/// known list on 139, and the result line.
#[tokio::test]
async fn a_successful_research_gains_expertise_and_teaches_the_blueprint() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 0).await;
    f.know(21, 10).await;
    let (item, kicker) = item_and_kicker(&f).await;

    research(&f, item, &[kicker], vec![0.0, 0.5]).await;

    assert!(!f.holds(item).await, "the researched item is used");
    assert!(!f.holds(kicker).await, "the kicker is used");
    assert_eq!(f.expertise(21).await, Some(15));
    assert_eq!(f.blueprints().await, vec![1]);
    assert_eq!(
        args_of(&f, method_idx::ON_UPDATE_DISCIPLINE),
        vec![vec![21, 0, 0, 0, 15, 0, 0, 0]]
    );
    // 139 byte-exact: ARRAY<INT32> count 1, blueprint 1.
    assert_eq!(
        args_of(&f, method_idx::ON_UPDATE_KNOWN_CRAFTS),
        vec![vec![1, 0, 0, 0, 1, 0, 0, 0]]
    );
    let lines = f.lines();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("Research succeeded:")
            && lines[0].contains("expertise increased to 15")
            && lines[0].ends_with("You learned 1 new blueprint."),
        "{lines:?}"
    );

    let completed = event(&capture, &f, "completed");
    for (k, v) in [
        ("result", "success".to_string()),
        ("chance", "95".to_string()),
        ("roll", "50".to_string()),
        ("discipline_id", "21".to_string()),
        ("eligible_disciplines", "21".to_string()),
        ("expertise", "21:10→15".to_string()),
        ("item_id", item.to_string()),
        ("blueprints_learned", "1:false→true".to_string()),
        (
            "consumed",
            format!("{item}:{ITEM}:1→0,{kicker}:{KICKER}:1→0"),
        ),
    ] {
        assert!(completed.has_field(k, &v), "{k}={v}: {completed:#?}");
    }
    let learned = event(&capture, &f, "blueprint_learned");
    for (k, v) in [
        ("blueprints", "1:false→true"),
        ("known_before", "0"),
        ("known_after", "1"),
    ] {
        assert!(learned.has_field(k, v), "{k}={v}: {learned:#?}");
    }
    f.cleanup().await;
}

/// The same research with roll 99 (not below 95): the item and the kicker
/// are still used, nothing is gained, and the line says so.
#[tokio::test]
async fn a_failed_research_uses_the_items_and_gains_nothing() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 1).await;
    f.know(21, 10).await;
    let (item, kicker) = item_and_kicker(&f).await;

    research(&f, item, &[kicker], vec![0.0, 0.99]).await;

    assert!(!f.holds(item).await && !f.holds(kicker).await);
    assert_eq!(f.expertise(21).await, Some(10));
    assert!(f.blueprints().await.is_empty());
    assert!(args_of(&f, method_idx::ON_UPDATE_DISCIPLINE).is_empty());
    assert!(args_of(&f, method_idx::ON_UPDATE_KNOWN_CRAFTS).is_empty());
    assert_eq!(
        f.lines(),
        vec!["Research complete, but no expertise was gained.".to_string()]
    );
    let completed = event(&capture, &f, "completed");
    for (k, v) in [("result", "failure"), ("chance", "95"), ("roll", "99")] {
        assert!(completed.has_field(k, v), "{k}={v}: {completed:#?}");
    }
    f.cleanup().await;
}

/// A success in 22, when blueprint 1's discipline (21) is not known:
/// 22 gains expertise, but no blueprint is taught and 139 is not sent.
#[tokio::test]
async fn a_success_teaches_no_blueprint_whose_discipline_is_unknown() {
    let pool = require_db_or_skip!();
    let f = VerbFixture::new(&pool, 2).await;
    f.know(22, 10).await;
    let item = f.stack(ITEM, INV_CRAFTING, 0).await;

    research(&f, item, &[], vec![0.0, 0.0]).await;

    assert!(!f.holds(item).await);
    assert_eq!(f.expertise(22).await, Some(15));
    assert!(f.blueprints().await.is_empty(), "21 is not known");
    assert!(args_of(&f, method_idx::ON_UPDATE_KNOWN_CRAFTS).is_empty());
    f.cleanup().await;
}

/// A blueprint the player already knows is not taught again, and 139 is
/// not re-sent.
#[tokio::test]
async fn a_known_blueprint_is_not_taught_again() {
    let pool = require_db_or_skip!();
    let f = VerbFixture::new(&pool, 4).await;
    f.know(21, 10).await;
    f.set_blueprints(&[1]).await;
    let item = f.stack(ITEM, INV_CRAFTING, 0).await;

    research(&f, item, &[], vec![0.0, 0.0]).await;

    assert_eq!(f.expertise(21).await, Some(15));
    assert_eq!(f.blueprints().await, vec![1]);
    assert!(args_of(&f, method_idx::ON_UPDATE_KNOWN_CRAFTS).is_empty());
    f.cleanup().await;
}

/// The request entry point routes `research` to its handler, which queues
/// an induction (the type-16 bar goes out), counts the request as
/// accepted, and consumes nothing until the induction ends. A logout
/// drops it with the item intact.
#[tokio::test]
async fn a_research_request_queues_an_induction_and_consumes_nothing_yet() {
    let pool = require_db_or_skip!();
    install_meter();
    let f = VerbFixture::new(&pool, 5).await;
    f.know(21, 10).await;
    let (item, kicker) = item_and_kicker(&f).await;
    let accepted = [("verb", "research"), ("outcome", "accepted")];
    let before = counter_total(METRIC_REQUESTS, &accepted);

    f.request(CraftVerb::Research {
        item_id: item,
        kickers: vec![kicker],
    })
    .await;

    assert_eq!(crafting_sessions().pending(f.entity_id), 1);
    assert!(counter_total(METRIC_REQUESTS, &accepted) > before);
    assert_eq!(args_of(&f, ON_TIMER_UPDATE).len(), 1, "the bar");
    assert!(f.lines().is_empty(), "no refusal");
    drop_player_inductions(f.entity_id, DropReason::Logout, "test");
    assert!(f.holds(item).await && f.holds(kicker).await);
    f.cleanup().await;
}

/// A plan built from a read taken before the transaction may name a
/// blueprint whose discipline the player dropped in between (a respec).
/// The transaction checks the discipline under its row lock: nothing is
/// taught, while the rest of the plan still applies.
#[tokio::test]
async fn a_blueprint_whose_discipline_was_dropped_before_the_transaction_is_not_taught() {
    use crate::base::crafting::telemetry::JobIds;
    use crate::base::crafting::transaction::{
        apply_craft_transaction, CraftTransaction, NamedItem,
    };

    let pool = require_db_or_skip!();
    let f = VerbFixture::new(&pool, 6).await;
    // The player knows 22 only; the stale plan still teaches blueprint 1
    // of discipline 21.
    f.know(22, 10).await;
    let item = f.stack(ITEM, INV_CRAFTING, 0).await;
    let plan = CraftTransaction {
        named_items: vec![NamedItem::new(item, ITEM)],
        consume_named: vec![(item, 1)],
        learn_blueprints: vec![(1, 21)],
        ..CraftTransaction::default()
    };
    let ids = JobIds {
        job_id: 0,
        verb: "research",
        account_id: f.account_id as u32,
        player_id: f.player_id,
        entity_id: f.entity_id,
        gm_entity_id: None,
    };

    let applied = apply_craft_transaction(&f.env, &ids, &plan)
        .await
        .expect("the transaction commits");

    assert!(applied.blueprints.is_none());
    assert!(f.blueprints().await.is_empty());
    assert!(!f.holds(item).await, "the rest of the plan applied");
    assert!(args_of(&f, method_idx::ON_UPDATE_KNOWN_CRAFTS).is_empty());
    f.cleanup().await;
}
