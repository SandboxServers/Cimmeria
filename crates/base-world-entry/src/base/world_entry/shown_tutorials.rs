//! Persistence leg of the one-time tutorial (Class Start v6, CS-03):
//! `CellToBaseMsg::RecordTutorialShown`, from the content `show_tutorial`
//! action.
//!
//! The insert is the decision. `sgw_player_tutorials` has the primary key
//! `(player_id, tutorial_id)`, so `ON CONFLICT DO NOTHING` adds a row exactly
//! once per character and tutorial, and only that first insert answers
//! [`TutorialRecordOutcome::First`], the one outcome on which the cell
//! displays the dialog. The account resolved from the session is the
//! ownership predicate, as in `looted_containers`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::cell::messages::{
    BaseToCellMsg, RecordTutorialShown, TutorialRecordOutcome, TutorialRecorded,
};

use super::super::session_identity::identity_for_entity;
use super::super::ConnectedClientState;

/// `event` of every row (target `content`), shared with the cell's half.
const EVENT: &str = "content_show_tutorial";

/// What [`record_tutorial_shown`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TutorialInsert {
    /// A new row: the first time this character is shown the tutorial.
    Inserted,
    /// The row already existed.
    Existing,
    /// `(player_id, account_id)` names no character: nothing written.
    NotOwned,
}

/// Record that `player_id` (owned by `account_id`) has been shown
/// `tutorial_id`, at most once.
pub(crate) async fn record_tutorial_shown(
    pool: &PgPool,
    player_id: i32,
    account_id: i32,
    tutorial_id: i32,
) -> Result<TutorialInsert, sqlx::Error> {
    let (owned, inserted): (bool, bool) = sqlx::query_as(
        "WITH owner AS ( \
             SELECT player_id FROM sgw_player WHERE player_id = $1 AND account_id = $2 \
         ), ins AS ( \
             INSERT INTO sgw_player_tutorials (player_id, tutorial_id) \
             SELECT player_id, $3 FROM owner \
             ON CONFLICT (player_id, tutorial_id) DO NOTHING \
             RETURNING 1 \
         ) \
         SELECT EXISTS (SELECT 1 FROM owner), EXISTS (SELECT 1 FROM ins)",
    )
    .bind(player_id)
    .bind(account_id)
    .bind(tutorial_id)
    .fetch_one(pool)
    .await?;
    Ok(match (owned, inserted) {
        (false, _) => TutorialInsert::NotOwned,
        (true, true) => TutorialInsert::Inserted,
        (true, false) => TutorialInsert::Existing,
    })
}

