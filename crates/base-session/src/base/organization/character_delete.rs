//! Deleting a character and logging what it did to its organizations.
//!
//! A character's member rows cascade from `sgw_player`, and the
//! member-delete trigger then promotes a new leader or disbands (D-ORG12).
//! The lock order is kept in the database, for every delete path: the
//! `sgw_player_before_delete_lock_orgs` trigger locks the character's
//! organizations (in `org_id` order) after the `sgw_player` row and before
//! the cascade reaches the member rows (`api` § "Lock order").
//!
//! [`delete_character`] adds the two things SQL cannot do: it refuses a
//! character the account does not own before touching anything, and after
//! its commit it logs the trigger's audit rows (`audit::export_committed`).

use sqlx::PgPool;

use super::audit::{current_tx_id, export_committed, ExportSource, OrgEventRow};

/// The result of [`delete_character`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterDeletion {
    /// `true` if a character was deleted.
    pub deleted: bool,
    /// What the member-delete trigger did to the character's organizations
    /// (promotions, disbands, memberless results), already logged at INFO
    /// on the `org` target and stamped exported.
    ///
    /// **Log only.** Never drive fanout from it: an export that failed
    /// leaves it empty (the startup sweep logs those rows later), and the
    /// export is at least once. Fanout reads the organization's state.
    pub org_events: Vec<OrgEventRow>,
}

/// Delete `player_id` if `account_id` owns it, then export the trigger's
/// audit rows for this transaction to the log.
///
/// The character row is locked first, filtered by `account_id`, so another
/// account's request locks nothing and returns `deleted = false`. The
/// DELETE then fires the `sgw_player` BEFORE DELETE trigger, which locks
/// the character's organizations before the member rows cascade.
///
/// A failed export is logged at WARN and does not fail the delete: the rows
/// stay unstamped and the next startup sweep logs them.
pub async fn delete_character(
    pool: &PgPool,
    player_id: i32,
    account_id: i32,
) -> Result<CharacterDeletion, sqlx::Error> {
    let mut tx = pool.begin().await?;
    // The names are read with the lock and before the delete, so the lines
    // below can still name a character that no longer exists.
    let owned: Option<(String, Option<String>)> = sqlx::query_as(
        "SELECT p.player_name, a.account_name FROM sgw_player p          LEFT JOIN account a ON a.account_id = p.account_id          WHERE p.player_id = $1 AND p.account_id = $2 FOR UPDATE OF p",
    )
    .bind(player_id)
    .bind(account_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((player_name, account_name)) = owned else {
        tx.rollback().await?;
        tracing::debug!(
            target: "org",
            event = "delete_character",
            player_id, // nt:id-only no owned character matched, so no name was read
            account_id, // nt:id-only no owned character matched, so no name was read
            rows_affected = 0u64,
            "Character delete matched no owned character"
        );
        return Ok(CharacterDeletion {
            deleted: false,
            org_events: Vec::new(),
        });
    };
    // What the DELETE is about to destroy, counted before the cascade and
    // the Black Market trigger (which clears the held bids) run. Telemetry
    // must not block the delete: a failed count is logged and the delete
    // goes on without the counts.
    let cascade = match cascade_counts(&mut tx, player_id).await {
        Ok(counts) => Some(counts),
        Err(e) => {
            tracing::warn!(
                target: "org",
                event = "character.delete_cascade_count_failed",
                player_id,
                player_name = player_name.as_str(),
                account_id,
                account_name = account_name.as_deref(),
                error = %e,
                "Character delete could not count what it destroys; deleting without the counts"
            );
            None
        }
    };
    let deleted = sqlx::query("DELETE FROM sgw_player WHERE player_id = $1 AND account_id = $2")
        .bind(player_id)
        .bind(account_id)
        .execute(&mut *tx)
        .await?;
    let tx_id = current_tx_id(&mut tx).await?;
    tx.commit().await?;
    if let Some(counts) = &cascade {
        log_cascade(
            counts,
            player_id,
            &player_name,
            account_id,
            account_name.as_deref(),
        );
    }

    let org_events = match export_committed(pool, tx_id, ExportSource::CharacterDelete).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(
                target: "org",
                event = "org_events_export",
                reason = "db_error",
                player_id,
                player_name = player_name.as_str(),
                account_id,
                account_name = account_name.as_deref(),
                error = %e,
                "Character deleted, but its organization events were not exported; the startup sweep will log them"
            );
            Vec::new()
        }
    };
    tracing::debug!(
        target: "org",
        event = "delete_character",
        player_id,
        player_name = player_name.as_str(),
        account_id,
        account_name = account_name.as_deref(),
        rows_affected = deleted.rows_affected(),
        org_events = org_events.len(),
        "Character deleted"
    );
    Ok(CharacterDeletion {
        deleted: deleted.rows_affected() > 0,
        org_events,
    })
}

