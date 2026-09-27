//! `.craftkit` end to end.

use cimmeria_entity::inventory::INV_CRAFTING;
use cimmeria_wire::crafting::GmCraftGrantKind;
use tracing::Level;

use super::*;
use crate::base::crafting::test_packets::update_item_rows;
use crate::mercury::method_idx;
use crate::test_support::{require_db_or_skip, LogCapture};

/// Steel Core (Materials), blueprint 25's set-1 component: `{17,15}`, does
/// not stack.
const STEEL_CORE: i32 = 5254;

fn kit(blueprint_id: i32, count: i32) -> GmCraftGrantKind {
    GmCraftGrantKind::Kit {
        blueprint_id,
        count,
    }
}

/// `.craftkit 25 2` puts 2 x 13 Steel Cores in the target's crafting bag,
/// one per slot (they do not stack), tells the target's client in one
/// `onUpdateItem`, queues one cell event per item, and logs `gm_craftkit`
/// with every granted stack as `type_id:bag:slot:before→after`.
#[tokio::test]
async fn craftkit_grants_set_one_into_the_crafting_bag() {
    let pool = require_db_or_skip!();
    let ids = ids(0);
    let capture = LogCapture::install();
    cleanup(&pool, ids).await;
    insert_player(&pool, ids).await;
    let maps = sessions(ids, 2);

    let typed = run(Some(&pool), ids, &maps, kit(25, 2)).await;
    let held = inventory(&pool, ids).await;
    let outbox: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM cell_event_outbox WHERE entity_id = $1 ORDER BY id",
    )
    .bind(ids.account)
    .fetch_all(&pool)
    .await
    .expect("outbox");
    cleanup(&pool, ids).await;

    assert_eq!(held, vec![(STEEL_CORE, INV_CRAFTING, 1); 26]);
    assert_eq!(outbox, vec!["inventory_item_granted".to_string(); 26]);
    let calls = target_calls(&typed, ids);
    assert_eq!(calls.len(), 1, "one inventory update: {calls:?}");
    assert_eq!(calls[0].method, method_idx::ON_UPDATE_ITEM);
    let rows = update_item_rows(&calls[0]);
    assert_eq!(rows.len(), 26);
    assert!(rows
        .iter()
        .all(|&(_, stack, container, _)| stack == 1 && container == INV_CRAFTING));

    let lines = gm_lines(&typed, ids);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with(&format!(
            "craftkit [{}]: blueprint 25 x2, 26 items granted (5254:15:",
            ids.target
        )),
        "{lines:?}"
    );

    let e = capture
        .find_message(Level::INFO, "craftkit granted")
        .expect("gm_craftkit granted");
    assert_eq!(e.target, "crafting");
    for (field, value) in [
        ("event", "gm_craftkit"),
        ("outcome", "granted"),
        ("blueprint_id", "25"),
        ("count", "2"),
        ("component_set", "1"),
    ] {
        assert!(e.has_field(field, value), "{field}: {e:#?}");
    }
    assert_identity(&e, ids);
    let granted = e.fields.get("granted").cloned().expect("granted field");
    let stacks: Vec<&str> = granted.split(',').collect();
    assert_eq!(stacks.len(), 26, "{granted}");
    assert!(stacks
        .iter()
        .all(|s| s.starts_with("5254:15:") && s.ends_with(":0→1")));
}

/// Every refusal grants nothing, sends the target nothing and tells the GM
/// why in one line; the event names the reason. A kit bigger than the
/// crafting bag (10 x 13 cores in 100 slots) is refused by the transaction
/// and rolls back whole.
#[tokio::test]
async fn craftkit_refusals_grant_nothing_and_answer_the_gm() {
    let pool = require_db_or_skip!();
    let ids = ids(1);
    let capture = LogCapture::install();
    cleanup(&pool, ids).await;
    insert_player(&pool, ids).await;
    let maps = sessions(ids, 2);

    let cases = [
        (
            kit(999_999, 1),
            "unknown_blueprint",
            "craftkit: refused, there is no blueprint 999999.".to_string(),
        ),
        (
            kit(21, 1),
            "no_components",
            "craftkit: refused, blueprint 21 has no component set 1.".to_string(),
        ),
        (
            kit(25, 0),
            "bad_count",
            "craftkit: refused, count 0 is not between 1 and 10.".to_string(),
        ),
        (
            kit(25, 10),
            "inventory_full",
            "craftkit: refused (inventory_full), the target's bags cannot take the kit; \
             nothing was granted."
                .to_string(),
        ),
    ];
    let mut sent = Vec::new();
    for (grant, _, _) in &cases {
        sent.push(run(Some(&pool), ids, &maps, *grant).await);
    }
    let held = inventory(&pool, ids).await;
    let outbox: i64 =
        sqlx::query_scalar("SELECT count(*) FROM cell_event_outbox WHERE entity_id = $1")
            .bind(ids.account)
            .fetch_one(&pool)
            .await
            .expect("outbox");
    cleanup(&pool, ids).await;

    assert!(held.is_empty(), "{held:?}");
    assert_eq!(outbox, 0);
    for ((_, reason, line), typed) in cases.iter().zip(&sent) {
        assert!(target_calls(typed, ids).is_empty(), "{reason}");
        assert_eq!(gm_lines(typed, ids), vec![line.clone()], "{reason}");
        let e = capture
            .find_event(Level::INFO, "craftkit refused", reason)
            .unwrap_or_else(|| panic!("gm_craftkit refused {reason}"));
        assert!(e.has_field("event", "gm_craftkit"), "{e:#?}");
        assert!(e.has_field("outcome", "refused"), "{e:#?}");
        assert_identity(&e, ids);
    }
}

/// A caller below GameMaster writes nothing: WARN `reason=not_gm`, and the
/// caller is told.
#[tokio::test]
async fn craftkit_from_a_non_gm_is_refused() {
    let pool = require_db_or_skip!();
    let ids = ids(2);
    let capture = LogCapture::install();
    cleanup(&pool, ids).await;
    insert_player(&pool, ids).await;
    let maps = sessions(ids, 0);

    let typed = run(Some(&pool), ids, &maps, kit(25, 1)).await;
    let held = inventory(&pool, ids).await;
    cleanup(&pool, ids).await;

    assert!(held.is_empty(), "{held:?}");
    assert!(target_calls(&typed, ids).is_empty());
    assert_eq!(
        gm_lines(&typed, ids),
        vec!["craftkit: refused, GameMaster access is required.".to_string()]
    );
    let e = capture
        .find_event(Level::WARN, "below GameMaster", "not_gm")
        .expect("not_gm WARN");
    assert!(e.has_field("event", "gm_craftkit"), "{e:#?}");
    assert!(e.has_field("access_level", "0"), "{e:#?}");
    assert_identity(&e, ids);
}

/// Without a database the grant is a `lookup_failed` WARN and a line.
#[tokio::test]
async fn craftkit_without_a_database_warns_and_answers() {
    let ids = ids(3);
    let capture = LogCapture::install();
    let maps = sessions(ids, 2);

    let typed = run(None, ids, &maps, kit(25, 1)).await;

    assert_eq!(
        gm_lines(&typed, ids),
        vec!["craftkit: failed, no database.".to_string()]
    );
    let e = capture
        .find_message(Level::WARN, "could not read what it needs")
        .expect("lookup_failed WARN");
    assert!(e.has_field("event", "lookup_failed"), "{e:#?}");
    assert!(e.has_field("phase", "db_pool"), "{e:#?}");
    assert!(e.has_field("command", "craftkit"), "{e:#?}");
    assert_identity(&e, ids);
}