/// Handle `CellToBaseMsg::RecordTutorialShown` and answer the cell with
/// `BaseToCellMsg::TutorialRecorded`.
pub(crate) async fn handle_record_tutorial_shown(
    record: RecordTutorialShown,
    db_pool: &Option<Arc<PgPool>>,
    connected: &Arc<Mutex<HashMap<SocketAddr, ConnectedClientState>>>,
    entity_to_addr: &Arc<Mutex<HashMap<u32, SocketAddr>>>,
    cell_tx: &Option<mpsc::Sender<BaseToCellMsg>>,
) {
    let RecordTutorialShown {
        entity_id,
        player_id,
        chain_id,
        tutorial_id,
    } = record;
    let who = identity_for_entity(connected, entity_to_addr, entity_id);
    let tutorial_name = cimmeria_names::book()
        .dialog(tutorial_id)
        .map(str::to_owned);
    let chain_name = cimmeria_names::book().chain(chain_id).map(str::to_owned);

    // The session's account, only while it still plays `player_id`.
    let owning_account = who.account_id.filter(|_| who.player_id == Some(player_id));
    let outcome = match (db_pool, owning_account) {
        (None, _) => {
            tracing::warn!(
                target: "content",
                event = EVENT,
                decision_outcome = "refused",
                reason = "no_database",
                entity_id,
                entity_name = who.player_name,
                player_id,
                player_name = who.player_name,
                chain_id,
                chain_name = chain_name.as_deref(),
                tutorial_id,
                tutorial_name = tutorial_name.as_deref(),
                "RecordTutorialShown: no DB pool -- the tutorial cannot be recorded, so it \
                 is not shown"
            );
            TutorialRecordOutcome::Refused
        }
        (Some(_), None) => {
            // A recycled entity id, or a session that swapped characters
            // while the request was in flight: writing would mark the
            // wrong character.
            tracing::warn!(
                target: "content",
                event = EVENT,
                decision_outcome = "refused",
                reason = "session_mismatch",
                entity_id,
                entity_name = who.player_name,
                account_id = who.account_id,
                account_name = who.account_name,
                player_id,
                session_player_id = who.player_id, // nt:id-only the session's player, named by player_name on this row
                player_name = who.player_name,
                chain_id,
                chain_name = chain_name.as_deref(),
                tutorial_id,
                tutorial_name = tutorial_name.as_deref(),
                "RecordTutorialShown: the entity's session does not play that character; \
                 nothing recorded"
            );
            TutorialRecordOutcome::Refused
        }
        (Some(pool), Some(account_id)) => {
            match record_tutorial_shown(pool, player_id, account_id as i32, tutorial_id).await {
                Ok(TutorialInsert::Inserted) => {
                    tracing::info!(
                        target: "content",
                        event = EVENT,
                        decision_outcome = "recorded",
                        entity_id,
                        entity_name = who.player_name,
                        account_id,
                        account_name = who.account_name,
                        player_id,
                        player_name = who.player_name,
                        chain_id,
                        chain_name = chain_name.as_deref(),
                        tutorial_id,
                        tutorial_name = tutorial_name.as_deref(),
                        "RecordTutorialShown: first time for this character; recorded"
                    );
                    TutorialRecordOutcome::First
                }
                Ok(TutorialInsert::Existing) => TutorialRecordOutcome::AlreadyShown,
                Ok(TutorialInsert::NotOwned) => {
                    tracing::warn!(
                        target: "content",
                        event = EVENT,
                        decision_outcome = "refused",
                        reason = "player_row_missing",
                        entity_id,
                        entity_name = who.player_name,
                        account_id,
                        account_name = who.account_name,
                        player_id,
                        player_name = who.player_name,
                        chain_id,
                        chain_name = chain_name.as_deref(),
                        tutorial_id,
                        tutorial_name = tutorial_name.as_deref(),
                        rows_affected = 0,
                        expected = 1,
                        "RecordTutorialShown: the player/account pair names no character; \
                         nothing recorded"
                    );
                    TutorialRecordOutcome::Refused
                }
                Err(e) => {
                    tracing::error!(
                        target: "content",
                        event = EVENT,
                        decision_outcome = "refused",
                        reason = "db_error",
                        entity_id,
                        entity_name = who.player_name,
                        account_id,
                        account_name = who.account_name,
                        player_id,
                        player_name = who.player_name,
                        chain_id,
                        chain_name = chain_name.as_deref(),
                        tutorial_id,
                        tutorial_name = tutorial_name.as_deref(),
                        error = %e,
                        "RecordTutorialShown: insert failed; nothing recorded or shown"
                    );
                    TutorialRecordOutcome::Refused
                }
            }
        }
    };

    let reply = BaseToCellMsg::TutorialRecorded(TutorialRecorded {
        entity_id,
        player_id,
        chain_id,
        tutorial_id,
        outcome,
    });
    let sent = match cell_tx {
        Some(tx) => tx.send(reply).await.is_ok(),
        None => false,
    };
    if !sent {
        // First: the row is written but the dialog never opens, and the
        // next world entry reads it as shown. Rare (the cell is gone).
        tracing::error!(
            target: "content",
            event = EVENT,
            decision_outcome = "reply_send_failed",
            reason = "no_cell_channel",
            entity_id,
            entity_name = who.player_name,
            account_id = who.account_id,
            account_name = who.account_name,
            player_id,
            player_name = who.player_name,
            chain_id,
            chain_name = chain_name.as_deref(),
            tutorial_id,
            tutorial_name = tutorial_name.as_deref(),
            record_outcome = outcome.as_str(),
            "RecordTutorialShown: the cell was not told the outcome; the tutorial is not shown"
        );
    }
}

#[cfg(test)]
mod tests {
    //! Live-DB guards for the one-time tutorial record (CS-03).
    use super::*;
    use crate::test_support::require_db_or_skip;

    const TUTORIAL: i32 = 5882;

    async fn cleanup(pool: &PgPool, id: i32) {
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(id)
            .execute(pool)
            .await;
    }

    /// An account plus one Soldier, both keyed by the sentinel `id`.
    async fn seed_player(pool: &PgPool, id: i32) {
        cleanup(pool, id).await;
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(id)
            .bind(format!("cs03-tutorial-{id}"))
            .execute(pool)
            .await
            .expect("insert account");
        sqlx::query(
            "INSERT INTO sgw_player (\
                account_id, player_id, level, alignment, archetype, gender, \
                player_name, extra_name, world_location, bodyset, \
                pos_x, pos_y, pos_z, skin_color_id\
             ) VALUES ($1, $1, 1, 0, 1, 1, $2, '', 'Castle_CellBlock', \
                       'BS_HumanMale.BS_HumanMale', 0.0, 0.0, 0.0, 0)",
        )
        .bind(id)
        .bind(format!("cs03-tutorial-{id}"))
        .execute(pool)
        .await
        .expect("insert player");
    }