/// What deleting one character destroys beyond the character row itself.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CascadeCounts {
    /// Listings the character sold (open or settled); the cascade removes them.
    auctions_destroyed: i64,
    /// Held cash on open auctions where the character is the standing bidder.
    /// The Black Market trigger clears these bids, so the cash is lost.
    held_cash_destroyed: i64,
    /// Open auctions the character held a bid on (the bids the trigger clears).
    bids_held_lost: i64,
    /// Mail addressed to the character; the cascade removes it.
    mail_destroyed: i64,
    /// Items still in escrow in that mail; the cascade removes them.
    mail_attachments_destroyed: i64,
    /// Mail addressed to the character that still carries an unpaid COD.
    cod_mails: i64,
    /// Cash on the character's non-COD mail; the cascade removes it.
    mail_cash_destroyed: i64,
    /// Bid-history rows (`sgw_auction_bid`) the character placed.
    bid_history_rows: i64,
    /// `sequence_id`s of the listings the character sold, comma-joined, at
    /// most 50 (see `truncated`).
    auction_sequence_ids: String,
    /// `item_id`s of the listings the character sold, in the same order.
    auction_item_ids: String,
    /// `sequence_id`s of the open auctions the character holds a bid on.
    held_bid_auction_ids: String,
    /// The held bids' amounts, in the same order as `held_bid_auction_ids`.
    held_bid_amounts: String,
    /// `mail_id`s of the character's mail, at most 50.
    mail_ids: String,
    /// `item_id`s of the escrowed items in that mail, at most 50.
    mail_attachment_item_ids: String,
    /// `true` when any list above was capped at 50 entries.
    truncated: bool,
}

/// Count what the character's delete will destroy, inside the delete
/// transaction, with the character row already locked.
///
/// The counts are read at READ COMMITTED before the DELETE, so a mail that
/// expires between this read and the cascade can make them differ slightly.
/// The reads run in a savepoint, so a failed read leaves the transaction
/// usable for the DELETE.
async fn cascade_counts(
    conn: &mut sqlx::PgConnection,
    player_id: i32,
) -> Result<CascadeCounts, sqlx::Error> {
    sqlx::query("SAVEPOINT cascade_counts")
        .execute(&mut *conn)
        .await?;
    match read_cascade_counts(&mut *conn, player_id).await {
        Ok(counts) => {
            sqlx::query("RELEASE SAVEPOINT cascade_counts")
                .execute(&mut *conn)
                .await?;
            Ok(counts)
        }
        Err(e) => {
            sqlx::query("ROLLBACK TO SAVEPOINT cascade_counts")
                .execute(&mut *conn)
                .await?;
            Err(e)
        }
    }
}

/// The reads behind [`cascade_counts`]: the counts, then the capped lists.
async fn read_cascade_counts(
    conn: &mut sqlx::PgConnection,
    player_id: i32,
) -> Result<CascadeCounts, sqlx::Error> {
    // `MAIL_COD` is flag bit 2 (`cell::mail::codes::flags`); a paid COD
    // clears it.
    let (auctions, held_cash, bids, mail, attachments, cod, mail_cash, bid_rows): (
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
        i64,
    ) = sqlx::query_as(
        "SELECT \
            (SELECT COUNT(*) FROM sgw_auction WHERE seller_id = $1), \
            (SELECT COALESCE(SUM(current_bid), 0)::bigint FROM sgw_auction \
              WHERE status = 0 AND current_bidder = $1 AND current_bid > 0), \
            (SELECT COUNT(*) FROM sgw_auction \
              WHERE status = 0 AND current_bidder = $1 AND current_bid > 0), \
            (SELECT COUNT(*) FROM sgw_gate_mail WHERE character_id = $1), \
            (SELECT COUNT(*) FROM sgw_gate_mail_item i \
              JOIN sgw_gate_mail m ON m.mail_id = i.mail_id \
              WHERE m.character_id = $1), \
            (SELECT COUNT(*) FROM sgw_gate_mail \
              WHERE character_id = $1 AND (flags & 2) <> 0), \
            (SELECT COALESCE(SUM(cash), 0)::bigint FROM sgw_gate_mail \
              WHERE character_id = $1 AND (flags & 2) = 0), \
            (SELECT COUNT(*) FROM sgw_auction_bid WHERE bidder_id = $1)",
    )
    .bind(player_id)
    .fetch_one(&mut *conn)
    .await?;
    // Each list is ordered the same way its count is taken, and capped by
    // the `LIMIT 50` in the SQL (`LIST_CAP`); the held bids share one order
    // so amounts line up with ids.
    let (sold_ids, sold_items, held_ids, held_amounts, mail_ids, attachment_ids): (
        String,
        String,
        String,
        String,
        String,
        String,
    ) = sqlx::query_as(
        "SELECT \
            array_to_string(ARRAY(SELECT sequence_id FROM sgw_auction \
              WHERE seller_id = $1 ORDER BY sequence_id LIMIT 50), ','), \
            array_to_string(ARRAY(SELECT item_id FROM sgw_auction \
              WHERE seller_id = $1 ORDER BY sequence_id LIMIT 50), ','), \
            array_to_string(ARRAY(SELECT sequence_id FROM sgw_auction \
              WHERE status = 0 AND current_bidder = $1 AND current_bid > 0 \
              ORDER BY sequence_id LIMIT 50), ','), \
            array_to_string(ARRAY(SELECT current_bid FROM sgw_auction \
              WHERE status = 0 AND current_bidder = $1 AND current_bid > 0 \
              ORDER BY sequence_id LIMIT 50), ','), \
            array_to_string(ARRAY(SELECT mail_id FROM sgw_gate_mail \
              WHERE character_id = $1 ORDER BY mail_id LIMIT 50), ','), \
            array_to_string(ARRAY(SELECT i.item_id FROM sgw_gate_mail_item i \
              JOIN sgw_gate_mail m ON m.mail_id = i.mail_id \
              WHERE m.character_id = $1 ORDER BY m.mail_id, i.item_id LIMIT 50), ',')",
    )
    .bind(player_id)
    .fetch_one(&mut *conn)
    .await?;
    Ok(CascadeCounts {
        auctions_destroyed: auctions,
        held_cash_destroyed: held_cash,
        bids_held_lost: bids,
        mail_destroyed: mail,
        mail_attachments_destroyed: attachments,
        cod_mails: cod,
        mail_cash_destroyed: mail_cash,
        bid_history_rows: bid_rows,
        auction_sequence_ids: sold_ids,
        auction_item_ids: sold_items,
        held_bid_auction_ids: held_ids,
        held_bid_amounts: held_amounts,
        mail_ids,
        mail_attachment_item_ids: attachment_ids,
        truncated: [auctions, bids, mail, attachments]
            .iter()
            .any(|n| *n > LIST_CAP),
    })
}

