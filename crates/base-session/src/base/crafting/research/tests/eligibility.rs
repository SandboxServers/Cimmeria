//! A research the player could learn nothing from is refused before
//! anything is used: at the request, at completion when a discipline was
//! dropped during the bar, and inside the completion transaction under
//! the player row lock.
//!
//! The item is the seed's 5481: applied science 1, tech competency 20,
//! disciplines 21 and 22, made by blueprint 1 (discipline 21). Kicker 5669
//! is applied science 4.

use cimmeria_cell_catalog::crafting::shared_crafting_catalog;

use crate::base::crafting::feedback::CraftReject;
use crate::base::crafting::item_lookup::held_instances;
use crate::base::crafting::research::rule::ResearchRoll;
use crate::base::crafting::research::{research_job, ResearchJob};
use crate::base::crafting::session::crafting_sessions;
use crate::base::crafting::telemetry::{JobIds, METRIC_REJECTIONS};
use crate::base::crafting::test_verbs::{engine, run_all, submit, VerbFixture};
use crate::base::crafting::transaction::apply_craft_transaction;
use crate::cell::messages::CraftVerb;
use crate::test_support::{require_db_or_skip, Captured, LogCapture, LogCaptureGuard};
use cimmeria_observability::testing::{counter_total, install as install_meter};

const ITEM: i32 = 5481;
const KICKER: i32 = 5669;
const INV_MAIN: i32 = 1;
const INV_CRAFTING: i32 = 15;

fn refusal(item_id: i32, known: &[(i32, i32)]) -> CraftReject {
    CraftReject::NoEligibleDiscipline {
        item_id,
        type_id: ITEM,
        applied_science_id: Some(1),
        tech_comp: 20,
        item_disciplines: vec![21, 22],
        known: known.to_vec(),
    }
}

fn labels() -> [(&'static str, &'static str); 2] {
    [("verb", "research"), ("reason", "no_eligible_discipline")]
}

/// The player's `rejected` event, with the identity and every value the
/// rule compared.
fn assert_rejected(capture: &LogCaptureGuard, f: &VerbFixture, item: i32, known: &str) -> Captured {
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
        ("reason", "no_eligible_discipline".to_string()),
        ("verb", "research".to_string()),
        ("account_id", f.account_id.to_string()),
        ("entity_id", f.entity_id.to_string()),
        ("item_id", item.to_string()),
        ("type_id", ITEM.to_string()),
        ("applied_science_id", "1".to_string()),
        ("tech_comp", "20".to_string()),
        ("item_disciplines", "21,22".to_string()),
        ("known_disciplines", known.to_string()),
    ] {
        assert!(rejected.has_field(k, &v), "{k}={v}: {rejected:#?}");
    }
    rejected
}

fn completed(capture: &LogCaptureGuard, f: &VerbFixture) -> bool {
    capture.all().into_iter().any(|c| {
        c.target == "crafting"
            && c.has_field("event", "completed")
            && c.has_field("player_id", &f.player_id.to_string())
    })
}

/// 21 known at expertise 0 and 22 at the tech competency: neither can be
/// researched. The request is refused with the line, nothing is queued,
/// and the item, the kicker, the expertise and the blueprints are as they
/// were.
#[tokio::test]
async fn live_db_a_research_with_no_eligible_discipline_is_refused_at_the_request() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 16).await;
    f.know(21, 0).await;
    f.know(22, 20).await;
    let item = f.stack(ITEM, INV_CRAFTING, 0).await;
    let kicker = f.stack(KICKER, INV_MAIN, 0).await;
    let inventory = f.inventory().await;
    let before = counter_total(METRIC_REJECTIONS, &labels());

    f.request(CraftVerb::Research {
        item_id: item,
        kickers: vec![kicker],
    })
    .await;

    let why = refusal(item, &[(21, 0), (22, 20)]);
    assert_eq!(f.lines(), vec![why.text()]);
    assert!(why.text().ends_with("Nothing was used."));
    assert_eq!(
        crafting_sessions().pending(f.entity_id),
        0,
        "nothing queued"
    );
    assert_eq!(f.inventory().await, inventory, "nothing is consumed");
    assert!(f.holds(item).await && f.holds(kicker).await);
    assert_eq!(
        (f.expertise(21).await, f.expertise(22).await),
        (Some(0), Some(20))
    );
    assert!(f.blueprints().await.is_empty());
    assert!(counter_total(METRIC_REJECTIONS, &labels()) > before);
    assert_rejected(&capture, &f, item, "21:0,22:20");
    assert!(!completed(&capture, &f));
    f.cleanup().await;
}

