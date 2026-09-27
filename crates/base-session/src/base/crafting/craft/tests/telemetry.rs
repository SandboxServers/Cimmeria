//! The craft's events and counters: `completed` names the blueprint, the
//! set and the quantity; an accepted craft is counted once; each seam that
//! can fail is a WARN with the player's identity.

use super::*;
use crate::base::crafting::telemetry::{METRIC_JOBS, METRIC_REJECTIONS, METRIC_REQUESTS};
use crate::test_support::{require_db_or_skip, LogCapture};
use cimmeria_observability::testing::{counter_total, install as install_meter};

/// `completed` carries the verb's own fields next to the transaction's
/// lists, with the full identity; the request is counted accepted once and
/// the job completed once.
#[tokio::test]
async fn completed_names_the_blueprint_set_and_quantity() {
    let pool = require_db_or_skip!();
    install_meter();
    let capture = LogCapture::install();
    let accepted = [("verb", VERB), ("outcome", "accepted")];
    let completed = [("verb", VERB), ("outcome", "completed")];
    let before = (
        counter_total(METRIC_REQUESTS, &accepted),
        counter_total(METRIC_JOBS, &completed),
    );
    let f = Fixture::new(&pool, 30).await;
    f.know(&[(DISCIPLINE, 10)], &[BLUEPRINT]).await;
    let steel = f.stacks(STEEL_CORE, INV_CRAFTING, 0, 14).await;

    f.craft(BLUEPRINT, &[steel[13]], 1).await;
    f.run_inductions().await;
    let plating = f.stacks_of(TITANIUM_PLATING).await;
    f.cleanup().await;

    assert_eq!(counter_total(METRIC_REQUESTS, &accepted) - before.0, 1);
    assert_eq!(counter_total(METRIC_JOBS, &completed) - before.1, 1);
    let event = capture
        .all()
        .into_iter()
        .find(|c| c.target == "crafting" && c.has_field("event", "completed"))
        .expect("completed event");
    let (_, _, bag, slot) = plating[0];
    for (k, v) in [
        ("verb", VERB.to_string()),
        ("account_id", f.account_id.to_string()),
        ("player_id", f.player_id.to_string()),
        ("entity_id", f.entity_id.to_string()),
        ("blueprint_id", BLUEPRINT.to_string()),
        ("component_set_id", "1".to_string()),
        ("quantity", "1".to_string()),
        ("granted", format!("{TITANIUM_PLATING}:{bag}:{slot}:0→1")),
        ("expertise", format!("{DISCIPLINE}:10→11")),
    ] {
        assert!(event.has_field(k, &v), "{k}={v}: {event:#?}");
    }
    let consumed = event.fields.get("consumed").expect("consumed field");
    assert_eq!(consumed.split(',').count(), 14, "{consumed}");
    assert!(consumed
        .split(',')
        .all(|c| c.contains(&format!(":{STEEL_CORE}:1→0"))));
}

/// A refusal counts one rejection under its reason and one rejected
/// request, and never an accepted one.
#[tokio::test]
async fn a_refusal_is_counted_under_its_reason() {
    install_meter();
    let reason = [("verb", VERB), ("reason", "bad_quantity")];
    let rejected = [("verb", VERB), ("outcome", "rejected")];
    let before = (
        counter_total(METRIC_REJECTIONS, &reason),
        counter_total(METRIC_REQUESTS, &rejected),
    );
    let f = offline(4291);

    handle_craft_with(&f.sessions, f.entity_id, 4292, BLUEPRINT, &[], 0, &f.ctx()).await;

    assert_eq!(counter_total(METRIC_REJECTIONS, &reason) - before.0, 1);
    assert_eq!(counter_total(METRIC_REQUESTS, &rejected) - before.1, 1);
}

/// With no database the craft cannot be decided: a `lookup_failed` WARN
/// with the identity and the phase, and the "unavailable" line.
#[tokio::test]
async fn no_database_is_a_lookup_warning_and_a_line() {
    let capture = LogCapture::install();
    let f = offline(4293);

    handle_craft_with(&f.sessions, f.entity_id, 4294, BLUEPRINT, &[7], 1, &f.ctx()).await;

    assert_eq!(
        f.lines(),
        vec!["Crafting is unavailable right now. Nothing was changed.".to_string()]
    );
    let warn = capture
        .find_message(
            tracing::Level::WARN,
            "craft: a lookup the decision needs failed",
        )
        .expect("lookup_failed WARN");
    for (k, v) in [
        ("event", "lookup_failed"),
        ("phase", "no_pool"),
        ("verb", VERB),
        ("account_id", "4242"),
        ("player_id", "4294"),
        ("entity_id", "4293"),
        ("blueprint_id", "412"),
    ] {
        assert!(warn.has_field(k, v), "{k}={v}: {warn:#?}");
    }
}

/// A product with no `resources.items` row still crafts, named by id, and
/// the miss is a WARN.
#[tokio::test]
async fn a_missing_product_name_is_a_warning() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let who = super::super::Who {
        account_id: Some(1),
        player_id: 2,
        entity_id: 3,
        blueprint_id: 4,
    };

    let name = super::super::product_name(&pool, 0x7000_C1FF, &who).await;

    assert_eq!(name, format!("item {}", 0x7000_C1FF));
    let warn = capture
        .find_message(
            tracing::Level::WARN,
            "craft: a lookup the decision needs failed",
        )
        .expect("lookup_failed WARN");
    assert!(warn.has_field("phase", "product_name"), "{warn:#?}");
    assert!(warn.has_field("error_class", "miss"), "{warn:#?}");
    assert!(warn.has_field("account_id", "1"), "{warn:#?}");
}

/// The queued line that cannot be sent is `client_sync_failed`, not a
/// silent drop.
#[tokio::test]
async fn an_unsent_queued_line_is_a_warning() {
    let capture = LogCapture::install();
    let f = offline(4295);
    let empty = Arc::new(Mutex::new(HashMap::new()));
    let client = crate::base::crafting::sync::CraftClient {
        transport: &f.transport,
        connected: &f.connected,
        entity_to_addr: &empty,
    };

    super::super::send_note(client, f.entity_id, 4296, "queued").await;

    let warn = capture
        .find_event(
            tracing::Level::WARN,
            "craft queued line not sent",
            "entity_to_addr_miss",
        )
        .expect("client_sync_failed WARN");
    assert!(warn.has_field("event", "client_sync_failed"), "{warn:#?}");
    assert!(warn.has_field("what", "craft_queued"), "{warn:#?}");
    assert!(warn.has_field("player_id", "4296"), "{warn:#?}");
    assert!(warn.has_field("entity_id", "4295"), "{warn:#?}");
}
