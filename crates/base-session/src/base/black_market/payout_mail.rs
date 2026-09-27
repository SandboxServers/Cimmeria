//! The Black Market's mail payouts: the one place auction settlement writes
//! `sgw_gate_mail`.
//!
//! The expiry sweep mails the winning bid to the seller, the item to the
//! buyer, and an unsold item back to its seller, through
//! [`send_mail_to_player`] and the subject/body texts below. Everything that
//! writes auction mail is in this file, so moving the payouts onto the
//! social-systems mail API (packet BM-02b in `docs/analysis/black-market/`)
//! replaces this module and leaves the settlement logic alone.

use sqlx::PgExecutor;

use super::helpers::now_unix_secs;

/// Mail subject/body for a sold auction's seller payout.
pub(super) const SOLD_SELLER_SUBJECT: &str = "Auction Sold";
pub(super) const SOLD_SELLER_BODY: &str =
    "Your Black Market auction sold. The winning bid is attached.";
/// Mail subject/body for a sold auction's buyer delivery.
pub(super) const SOLD_BUYER_SUBJECT: &str = "Auction Won";
pub(super) const SOLD_BUYER_BODY: &str = "You won a Black Market auction. Your item is attached.";
/// Mail subject/body for an unsold auction returned to the seller.
pub(super) const UNSOLD_SUBJECT: &str = "Auction Expired";
pub(super) const UNSOLD_BODY: &str =
    "Your Black Market auction expired with no bids. Your item is returned.";
/// Mail `sender_name` used for all system-generated auction mail.
pub(super) const BM_SENDER_NAME: &str = "Black Market";

/// Insert one `sgw_gate_mail` row delivering cash and/or an item to a recipient.
///
/// `cash` lands in the `cash` column (BIGINT); `item_id`, when `Some`, is the
/// `sgw_inventory.item_id` instance id attached to the mail. `flags` is written
/// verbatim. `sent_time` is stamped now; `read_time` is 0 (unread). Returns the
/// freshly-allocated `mail_id`.
///
/// Mirrors the column set the mail read/list path expects (see
/// `cimmeria-base-methods`'s `base::world_entry::methods::mail`). `sender_id` is left NULL (system mail);
/// `sender_name` falls back to the column default.
pub async fn send_mail_to_player<'e, E>(
    exec: E,
    recipient_id: i32,
    cash: i64,
    item_id: Option<i32>,
    flags: i32,
    subject: &str,
    body: &str,
    sender_name: &str,
) -> Result<i32, sqlx::Error>
where
    E: PgExecutor<'e>,
{
    let now = now_unix_secs();
    sqlx::query_scalar::<_, i32>(
        "INSERT INTO sgw_gate_mail \
            (character_id, sender_id, subject, message, cash, sent_time, read_time, \
             flags, item_id, sender_name) \
         VALUES ($1, NULL, $2, $3, $4, $5, 0, $6, $7, $8) \
         RETURNING mail_id",
    )
    .bind(recipient_id)
    .bind(subject)
    .bind(body)
    .bind(cash)
    .bind(now)
    .bind(flags)
    .bind(item_id)
    .bind(sender_name)
    .fetch_one(exec)
    .await
}
