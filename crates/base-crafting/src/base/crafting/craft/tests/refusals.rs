//! Every craft refusal: the player reads a line, nothing is queued or
//! consumed, and `rejected` carries the reason and the full identity.

use super::*;
use crate::test_support::{require_db_or_skip, LogCapture, LogCaptureGuard};

/// The refusal left exactly one line, `text`, no induction bar and no
/// queued job, and logged `rejected` with `reason`, the verb, the player's
/// identity and `fields`.
fn assert_refused(
    f: &Fixture,
    capture: &LogCaptureGuard,
    reason: &str,
    text: &str,
    fields: &[(&str, String)],
) {
    let calls = f.calls();
    assert_eq!(
        calls.iter().map(|c| c.method).collect::<Vec<_>>(),
        vec![method_idx::ON_PLAYER_COMMUNICATION],
        "one line and nothing else ({reason})"
    );
    assert_eq!(feedback_text(&calls[0]), text);
    assert_eq!(f.sessions.pending(f.entity_id), 0, "nothing queued");
    let event = capture
        .all()
        .into_iter()
        .find(|c| {
            c.target == "crafting"
                && c.has_field("event", "rejected")
                && c.has_field("player_id", &f.player_id.to_string())
        })
        .unwrap_or_else(|| panic!("no rejected event ({reason})"));
    let identity = [
        ("reason", reason.to_string()),
        ("verb", VERB.to_string()),
        ("account_id", f.account_id.to_string()),
        ("entity_id", f.entity_id.to_string()),
    ];
    for (k, v) in identity.iter().chain(fields) {
        assert!(event.has_field(k, v), "{k}={v}: {event:#?}");
    }
}

#[tokio::test]
async fn live_db_a_quantity_outside_one_to_a_hundred_is_refused() {
    let pool = require_db_or_skip!();
    for (slot, quantity) in [(10, 0), (11, MAX_CRAFT_QUANTITY + 1), (12, -3)] {
        let capture = LogCapture::install();
        let f = Fixture::new(&pool, slot).await;
        f.know(&[(DISCIPLINE, 10)], &[BLUEPRINT]).await;
        let steel = f.stacks(STEEL_CORE, INV_MAIN, 0, 14).await;

        f.craft(BLUEPRINT, &[steel[13]], quantity).await;

        assert_refused(
            &f,
            &capture,
            "bad_quantity",
            "You can craft between 1 and 100 at a time.",
            &[
                ("quantity", quantity.to_string()),
                ("blueprint_id", BLUEPRINT.to_string()),
            ],
        );
        assert_eq!(f.units(STEEL_CORE).await, 14);
        f.cleanup().await;
    }
}

#[tokio::test]
async fn live_db_an_unknown_blueprint_is_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = Fixture::new(&pool, 13).await;
    f.know(&[(DISCIPLINE, 10)], &[]).await;
    let steel = f.stacks(STEEL_CORE, INV_MAIN, 0, 14).await;

    f.craft(BLUEPRINT, &[steel[13]], 1).await;

    assert_refused(
        &f,
        &capture,
        "unknown_blueprint",
        "You do not know that blueprint.",
        &[("blueprint_id", BLUEPRINT.to_string())],
    );
    assert_eq!(f.units(STEEL_CORE).await, 14);
    f.cleanup().await;
}

/// The guard for the discipline rule: the blueprint alone is not enough.
#[tokio::test]
async fn live_db_a_known_blueprint_of_an_unknown_discipline_is_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = Fixture::new(&pool, 14).await;
    f.know(&[], &[BLUEPRINT]).await;
    let steel = f.stacks(STEEL_CORE, INV_MAIN, 0, 14).await;

    f.craft(BLUEPRINT, &[steel[13]], 1).await;

    assert_refused(
        &f,
        &capture,
        "discipline_unknown",
        "You must learn the blueprint's discipline first.",
        &[
            ("blueprint_id", BLUEPRINT.to_string()),
            ("discipline_id", DISCIPLINE.to_string()),
        ],
    );
    assert_eq!(f.units(STEEL_CORE).await, 14);
    assert!(f.stacks_of(TITANIUM_PLATING).await.is_empty());
    f.cleanup().await;
}

#[tokio::test]
async fn live_db_an_alloy_blueprint_is_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = Fixture::new(&pool, 15).await;
    f.know(&[(DISCIPLINE, 10)], &[ALLOY]).await;

    f.craft(ALLOY, &[], 1).await;

    assert_refused(
        &f,
        &capture,
        "is_alloy",
        "That blueprint is an alloy. Use alloying to make it.",
        &[("blueprint_id", ALLOY.to_string())],
    );
    f.cleanup().await;
}

