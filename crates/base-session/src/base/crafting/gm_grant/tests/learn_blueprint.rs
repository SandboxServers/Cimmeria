//! `.learnblueprint` end to end.

use cimmeria_wire::cell::client_methods::player::ON_UPDATE_KNOWN_CRAFTS;
use cimmeria_wire::crafting::{known_crafts_args, GmCraftGrantKind};
use tracing::Level;

use super::*;
use crate::base::crafting::persistence::load_crafting_state;
use crate::test_support::{require_db_or_skip, LogCapture};

fn learn(blueprint_id: i32) -> GmCraftGrantKind {
    GmCraftGrantKind::LearnBlueprint { blueprint_id }
}

async fn set_known(pool: &PgPool, ids: Ids, blueprint_ids: &[i32]) {
    sqlx::query("UPDATE sgw_player SET blueprint_ids = $2 WHERE player_id = $1")
        .bind(ids.player)
        .bind(blueprint_ids)
        .execute(pool)
        .await
        .expect("set known blueprints");
}

/// `.learnblueprint 25` saves the blueprint in the player's sorted list,
/// sends the target the full list in 139, logs `blueprint_learned` in the
/// Blueprint item's shape with `source=gm`, and tells the GM.
#[tokio::test]
async fn learnblueprint_teaches_saves_and_pushes_the_list() {
    let pool = require_db_or_skip!();
    let ids = ids(4);
    let capture = LogCapture::install();
    cleanup(&pool, ids).await;
    insert_player(&pool, ids).await;
    set_known(&pool, ids, &[30]).await;
    let maps = sessions(ids, 2);

    let typed = run(Some(&pool), ids, &maps, learn(25)).await;
    let reloaded = load_crafting_state(&pool, ids.player).await;
    cleanup(&pool, ids).await;

    assert_eq!(reloaded.expect("reload").blueprint_ids, vec![25, 30]);
    let calls = target_calls(&typed, ids);
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].method, ON_UPDATE_KNOWN_CRAFTS);
    assert_eq!(calls[0].entity_id, ids.target);
    assert_eq!(calls[0].args, known_crafts_args(&[25, 30]));
    assert_eq!(
        gm_lines(&typed, ids),
        vec![format!(
            "learnblueprint [{}]: blueprint 25 learned (2 known).",
            ids.target
        )]
    );

    let e = capture
        .find_message(Level::INFO, "blueprint taught by a GM")
        .expect("blueprint_learned");
    assert_eq!(e.target, "crafting");
    for (field, value) in [
        ("event", "blueprint_learned"),
        ("source", "gm"),
        ("blueprint_id", "25"),
        ("blueprints", "25:false→true"),
        ("known_before", "1"),
        ("known_after", "2"),
    ] {
        assert!(e.has_field(field, value), "{field}: {e:#?}");
    }
    assert_identity(&e, ids);
}

/// A known blueprint and an unknown id are refused: nothing saved, nothing
/// sent to the target, one line each for the GM, the reason on the event.
#[tokio::test]
async fn learnblueprint_refusals_change_nothing() {
    let pool = require_db_or_skip!();
    let ids = ids(5);
    let capture = LogCapture::install();
    cleanup(&pool, ids).await;
    insert_player(&pool, ids).await;
    set_known(&pool, ids, &[25]).await;
    let maps = sessions(ids, 2);

    let known = run(Some(&pool), ids, &maps, learn(25)).await;
    let unknown = run(Some(&pool), ids, &maps, learn(999_999)).await;
    let reloaded = load_crafting_state(&pool, ids.player).await;
    cleanup(&pool, ids).await;

    assert_eq!(reloaded.expect("reload").blueprint_ids, vec![25]);
    for (typed, reason, line) in [
        (
            &known,
            "already_known",
            format!(
                "learnblueprint [{}]: refused, blueprint 25 is already known.",
                ids.target
            ),
        ),
        (
            &unknown,
            "unknown_blueprint",
            "learnblueprint: refused, there is no blueprint 999999.".to_string(),
        ),
    ] {
        assert!(target_calls(typed, ids).is_empty(), "{reason}");
        assert_eq!(gm_lines(typed, ids), vec![line], "{reason}");
        let e = capture
            .find_event(Level::INFO, "learnblueprint refused", reason)
            .unwrap_or_else(|| panic!("gm_learnblueprint refused {reason}"));
        assert!(e.has_field("event", "gm_learnblueprint"), "{e:#?}");
        assert!(e.has_field("outcome", "refused"), "{e:#?}");
        assert_identity(&e, ids);
    }
}

/// A caller below GameMaster teaches nothing.
#[tokio::test]
async fn learnblueprint_from_a_non_gm_is_refused() {
    let pool = require_db_or_skip!();
    let ids = ids(6);
    let capture = LogCapture::install();
    cleanup(&pool, ids).await;
    insert_player(&pool, ids).await;
    let maps = sessions(ids, 1);

    let typed = run(Some(&pool), ids, &maps, learn(25)).await;
    let reloaded = load_crafting_state(&pool, ids.player).await;
    cleanup(&pool, ids).await;

    assert!(reloaded.expect("reload").blueprint_ids.is_empty());
    assert!(target_calls(&typed, ids).is_empty());
    assert_eq!(
        gm_lines(&typed, ids),
        vec!["learnblueprint: refused, GameMaster access is required.".to_string()]
    );
    let e = capture
        .find_event(Level::WARN, "below GameMaster", "not_gm")
        .expect("not_gm WARN");
    assert!(e.has_field("event", "gm_learnblueprint"), "{e:#?}");
    assert_identity(&e, ids);
}

/// A player row that is not there is a WARN `persist_failed` naming the
/// phase, with the paired `rows_affected` / `expected`.
#[tokio::test]
async fn learnblueprint_for_a_missing_player_logs_the_shortfall() {
    let pool = require_db_or_skip!();
    let ids = ids(7);
    let capture = LogCapture::install();
    cleanup(&pool, ids).await;
    let maps = sessions(ids, 2);

    let typed = run(Some(&pool), ids, &maps, learn(25)).await;

    assert!(target_calls(&typed, ids).is_empty());
    assert_eq!(
        gm_lines(&typed, ids),
        vec!["learnblueprint: failed, the crafting state could not be saved.".to_string()]
    );
    let e = capture
        .find_message(Level::WARN, "learnblueprint save failed")
        .expect("persist_failed");
    for (field, value) in [
        ("event", "persist_failed"),
        ("phase", "load_crafting_state_locked"),
        ("rows_affected", "0"),
        ("expected", "1"),
    ] {
        assert!(e.has_field(field, value), "{field}: {e:#?}");
    }
    assert_identity(&e, ids);
}
