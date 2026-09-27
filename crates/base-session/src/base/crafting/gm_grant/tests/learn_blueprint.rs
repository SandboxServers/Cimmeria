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

/// Live DB, the lost-update guard: a crafting completion holds the
/// player-wide advisory key (not the `sgw_player` row) while it writes
/// expertise. `.learnblueprint` rewrites every expertise row from its load,
/// so it must take that key first: it waits for the completion, and the
/// completion's +1 survives. Without the key it loads expertise 10, blocks
/// only on the expertise row, and writes 10 back over the committed 11.
#[tokio::test]
async fn learnblueprint_waits_for_a_completion_and_keeps_its_expertise() {
    let pool = require_db_or_skip!();
    // Slot 12 (`0x7000_CBB0`), past the vendor test's `0x7000_CBA0..CBA1`.
    let ids = ids(12);
    cleanup(&pool, ids).await;
    insert_player(&pool, ids).await;
    sqlx::query("UPDATE sgw_player SET discipline_ids = '{78}' WHERE player_id = $1")
        .bind(ids.player)
        .execute(&pool)
        .await
        .expect("know discipline 78");
    sqlx::query(
        "INSERT INTO sgw_player_discipline_expertise (player_id, discipline_id, expertise) \
         VALUES ($1, 78, 10)",
    )
    .bind(ids.player)
    .execute(&pool)
    .await
    .expect("expertise 10");
    let maps = sessions(ids, 2);

    // The completion: the player-wide key, then its uncommitted +1.
    let mut completion = pool.begin().await.expect("begin");
    sqlx::query("SELECT pg_advisory_xact_lock($1, 0)")
        .bind(ids.player)
        .execute(&mut *completion)
        .await
        .expect("player-wide key");
    sqlx::query(
        "UPDATE sgw_player_discipline_expertise SET expertise = 11 \
         WHERE player_id = $1 AND discipline_id = 78",
    )
    .bind(ids.player)
    .execute(&mut *completion)
    .await
    .expect("uncommitted +1");

    let (grant_pool, grant_maps) = (pool.clone(), maps.clone());
    let grant = tokio::spawn(async move {
        run(Some(&grant_pool), ids, &grant_maps, learn(25)).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let finished_while_held = grant.is_finished();
    completion.commit().await.expect("commit the completion");
    tokio::time::timeout(std::time::Duration::from_secs(10), grant)
        .await
        .expect("the grant finishes once the key is free")
        .expect("grant task");
    let reloaded = load_crafting_state(&pool, ids.player).await;
    cleanup(&pool, ids).await;
    let reloaded = reloaded.expect("reload");

    assert!(
        !finished_while_held,
        "the grant waits for the player-wide key"
    );
    assert_eq!(
        reloaded.get_expertise(78),
        Some(11),
        "the completion's committed +1 survives the grant"
    );
    assert_eq!(reloaded.blueprint_ids, vec![25], "and the grant landed");
}
