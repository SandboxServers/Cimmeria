//! Atomic Frost's Letter consumption and mission 1360 persistence fence.

use cimmeria_entity::inventory::INV_CRAFTING;
use sqlx::{PgPool, Postgres, Transaction};

use crate::base::inventory_locks::take_inventory_locks;
use crate::cell::messages::SavedMission;

const MISSION_ID: i32 = 1360;
const LETTER_TYPE_ID: i32 = 3730;
const ACTIVE_STEP: i32 = 4038;
const ACCESSIBLE_BAGS: [i32; 15] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, INV_CRAFTING];

/// The persisted result of a successful one-shot turn-in.
#[derive(Debug, Clone)]
pub struct CommittedLetterTurnIn {
    /// Mission state read from the same transaction that consumed the letter.
    pub mission: SavedMission,
    /// The unique inventory instance that supplied the letter.
    pub item_id: i32,
    /// Its accessible source container.
    pub container_id: i32,
    /// Stack count before consumption.
    pub stack_before: i32,
    /// Stack count after consumption; zero means the row was deleted.
    pub stack_after: i32,
}

/// A definitive refusal that did not consume a letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LetterTurnInRefusal {
    /// Attempt or session text is invalid.
    InvalidCorrelation,
    /// The player does not exist.
    NoPlayer,
    /// Another attempt already committed this one-shot mission.
    AlreadyCompleted,
    /// No owned letter is present in a carried, accessible container.
    NoAccessibleLetter,
    /// The mission is absent or differs from the exact Castle postimage.
    WrongMissionState,
    /// A locked row changed before the conditional mutation.
    StateChanged,
}

/// Outcome of the mission-specific transaction. SQL errors need same-attempt retry.
#[derive(Debug, Clone)]
pub enum LetterTurnInVerdict {
    /// The current call committed the item, mission and receipt together.
    Committed(CommittedLetterTurnIn),
    /// The same attempt/session had already committed and was reconciled.
    AlreadyCommitted(CommittedLetterTurnIn),
    /// No successful turn-in was recorded by this call.
    Refused(LetterTurnInRefusal),
}

#[derive(sqlx::FromRow)]
struct ReceiptRow {
    attempt_key: String,
    session_token: String,
    item_id: i32,
    container_id: i32,
    stack_before: i32,
    stack_after: i32,
}

#[derive(sqlx::FromRow)]
struct LetterRow {
    item_id: i32,
    container_id: i32,
    stack_size: i32,
}

#[derive(sqlx::FromRow)]
struct MissionRow {
    status: i32,
    current_step_id: Option<i32>,
    completed_step_ids: Vec<i32>,
    completed_objective_ids: Vec<i32>,
    active_objective_ids: Vec<i32>,
    failed_objective_ids: Vec<i32>,
    repeats: i32,
}

impl MissionRow {
    fn is_castle_postimage(&self) -> bool {
        self.status == 1
            && self.current_step_id == Some(ACTIVE_STEP)
            && self.completed_step_ids == [4037]
            && self.completed_objective_ids == [4650]
            && self.active_objective_ids == [4651]
            && self.failed_objective_ids.is_empty()
            && self.repeats == 0
    }

    fn is_terminal_postimage(&self) -> bool {
        self.status == 2
            && self.current_step_id.is_none()
            && self.completed_step_ids == [4037, ACTIVE_STEP]
            && self.completed_objective_ids == [4650, 4651]
            && self.active_objective_ids == [4651]
            && self.failed_objective_ids.is_empty()
            && self.repeats == 1
    }

