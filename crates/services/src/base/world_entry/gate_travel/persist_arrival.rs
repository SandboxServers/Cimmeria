//! The single `sgw_player` write a committed gate arrival performs:
//! destination world, destination position, and the addresses the traveller
//! learned by making the trip.
//!
//! ## Why one statement
//!
//! These used to be two UPDATEs against the same row with the same key. One
//! statement gives atomicity for free, halves the round trip, and — the real
//! reason — removes a duplicated `WHERE player_id = $ AND account_id = $`
//! whose two copies had to stay byte-identical for the "no row returned just
//! means nothing new" reasoning to hold. The set-difference sub-SELECT stays
//! **inline and correlated** rather than lifted into a CTE: under READ
//! COMMITTED a correlated sub-SELECT in `SET` is re-evaluated against the
//! freshly-locked row on an EvalPlanQual recheck, while a CTE is a stable
//! subplan from the old snapshot and could double-append.
//!
//! ## Provenance — unlock-on-visit is new behaviour, not a restoration
//!
//! There is no learn-on-arrival anywhere in the 2009 Python.
//! `SGWPlayer.addStargateAddress` (`deprecated/python/cell/SGWPlayer.py:609`)
//! has exactly two callers: the GM console command `giveaddress`
//! (`deprecated/python/cell/commands/Player.py:74`) and the Atrea authoring
//! node `Act_StargateAddress`
//! (`deprecated/entities-editor/editor/Nodes.xml:2428`). Addresses were
//! authored content. Cimmeria has neither a content-engine equivalent of that
//! action nor any chain that grants an address, so with the dial gate now
//! enforced (CAT-O-01) this is the only grant path in the game.

use std::sync::Arc;

use sqlx::PgPool;