/// The most entries any one id list carries; more sets `truncated`.
const LIST_CAP: i64 = 50;

/// Log what a committed delete destroyed: WARN when held cash, mail
/// attachments, listings or mail cash were lost (the player can feel it),
/// else DEBUG.
fn log_cascade(
    c: &CascadeCounts,
    player_id: i32,
    player_name: &str,
    account_id: i32,
    account_name: Option<&str>,
) {
    if c.held_cash_destroyed != 0
        || c.mail_attachments_destroyed != 0
        || c.auctions_destroyed != 0
        || c.mail_cash_destroyed != 0
    {
        tracing::warn!(
            target: "org",
            event = "character.delete_cascade",
            player_id,
            player_name,
            account_id,
            account_name,
            auctions_destroyed = c.auctions_destroyed,
            held_cash_destroyed = c.held_cash_destroyed,
            bids_held_lost = c.bids_held_lost,
            mail_destroyed = c.mail_destroyed,
            mail_attachments_destroyed = c.mail_attachments_destroyed,
            cod_mails = c.cod_mails,
            mail_cash_destroyed = c.mail_cash_destroyed,
            bid_history_rows = c.bid_history_rows,
            auction_sequence_ids = c.auction_sequence_ids.as_str(),
            auction_item_ids = c.auction_item_ids.as_str(),
            held_bid_auction_ids = c.held_bid_auction_ids.as_str(),
            held_bid_amounts = c.held_bid_amounts.as_str(),
            mail_ids = c.mail_ids.as_str(),
            mail_attachment_item_ids = c.mail_attachment_item_ids.as_str(),
            truncated = c.truncated,
            "Character delete destroyed listings, held cash, mail cash or mail attachments"
        );
    } else {
        tracing::debug!(
            target: "org",
            event = "character.delete_cascade",
            player_id,
            player_name,
            account_id,
            account_name,
            auctions_destroyed = c.auctions_destroyed,
            held_cash_destroyed = c.held_cash_destroyed,
            bids_held_lost = c.bids_held_lost,
            mail_destroyed = c.mail_destroyed,
            mail_attachments_destroyed = c.mail_attachments_destroyed,
            cod_mails = c.cod_mails,
            mail_cash_destroyed = c.mail_cash_destroyed,
            bid_history_rows = c.bid_history_rows,
            auction_sequence_ids = c.auction_sequence_ids.as_str(),
            auction_item_ids = c.auction_item_ids.as_str(),
            held_bid_auction_ids = c.held_bid_auction_ids.as_str(),
            held_bid_amounts = c.held_bid_amounts.as_str(),
            mail_ids = c.mail_ids.as_str(),
            mail_attachment_item_ids = c.mail_attachment_item_ids.as_str(),
            truncated = c.truncated,
            "Character delete destroyed no listings, held cash, mail cash or mail attachments"
        );
    }
}