    fn into_saved(self) -> SavedMission {
        SavedMission {
            mission_id: MISSION_ID,
            status: self.status as i8,
            current_step_id: self.current_step_id,
            completed_step_ids: self.completed_step_ids,
            completed_objective_ids: self.completed_objective_ids,
            active_objective_ids: self.active_objective_ids,
            failed_objective_ids: self.failed_objective_ids,
            repeats: self.repeats,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FailureStage {
    None,
    BeforeMissionWrite,
    BeforeReceiptInsert,
}

fn valid_correlation(attempt_key: &str, session_token: &str) -> bool {
    let attempt_valid = (1..=256).contains(&attempt_key.len())
        && attempt_key.bytes().all(|byte| byte.is_ascii_graphic());
    let session_valid = session_token.len() == 32
        && session_token
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    attempt_valid && session_valid
}

async fn read_mission(
    tx: &mut Transaction<'_, Postgres>,
    player_id: i32,
) -> Result<Option<MissionRow>, sqlx::Error> {
    sqlx::query_as::<_, MissionRow>(
        "SELECT status, current_step_id, completed_step_ids, completed_objective_ids, \
         active_objective_ids, failed_objective_ids, repeats FROM sgw_mission \
         WHERE player_id = $1 AND mission_id = 1360 FOR UPDATE",
    )
    .bind(player_id)
    .fetch_optional(&mut **tx)
    .await
}

/// Consume one owned Frost's Letter and complete mission 1360 in one transaction.
///
/// Callers must retry an SQL error with the same attempt and session: a commit
/// error can mean the transaction succeeded before its acknowledgement was lost.
pub async fn commit_letter_turn_in(
    pool: &PgPool,
    player_id: i32,
    attempt_key: &str,
    session_token: &str,
) -> Result<LetterTurnInVerdict, sqlx::Error> {
    commit_letter_turn_in_inner(
        pool,
        player_id,
        attempt_key,
        session_token,
        FailureStage::None,
    )
    .await
}

async fn commit_letter_turn_in_inner(
    pool: &PgPool,
    player_id: i32,
    attempt_key: &str,
    session_token: &str,
    failure_stage: FailureStage,
) -> Result<LetterTurnInVerdict, sqlx::Error> {
    if player_id <= 0 || !valid_correlation(attempt_key, session_token) {
        return Ok(LetterTurnInVerdict::Refused(
            LetterTurnInRefusal::InvalidCorrelation,
        ));
    }

    let mut tx = pool.begin().await?;
    // Generic mission writes take only a mission row. This path takes every
    // inventory advisory lock first, then player KEY SHARE, item, mission.
    take_inventory_locks(&mut tx, player_id, &ACCESSIBLE_BAGS).await?;
    let player: Option<i32> =
        sqlx::query_scalar("SELECT player_id FROM sgw_player WHERE player_id = $1 FOR KEY SHARE")
            .bind(player_id)
            .fetch_optional(&mut *tx)
            .await?;
    if player.is_none() {
        return Ok(LetterTurnInVerdict::Refused(LetterTurnInRefusal::NoPlayer));
    }

    let receipt = sqlx::query_as::<_, ReceiptRow>(
        "SELECT attempt_key, session_token, item_id, container_id, stack_before, stack_after \
         FROM sgw_mission_1360_receipt WHERE player_id = $1 AND mission_id = 1360",
    )
    .bind(player_id)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(receipt) = receipt {
        if receipt.attempt_key != attempt_key || receipt.session_token != session_token {
            return Ok(LetterTurnInVerdict::Refused(
                LetterTurnInRefusal::AlreadyCompleted,
            ));
        }
        let Some(mission) = read_mission(&mut tx, player_id).await? else {
            return Ok(LetterTurnInVerdict::Refused(
                LetterTurnInRefusal::WrongMissionState,
            ));
        };
        if !mission.is_terminal_postimage() {
            return Ok(LetterTurnInVerdict::Refused(
                LetterTurnInRefusal::WrongMissionState,
            ));
        }
        return Ok(LetterTurnInVerdict::AlreadyCommitted(
            CommittedLetterTurnIn {
                mission: mission.into_saved(),
                item_id: receipt.item_id,
                container_id: receipt.container_id,
                stack_before: receipt.stack_before,
                stack_after: receipt.stack_after,
            },
        ));
    }

    let letter = sqlx::query_as::<_, LetterRow>(
        "SELECT item_id, container_id, stack_size FROM sgw_inventory \
         WHERE character_id = $1 AND type_id = 3730 \
           AND container_id = ANY($2) AND stack_size >= 1 \
         ORDER BY container_id, slot_id, item_id LIMIT 1 FOR UPDATE",
    )
    .bind(player_id)
    .bind(&ACCESSIBLE_BAGS[..])
    .fetch_optional(&mut *tx)
    .await?;
    let Some(letter) = letter else {
        return Ok(LetterTurnInVerdict::Refused(
            LetterTurnInRefusal::NoAccessibleLetter,
        ));
    };
    let Some(mission) = read_mission(&mut tx, player_id).await? else {
        return Ok(LetterTurnInVerdict::Refused(
            LetterTurnInRefusal::WrongMissionState,
        ));
    };
    if !mission.is_castle_postimage() {
        return Ok(LetterTurnInVerdict::Refused(
            LetterTurnInRefusal::WrongMissionState,
        ));
    }

    let consumed = if letter.stack_size == 1 {
        sqlx::query(
            "DELETE FROM sgw_inventory WHERE item_id = $1 AND character_id = $2 \
             AND type_id = $3 AND container_id = $4 AND stack_size = 1",
        )
        .bind(letter.item_id)
        .bind(player_id)
        .bind(LETTER_TYPE_ID)
        .bind(letter.container_id)
        .execute(&mut *tx)
        .await?
    } else {
        sqlx::query(
            "UPDATE sgw_inventory SET stack_size = stack_size - 1 \
             WHERE item_id = $1 AND character_id = $2 AND type_id = $3 \
               AND container_id = $4 AND stack_size = $5 AND stack_size > 1",
        )
        .bind(letter.item_id)
        .bind(player_id)
        .bind(LETTER_TYPE_ID)
        .bind(letter.container_id)
        .bind(letter.stack_size)
        .execute(&mut *tx)
        .await?
    };
    if consumed.rows_affected() != 1 {
        return Ok(LetterTurnInVerdict::Refused(
            LetterTurnInRefusal::StateChanged,
        ));
    }
    if failure_stage == FailureStage::BeforeMissionWrite {
        return Err(sqlx::Error::Protocol(
            "letter turn-in test failure before mission write".into(),
        ));
    }

    let changed = sqlx::query(
        "UPDATE sgw_mission SET status = 2, current_step_id = NULL, repeats = 1, \
         completed_step_ids = array_append(completed_step_ids, 4038), \
         completed_objective_ids = array_append(completed_objective_ids, 4651) \
         WHERE player_id = $1 AND mission_id = 1360 AND status = 1 \
           AND current_step_id = 4038 AND repeats = 0 \
           AND completed_step_ids = ARRAY[4037]::integer[] \
           AND completed_objective_ids = ARRAY[4650]::integer[] \
           AND active_objective_ids = ARRAY[4651]::integer[] \
           AND failed_objective_ids = ARRAY[]::integer[]",
    )
    .bind(player_id)
    .execute(&mut *tx)
    .await?;
    if changed.rows_affected() != 1 {
        return Ok(LetterTurnInVerdict::Refused(
            LetterTurnInRefusal::StateChanged,
        ));
    }
    let Some(postimage) = read_mission(&mut tx, player_id).await? else {
        return Ok(LetterTurnInVerdict::Refused(
            LetterTurnInRefusal::StateChanged,
        ));
    };
    if !postimage.is_terminal_postimage() {
        return Ok(LetterTurnInVerdict::Refused(
            LetterTurnInRefusal::StateChanged,
        ));
    }
    if failure_stage == FailureStage::BeforeReceiptInsert {
        return Err(sqlx::Error::Protocol(
            "letter turn-in test failure before receipt insert".into(),
        ));
    }

    sqlx::query(
        "INSERT INTO sgw_mission_1360_receipt \
         (player_id, mission_id, attempt_key, session_token, item_id, container_id, \
          stack_before, stack_after) VALUES ($1, 1360, $2, $3, $4, $5, $6, $7)",
    )
    .bind(player_id)
    .bind(attempt_key)
    .bind(session_token)
    .bind(letter.item_id)
    .bind(letter.container_id)
    .bind(letter.stack_size)
    .bind(letter.stack_size - 1)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(LetterTurnInVerdict::Committed(CommittedLetterTurnIn {
        mission: postimage.into_saved(),
        item_id: letter.item_id,
        container_id: letter.container_id,
        stack_before: letter.stack_size,
        stack_after: letter.stack_size - 1,
    }))
}

/// Persist only the two exact active snapshots of mission 1360.
///
/// A is accepted on first write or repeated A; B requires prior A, and a
/// repeated B is idempotent. The conditional row write serializes with a1's
/// terminal update without ever holding an inventory row while waiting.
pub(super) async fn persist_active_letter_mission(
    pool: &PgPool,
    player_id: i32,
    status: i8,
    current_step_id: Option<i32>,
    completed_step_ids: &[i32],
    completed_objective_ids: &[i32],
    active_objective_ids: &[i32],
    failed_objective_ids: &[i32],
    repeats: i32,
) -> Result<bool, sqlx::Error> {
    let is_a = status == 1
        && current_step_id == Some(4037)
        && repeats == 0
        && completed_step_ids.is_empty()
        && completed_objective_ids.is_empty()
        && active_objective_ids == [4650]
        && failed_objective_ids.is_empty();
    let is_b = status == 1
        && current_step_id == Some(ACTIVE_STEP)
        && repeats == 0
        && completed_step_ids == [4037]
        && completed_objective_ids == [4650]
        && active_objective_ids == [4651]
        && failed_objective_ids.is_empty();
    if is_a {
        let done = sqlx::query(
            "INSERT INTO sgw_mission (player_id, mission_id, status, current_step_id, \
             completed_step_ids, completed_objective_ids, active_objective_ids, \
             failed_objective_ids, repeats) \
             VALUES ($1, 1360, 1, 4037, ARRAY[]::integer[], ARRAY[]::integer[], \
                     ARRAY[4650]::integer[], ARRAY[]::integer[], 0) \
             ON CONFLICT (player_id, mission_id) DO UPDATE SET \
             status = EXCLUDED.status, current_step_id = EXCLUDED.current_step_id, \
             completed_step_ids = EXCLUDED.completed_step_ids, \
             completed_objective_ids = EXCLUDED.completed_objective_ids, \
             active_objective_ids = EXCLUDED.active_objective_ids, \
             failed_objective_ids = EXCLUDED.failed_objective_ids, repeats = EXCLUDED.repeats \
             WHERE sgw_mission.status = 1 AND sgw_mission.current_step_id = 4037 \
               AND sgw_mission.repeats = 0 \
               AND sgw_mission.completed_step_ids = ARRAY[]::integer[] \
               AND sgw_mission.completed_objective_ids = ARRAY[]::integer[] \
               AND sgw_mission.active_objective_ids = ARRAY[4650]::integer[] \
               AND sgw_mission.failed_objective_ids = ARRAY[]::integer[]",
        )
        .bind(player_id)
        .execute(pool)
        .await?;
        return Ok(done.rows_affected() == 1);
    }
    if is_b {
        let done = sqlx::query(
            "UPDATE sgw_mission SET status = 1, current_step_id = 4038, repeats = 0, \
             completed_step_ids = ARRAY[4037]::integer[], \
             completed_objective_ids = ARRAY[4650]::integer[], \
             active_objective_ids = ARRAY[4651]::integer[], \
             failed_objective_ids = ARRAY[]::integer[] \
             WHERE player_id = $1 AND mission_id = 1360 AND status = 1 \
               AND repeats = 0 AND failed_objective_ids = ARRAY[]::integer[] \
               AND ((current_step_id = 4037 \
                     AND completed_step_ids = ARRAY[]::integer[] \
                     AND completed_objective_ids = ARRAY[]::integer[] \
                     AND active_objective_ids = ARRAY[4650]::integer[]) \
                    OR (current_step_id = 4038 \
                     AND completed_step_ids = ARRAY[4037]::integer[] \
                     AND completed_objective_ids = ARRAY[4650]::integer[] \
                     AND active_objective_ids = ARRAY[4651]::integer[]))",
        )
        .bind(player_id)
        .execute(pool)
        .await?;
        return Ok(done.rows_affected() == 1);
    }
    Ok(false)
}

#[cfg(test)]
pub(super) async fn commit_with_test_failure(
    pool: &PgPool,
    player_id: i32,
    attempt_key: &str,
    session_token: &str,
    after_mission_write: bool,
) -> Result<LetterTurnInVerdict, sqlx::Error> {
    let stage = if after_mission_write {
        FailureStage::BeforeReceiptInsert
    } else {
        FailureStage::BeforeMissionWrite
    };
    commit_letter_turn_in_inner(pool, player_id, attempt_key, session_token, stage).await
}