/// Titanium Cores alone are part of set 2 but match no whole set.
#[tokio::test]
async fn live_db_components_that_match_no_set_are_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = Fixture::new(&pool, 16).await;
    f.know(&[(DISCIPLINE, 10)], &[BLUEPRINT]).await;
    let titanium = f.stacks(TITANIUM_CORE, INV_MAIN, 0, 5).await;

    f.craft(BLUEPRINT, &[titanium[4]], 1).await;

    assert_refused(
        &f,
        &capture,
        "no_component_set",
        "Those components do not match any recipe of this blueprint. Nothing was used.",
        &[
            ("blueprint_id", BLUEPRINT.to_string()),
            ("type_ids", format!("[{TITANIUM_CORE}]")),
        ],
    );
    assert_eq!(f.units(TITANIUM_CORE).await, 5);
    f.cleanup().await;
}

/// Blueprint 21 has no component set in the seed: no submission can
/// craft it.
#[tokio::test]
async fn live_db_a_blueprint_with_no_components_is_refused() {
    let pool = require_db_or_skip!();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM resources.blueprints_components WHERE blueprint_id = $1",
    )
    .bind(NO_COMPONENTS)
    .fetch_one(&pool)
    .await
    .expect("count 21");
    assert_eq!(count, 0, "blueprint 21 has no components in the seed");
    let capture = LogCapture::install();
    let f = Fixture::new(&pool, 17).await;
    f.know(&[(DISCIPLINE, 10)], &[NO_COMPONENTS]).await;
    let steel = f.stacks(STEEL_CORE, INV_MAIN, 0, 1).await;

    f.craft(NO_COMPONENTS, &steel, 1).await;

    assert_refused(
        &f,
        &capture,
        "no_component_set",
        "Those components do not match any recipe of this blueprint. Nothing was used.",
        &[("blueprint_id", NO_COMPONENTS.to_string())],
    );
    assert_eq!(f.units(STEEL_CORE).await, 1);
    assert!(f.stacks_of(19).await.is_empty(), "no Ambernol Vial");
    f.cleanup().await;
}

/// 13 Steel Cores for set 1's 14; the Steel Core in the bank does not
/// count.
#[tokio::test]
async fn live_db_too_few_components_in_the_two_bags_are_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = Fixture::new(&pool, 18).await;
    f.know(&[(DISCIPLINE, 10)], &[BLUEPRINT]).await;
    let steel = f.stacks(STEEL_CORE, INV_MAIN, 0, 13).await;
    f.stack(STEEL_CORE, INV_BANK, 0, 1).await;

    f.craft(BLUEPRINT, &[steel[12]], 1).await;

    assert_refused(
        &f,
        &capture,
        "insufficient_components",
        "You do not have enough components: 13 of 14 needed. Nothing was used.",
        &[
            ("blueprint_id", BLUEPRINT.to_string()),
            ("design_id", STEEL_CORE.to_string()),
            ("needed", "14".to_string()),
            ("available", "13".to_string()),
        ],
    );
    assert_eq!(f.units(STEEL_CORE).await, 14);
    f.cleanup().await;
}

/// An instance id that is not the player's (another player's, or none).
#[tokio::test]
async fn live_db_a_component_that_is_not_the_players_is_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = Fixture::new(&pool, 19).await;
    let other = Fixture::new(&pool, 20).await;
    f.know(&[(DISCIPLINE, 10)], &[BLUEPRINT]).await;
    f.stacks(STEEL_CORE, INV_MAIN, 0, 14).await;
    let theirs = other.stack(STEEL_CORE, INV_MAIN, 0, 1).await;

    f.craft(BLUEPRINT, &[theirs], 1).await;

    assert_refused(
        &f,
        &capture,
        "component_missing",
        "A component is no longer in your inventory. Nothing was used.",
        &[("item_id", theirs.to_string())],
    );
    assert_eq!(f.units(STEEL_CORE).await, 14);
    assert_eq!(other.units(STEEL_CORE).await, 1);
    f.cleanup().await;
    other.cleanup().await;
}