/// Eligible at the request (21 at 10), then 21 is forgotten before the bar
/// ends. The completion refuses with the same line: the item and the
/// kicker stay, no expertise or blueprint appears, and no `completed` is
/// logged.
#[tokio::test]
async fn live_db_a_discipline_dropped_during_the_bar_refuses_the_research_at_completion() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 17).await;
    f.know(21, 10).await;
    let item = f.stack(ITEM, INV_CRAFTING, 0).await;
    let kicker = f.stack(KICKER, INV_MAIN, 0).await;
    let job = research_job(f.entity_id, f.player_id, item, &[kicker], &f.ctx())
        .await
        .expect("eligible at the request");
    let (sessions, scheduler) = engine(vec![0.0, 0.0]);
    submit(&sessions, &f, Box::new(job)).await;
    sqlx::query("UPDATE sgw_player SET discipline_ids = '{}' WHERE player_id = $1")
        .bind(f.player_id)
        .execute(&pool)
        .await
        .expect("forget 21");
    sqlx::query("DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1")
        .bind(f.player_id)
        .execute(&pool)
        .await
        .expect("drop expertise");
    let before = counter_total(METRIC_REJECTIONS, &labels());

    assert_eq!(run_all(&sessions, &scheduler, &f.env).await, 1);

    assert_eq!(f.lines(), vec![refusal(item, &[]).text()]);
    assert!(f.holds(item).await, "the researched item is not used");
    assert!(f.holds(kicker).await, "the kicker is not used");
    assert_eq!(f.expertise(21).await, None);
    assert!(f.blueprints().await.is_empty());
    assert!(counter_total(METRIC_REJECTIONS, &labels()) > before);
    assert_rejected(&capture, &f, item, "");
    assert!(!completed(&capture, &f), "nothing completed");
    f.cleanup().await;
}

/// A plan built from a roll that picked 21, applied when the player no
/// longer knows it: the transaction reads the state under the player row
/// lock, refuses, and rolls back, so a change that lands between the
/// completion's read and the transaction still uses nothing.
#[tokio::test]
async fn live_db_the_transaction_refuses_a_research_the_player_can_no_longer_learn_from() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let f = VerbFixture::new(&pool, 18).await;
    // Known, but at the tech competency: not eligible.
    f.know(21, 20).await;
    let item = f.stack(ITEM, INV_CRAFTING, 0).await;
    let kicker = f.stack(KICKER, INV_MAIN, 0).await;
    let held = held_instances(&pool, f.player_id, &[item, kicker])
        .await
        .expect("read")
        .expect("held");
    let job = ResearchJob {
        catalog: shared_crafting_catalog(&pool).await.expect("catalog"),
        item: held[0],
        kickers: vec![held[1]],
    };
    let won = ResearchRoll {
        eligible: vec![21],
        discipline_id: Some(21),
        chance: Some(95.0),
        roll: Some(0.0),
        success: true,
    };
    let plan = job.plan(&won, vec![(1, 21)]);
    let ids = JobIds {
        job_id: 0,
        verb: "research",
        account_id: f.account_id as u32,
        player_id: f.player_id,
        entity_id: f.entity_id,
        gm_entity_id: None,
    };

    let result = apply_craft_transaction(&f.env, &ids, &plan).await;

    let why = refusal(item, &[(21, 20)]);
    assert_eq!(result.err(), Some(why.clone()));
    assert!(f.holds(item).await && f.holds(kicker).await, "rolled back");
    assert_eq!(f.expertise(21).await, Some(20));
    assert!(f.blueprints().await.is_empty());
    assert_eq!(f.lines(), vec![why.text()]);
    assert_rejected(&capture, &f, item, "21:20");
    f.cleanup().await;
}