    async fn rows(pool: &PgPool, id: i32) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM sgw_player_tutorials WHERE player_id = $1")
            .bind(id)
            .fetch_one(pool)
            .await
            .expect("count rows")
    }

    /// **Guard: the first record is the only "show".** The first insert
    /// answers `Inserted`, a replay `Existing`, and the wrong account
    /// `NotOwned`, which writes nothing. Drop `ON CONFLICT DO NOTHING` and
    /// the replay errors; answer from `owner` alone and every replay reads
    /// as a first time.
    #[tokio::test]
    async fn live_db_record_tutorial_shown_reports_first_only_once() {
        const ID: i32 = 0x7030_0350;
        let pool = require_db_or_skip!();
        seed_player(&pool, ID).await;

        let first = record_tutorial_shown(&pool, ID, ID, TUTORIAL).await;
        let again = record_tutorial_shown(&pool, ID, ID, TUTORIAL).await;
        let stranger = record_tutorial_shown(&pool, ID, ID + 1, 5883).await;
        let count = rows(&pool, ID).await;
        cleanup(&pool, ID).await;

        assert_eq!(first.unwrap(), TutorialInsert::Inserted);
        assert_eq!(again.unwrap(), TutorialInsert::Existing);
        assert_eq!(stranger.unwrap(), TutorialInsert::NotOwned);
        assert_eq!(count, 1, "only the owner's first record is written");
    }

    /// **Guard: deleting the character deletes its tutorial rows** (the
    /// `ON DELETE CASCADE` foreign key in `_foreign_keys.sql`). Without the
    /// cascade the delete fails on the foreign key, or, with no key at all,
    /// the row outlives the character.
    #[tokio::test]
    async fn live_db_tutorial_rows_cascade_with_the_character() {
        const ID: i32 = 0x7030_0351;
        let pool = require_db_or_skip!();
        seed_player(&pool, ID).await;
        record_tutorial_shown(&pool, ID, ID, TUTORIAL)
            .await
            .expect("record");
        assert_eq!(rows(&pool, ID).await, 1);

        let deleted = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(ID)
            .execute(&pool)
            .await;
        let after = rows(&pool, ID).await;
        cleanup(&pool, ID).await;

        deleted.expect("deleting the character must not be blocked by its tutorial rows");
        assert_eq!(after, 0, "the character's tutorial rows go with it");
    }

    /// **Guard: the handler answers `First` once, then `AlreadyShown`, and
    /// `Refused` for a session that plays another character.** The reply is
    /// what the cell displays on, so this is the "shown exactly once"
    /// contract at the base.
    #[tokio::test]
    async fn live_db_handler_answers_first_then_already_shown() {
        const ID: i32 = 0x7030_0352;
        const ENTITY: u32 = 4242;
        let pool = require_db_or_skip!();
        seed_player(&pool, ID).await;

        let addr: SocketAddr = "127.0.0.1:40352".parse().unwrap();
        let mut state = crate::test_support::test_default_connected_client_state();
        state.account_id = ID as u32;
        state.active_player_id = Some(ID);
        let connected = Arc::new(Mutex::new(HashMap::from([(addr, state)])));
        let entity_to_addr = Arc::new(Mutex::new(HashMap::from([(ENTITY, addr)])));
        let (tx, mut rx) = mpsc::channel(8);
        let cell_tx = Some(tx);
        let db_pool = Some(Arc::new(pool.clone()));

        let request = |player_id| RecordTutorialShown {
            entity_id: ENTITY,
            player_id,
            chain_id: 7101,
            tutorial_id: TUTORIAL,
        };
        let mut outcomes = Vec::new();
        for player_id in [ID, ID, ID + 1] {
            handle_record_tutorial_shown(
                request(player_id),
                &db_pool,
                &connected,
                &entity_to_addr,
                &cell_tx,
            )
            .await;
            match rx.try_recv() {
                Ok(BaseToCellMsg::TutorialRecorded(r)) => outcomes.push(r.outcome),
                _ => panic!("expected one TutorialRecorded reply per request"),
            }
        }
        cleanup(&pool, ID).await;

        assert_eq!(
            outcomes,
            vec![
                TutorialRecordOutcome::First,
                TutorialRecordOutcome::AlreadyShown,
                TutorialRecordOutcome::Refused,
            ]
        );
    }
}