/// Persist the arrival and learn its addresses. Never fails the transfer: the
/// player has already been handed to the destination space by the time this
/// runs, so every branch here logs and returns.
///
/// `destination_gates` is the destination world's gate list, which the caller
/// already has. The **origin** world's gates are resolved inside the statement
/// from the row's own pre-update `world_location` — see
/// "Both ends of the trip" below.
///
/// `player_id`/`account_id` are the fail-closed pair the caller resolved
/// before the transfer became destructive. `account_id` is not redundant with
/// the `player_id` primary key: it is the ownership predicate that makes a
/// wrong `player_id` miss rather than land on a stranger's row.
///
/// ## Both ends of the trip, and the origin is the half that works
///
/// The packet's rule reads "learn the destination gate on arrival", but on the
/// dial route that is a no-op by construction:
/// `cell::gate_travel::handle_dial_gate` refuses an address the player does
/// not already hold, so a dialled destination is always already known. What a
/// traveller does *not* have is a way back — they may have been delivered
/// somewhere by content, by a GM, or by the respawn fork, none of which
/// consult the address book. Learning the world they departed from is what
/// makes the rule do anything.
///
/// The origin is read from `sgw_player.world_location` **inside** the `SET`,
/// where a reference to the target table still yields the pre-update value,
/// rather than from `ConnectedClientState::world_name`. That session field is
/// written once at `play_character` and **never updated on gate travel**, so
/// after the first hop it still names the login world — see the integration
/// request in the H06 worknote. The row is the only source that is correct on
/// every hop, and using it also saves a round trip.
pub(super) async fn persist_arrival(
    db_pool: &Option<Arc<PgPool>>,
    player_id: i32,
    account_id: u32,
    target_world_name: &str,
    position: [f32; 3],
    destination_gates: &[i32],
) {
    let Some(pool) = db_pool else { return };

    // `array_agg(DISTINCT t.x ORDER BY t.x)` rather than a plain aggregate:
    // the `NOT (t.x = ANY(known_stargates))` filter only compares against the
    // *pre-update* array, so a duplicate inside the union would pass twice and
    // be appended twice — and `resources.stargates.stargate_id` carries no
    // uniqueness constraint. `t.x IS NOT NULL` is the same class of insurance.
    //
    // The `w.world = sgw_player.world_location` correlation is the origin
    // half: inside an UPDATE's `SET`, a reference to the target table is the
    // row as it was before this statement, so this reads the world being left,
    // not the one being written on the line above. Verified against the live
    // schema across two consecutive hops.
    //
    // Note the result is "existing order, then a sorted block", not a sorted
    // array — `mercury::world_data::map_loaded` serialises it in array order
    // into the client's dial list, so it is worth being precise about.
    let res = sqlx::query_scalar::<_, Vec<i32>>(
        "UPDATE sgw_player \
            SET world_location = $1, \
                world_id = COALESCE((SELECT world_id FROM resources.worlds WHERE world = $1), world_id), \
                pos_x = $2, pos_y = $3, pos_z = $4, \
                known_stargates = known_stargates || ( \
                     SELECT COALESCE(array_agg(DISTINCT t.x ORDER BY t.x), '{}'::integer[]) \
                       FROM ( \
                             SELECT unnest($5::integer[]) AS x \
                             UNION ALL \
                             SELECT s.stargate_id \
                               FROM resources.stargates s \
                               JOIN resources.worlds w ON w.world_id = s.world_id \
                              WHERE w.world = sgw_player.world_location \
                            ) t \
                      WHERE t.x IS NOT NULL AND NOT (t.x = ANY(known_stargates)) \
                    ) \
          WHERE player_id = $6 AND account_id = $7 \
      RETURNING known_stargates",
    )
    .bind(target_world_name)
    .bind(position[0])
    .bind(position[1])
    .bind(position[2])
    .bind(destination_gates)
    .bind(player_id)
    .bind(account_id as i32)
    .fetch_optional(pool.as_ref())
    .await;

    match res {
        Ok(Some(known)) => {
            tracing::debug!(
                player_id,
                account_id,
                world_name = %target_world_name,
                known_count = known.len(),
                "GateTravel: destination persisted and the address book updated"
            );
        }
        Ok(None) => {
            // Same shape the pre-merge code warned on as `rows_affected == 0`.
            tracing::warn!(
                player_id,
                account_id,
                world_name = %target_world_name,
                rows_affected = 0,
                expected = 1,
                reason = "rows_affected_zero",
                "GateTravel: persistence UPDATE matched 0 rows -- the destination \
                 world, position and any learned stargate addresses are all lost; \
                 a relog will drop the player back at their pre-gate location"
            );
        }
        Err(e) => {
            tracing::error!(
                player_id,
                account_id,
                world_name = %target_world_name,
                reason = "persist_arrival_failed",
                "GateTravel: failed to persist the arrival ({e}) -- a relog will drop \
                 the player back at their pre-gate location and they may have no way \
                 to dial out of this world"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::require_db_or_skip;

    /// Sentinel range for H06, per the Harset packet allocation
    /// (`0x7006_xxxx`). Cleanup deletes by exact id, never by range.
    const TEST_BASE: i32 = 0x7006_0600;

    /// World 12 (`Castle_CellBlock`) has no gate; `Castle` (world 8) does.
    /// Only used for the world_location FK, which must name a real world.
    const A_REAL_WORLD: &str = "Castle";

    #[tokio::test]
    async fn no_db_pool_is_a_silent_noop() {
        persist_arrival(&None, 1, 1, "Harset", [0.0; 3], &[3]).await;
    }

    async fn cleanup(pool: &PgPool, account_id: i32, player_id: i32) {
        let _ = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1")
            .bind(player_id)
            .execute(pool)
            .await;
        let _ = sqlx::query("DELETE FROM account WHERE account_id = $1")
            .bind(account_id)
            .execute(pool)
            .await;
    }

    async fn seed(pool: &PgPool, account_id: i32, player_id: i32, known: &[i32]) {
        cleanup(pool, account_id, player_id).await;
        sqlx::query("INSERT INTO account (account_id, account_name, password) VALUES ($1, $2, '')")
            .bind(account_id)
            .bind(format!("h06-{account_id}"))
            .execute(pool)
            .await
            .expect("insert sentinel account");
        // `extra_name` and `bodyset` are NOT NULL with a `NULL::varchar`
        // default, so they have to be named explicitly even though nothing
        // here reads them.
        sqlx::query(
            "INSERT INTO sgw_player \
               (account_id, player_id, player_name, extra_name, bodyset, world_location, \
                alignment, archetype, gender, pos_x, pos_y, pos_z, skin_color_id, \
                known_stargates) \
             VALUES ($1, $2, $3, '', 'BS_HumanMale.BS_HumanMale', $4, \
                     1, 1, 1, 0, 0, 0, 0, $5::integer[])",
        )
        .bind(account_id)
        .bind(player_id)
        .bind(format!("H06Pin{player_id}"))
        .bind(A_REAL_WORLD)
        .bind(known)
        .execute(pool)
        .await
        .expect("insert sentinel player");
    }

    async fn row_of(pool: &PgPool, player_id: i32) -> (Vec<i32>, String, f32) {
        let r: (Vec<i32>, String, f32) = sqlx::query_as(
            "SELECT known_stargates, world_location, pos_x FROM sgw_player WHERE player_id = $1",
        )
        .bind(player_id)
        .fetch_one(pool)
        .await
        .expect("sentinel player row must exist");
        r
    }

    /// The exactly-once contract. Reverting the
    /// `WHERE NOT (x = ANY(known_stargates))` filter to a bare
    /// `array_append` duplicates ids and fails the second assertion; dropping
    /// the `DISTINCT` fails the first.
    #[tokio::test]
    async fn an_arrival_learns_each_address_exactly_once() {
        let pool = require_db_or_skip!();
        let (account_id, player_id) = (TEST_BASE, TEST_BASE + 10);
        seed(&pool, account_id, player_id, &[3]).await;

        let db = Some(Arc::new(pool.clone()));
        // 3 is already held; 41 appears twice in the input; 42 is new.
        persist_arrival(
            &db,
            player_id,
            account_id as u32,
            A_REAL_WORLD,
            [11.0, 12.0, 13.0],
            &[3, 41, 42, 41],
        )
        .await;
        let (known, _, _) = row_of(&pool, player_id).await;
        assert_eq!(
            known,
            vec![3, 41, 42],
            "an arrival appends exactly the addresses the player lacked, with a \
             duplicate in the input collapsing to one entry"
        );

        persist_arrival(
            &db,
            player_id,
            account_id as u32,
            A_REAL_WORLD,
            [11.0, 12.0, 13.0],
            &[3, 41, 42],
        )
        .await;
        let (known, _, _) = row_of(&pool, player_id).await;
        assert_eq!(
            known,
            vec![3, 41, 42],
            "a second arrival at the same world must not grow the address book"
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// The merge's own regression guard. If the address-learning half is ever
    /// re-gated with an `AND EXISTS (... NOT (x = ANY(known_stargates)))` in
    /// the `WHERE` — the obvious way to avoid a no-op write — then every
    /// arrival at an already-known world silently stops persisting
    /// `world_location` and `pos_*`, and the player rubber-bands to their
    /// pre-gate location on relog.
    #[tokio::test]
    async fn an_arrival_with_nothing_new_to_learn_still_persists_the_destination() {
        let pool = require_db_or_skip!();
        let (account_id, player_id) = (TEST_BASE + 1, TEST_BASE + 11);
        seed(&pool, account_id, player_id, &[3, 41]).await;

        let db = Some(Arc::new(pool.clone()));
        persist_arrival(
            &db,
            player_id,
            account_id as u32,
            A_REAL_WORLD,
            [777.5, 0.0, 0.0],
            &[3, 41],
        )
        .await;

        let (known, world, pos_x) = row_of(&pool, player_id).await;
        assert_eq!(known, vec![3, 41], "nothing new to learn");
        assert_eq!(world, A_REAL_WORLD);
        assert_eq!(
            pos_x, 777.5,
            "destination persistence must not be gated on having learned something"
        );

        cleanup(&pool, account_id, player_id).await;
    }

    /// The origin half of the rule, which is the half that does any work: a
    /// traveller delivered to a world by something that never consulted the
    /// address book (content, a GM, the respawn fork) must come away knowing
    /// the world they left, or they have no way to dial back.
    ///
    /// Castle (world 8) carries exactly one gate, `stargate_id = 2`. The
    /// player starts holding nothing and arrives somewhere with no gate at
    /// all, so `2` can only have come from the pre-update `world_location`.
    /// Reverting the `w.world = sgw_player.world_location` correlation leaves
    /// the book empty and fails this.
    #[tokio::test]
    async fn an_arrival_learns_the_world_the_traveller_left() {
        let pool = require_db_or_skip!();
        let (account_id, player_id) = (TEST_BASE + 3, TEST_BASE + 13);
        seed(&pool, account_id, player_id, &[]).await;

        let castle_gates: Vec<i32> = sqlx::query_scalar(
            "SELECT COALESCE(array_agg(s.stargate_id), '{}'::integer[]) \
               FROM resources.stargates s \
               JOIN resources.worlds w ON w.world_id = s.world_id \
              WHERE w.world = $1",
        )
        .bind(A_REAL_WORLD)
        .fetch_one(&pool)
        .await
        .expect("the seed must give Castle at least one gate");
        assert!(
            !castle_gates.is_empty(),
            "fixture assumption: {A_REAL_WORLD} has a seeded stargate"
        );

        // Depart Castle for a gateless world: nothing is contributed by the
        // destination, so whatever is learned came from the origin.
        let db = Some(Arc::new(pool.clone()));
        persist_arrival(
            &db,
            player_id,
            account_id as u32,
            "Castle_CellBlock",
            [0.0; 3],
            &[],
        )
        .await;

        let (known, world, _) = row_of(&pool, player_id).await;
        assert_eq!(
            known, castle_gates,
            "the departed world's addresses must be learned, read from the row's \
             pre-update world_location rather than from the session's stale copy"
        );
        assert_eq!(world, "Castle_CellBlock", "and the arrival still persists");

        cleanup(&pool, account_id, player_id).await;
    }

    /// `account_id` is an ownership predicate, not decoration: a write aimed
    /// at the right `player_id` from the wrong account must match no row, and
    /// must say so.
    #[tokio::test]
    async fn a_wrong_account_writes_nothing_and_warns() {
        let pool = require_db_or_skip!();
        let (account_id, player_id) = (TEST_BASE + 2, TEST_BASE + 12);
        seed(&pool, account_id, player_id, &[]).await;

        let capture = crate::test_support::LogCapture::install();
        let db = Some(Arc::new(pool.clone()));
        persist_arrival(
            &db,
            player_id,
            (account_id + 9999) as u32,
            A_REAL_WORLD,
            [999.0, 0.0, 0.0],
            &[41],
        )
        .await;

        let (known, _, pos_x) = row_of(&pool, player_id).await;
        assert!(known.is_empty(), "no address may be granted cross-account");
        assert_eq!(pos_x, 0.0, "no position may be written cross-account");
        assert!(
            capture
                .find_event(tracing::Level::WARN, "matched 0 rows", "rows_affected_zero")
                .is_some(),
            "negative-logging convention: a zero-row persistence UPDATE must warn"
        );

        cleanup(&pool, account_id, player_id).await;
    }
}