#[tokio::test]
async fn live_db_a_component_in_the_bank_is_refused() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = Fixture::new(&pool, 21).await;
    f.know(&[(DISCIPLINE, 10)], &[BLUEPRINT]).await;
    f.stacks(STEEL_CORE, INV_MAIN, 0, 14).await;
    let banked = f.stack(STEEL_CORE, INV_BANK, 0, 1).await;

    f.craft(BLUEPRINT, &[banked], 1).await;

    assert_refused(
        &f,
        &capture,
        "component_not_in_crafting_bags",
        "Components must be in your backpack or crafting bag. Nothing was used.",
        &[
            ("item_id", banked.to_string()),
            ("container_id", INV_BANK.to_string()),
        ],
    );
    assert_eq!(f.units(STEEL_CORE).await, 15);
    f.cleanup().await;
}

/// A respec during the bar: the discipline is gone when the craft
/// completes, so it is refused with the discipline line and nothing is
/// consumed or granted.
#[tokio::test]
async fn live_db_a_discipline_forgotten_during_the_bar_refuses_the_craft() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = Fixture::new(&pool, 23).await;
    f.know(&[(DISCIPLINE, 10)], &[BLUEPRINT]).await;
    let steel = f.stacks(STEEL_CORE, INV_MAIN, 0, 14).await;

    f.craft(BLUEPRINT, &[steel[13]], 1).await;
    sqlx::query("UPDATE sgw_player SET discipline_ids = '{}' WHERE player_id = $1")
        .bind(f.player_id)
        .execute(&pool)
        .await
        .expect("forget the discipline");
    sqlx::query("DELETE FROM sgw_player_discipline_expertise WHERE player_id = $1")
        .bind(f.player_id)
        .execute(&pool)
        .await
        .expect("clear expertise");
    f.typed.clear();
    assert_eq!(f.run_inductions().await, 1);

    let lines = f.lines();
    let steel_left = f.units(STEEL_CORE).await;
    let plating = f.stacks_of(TITANIUM_PLATING).await;
    f.cleanup().await;

    assert_eq!(
        lines,
        vec!["You must learn the blueprint's discipline first.".to_string()]
    );
    assert_eq!(steel_left, 14, "nothing consumed");
    assert!(plating.is_empty(), "nothing granted");
    let event = capture
        .all()
        .into_iter()
        .find(|c| c.has_field("event", "rejected") && c.has_field("reason", "discipline_unknown"))
        .expect("rejected at completion");
    for (k, v) in [
        ("verb", VERB.to_string()),
        ("account_id", f.account_id.to_string()),
        ("player_id", f.player_id.to_string()),
        ("entity_id", f.entity_id.to_string()),
    ] {
        assert!(event.has_field(k, &v), "{k}={v}: {event:#?}");
    }
}

/// Validated at the request, re-checked at the end: components sold
/// during the bar leave the craft refused at completion, with the line
/// and a resync, and no product.
#[tokio::test]
async fn live_db_components_gone_by_the_end_of_the_bar_refuse_the_craft() {
    let pool = require_db_or_skip!();
    let capture = LogCapture::install();
    let f = Fixture::new(&pool, 22).await;
    f.know(&[(DISCIPLINE, 10)], &[BLUEPRINT]).await;
    let steel = f.stacks(STEEL_CORE, INV_MAIN, 0, 14).await;

    f.craft(BLUEPRINT, &[steel[13]], 1).await;
    sqlx::query("DELETE FROM sgw_inventory WHERE item_id = $1")
        .bind(steel[0])
        .execute(&pool)
        .await
        .expect("sell one core");
    f.typed.clear();
    assert_eq!(f.run_inductions().await, 1);

    let calls = f.calls();
    let steel_left = f.units(STEEL_CORE).await;
    let plating = f.stacks_of(TITANIUM_PLATING).await;
    let expertise = f.expertise(DISCIPLINE).await;
    f.cleanup().await;

    assert_eq!(
        calls.iter().map(|c| c.method).collect::<Vec<_>>(),
        vec![
            method_idx::ON_PLAYER_COMMUNICATION,
            method_idx::ON_UPDATE_ITEM
        ],
        "the line, then the resync"
    );
    assert_eq!(
        feedback_text(&calls[0]),
        "You do not have enough components. Nothing was used."
    );
    assert_eq!(steel_left, 13);
    assert!(plating.is_empty());
    assert_eq!(expertise, Some(10));
    assert!(
        capture
            .all()
            .iter()
            .any(|c| c.has_field("event", "rejected")
                && c.has_field("reason", "not_enough_components")
                && c.has_field("verb", VERB)),
        "rejected at completion"
    );
}
