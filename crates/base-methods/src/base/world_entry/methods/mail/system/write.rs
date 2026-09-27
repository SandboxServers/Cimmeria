//! The SQL of a server-written mail: the item checks and locks, the
//! recipient lock, the mail row and its escrow row. Shared by
//! [`super::send_system_mail_tx`] and the GM's COD mail (`mail/gm.rs`),
//! which differ only in the header: a COD mail has the GM as `sender_id`,
//! `MAIL_COD` and the price in `cash`.

use sqlx::PgConnection;

use super::super::expiry::expires_at;
use super::super::send::escrow::{escrow_item, SourceItem};
use super::{
    SystemEscrow, SystemItem, SystemMailError, SERVER_HELD_CONTAINERS, SYSTEM_SOURCE_CHARACTER_ID,
};
use crate::base::crafting::inventory_locks::take_inventory_locks;
use crate::cell::mail::codes::flags::MAIL_ARCHIVE;

/// The `sgw_gate_mail` columns of a server-written mail.
#[derive(Debug, Clone, Copy)]
pub(in super::super) struct MailHeader<'a> {
    pub(in super::super) recipient_player_id: i32,
    /// `None` for system mail (not returnable, D-SS10).
    pub(in super::super) sender_id: Option<i32>,
    pub(in super::super) sender_name: &'a str,
    pub(in super::super) subject: &'a str,
    pub(in super::super) body: &'a str,
    /// Gift cash, or the COD price when `flags` has `MAIL_COD`.
    pub(in super::super) cash: i64,
    pub(in super::super) flags: i32,
}

/// What [`write_mail`] wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in super::super) struct Written {
    pub(in super::super) mail_id: i32,
    pub(in super::super) item: Option<SystemEscrow>,
    /// The recipient's open (not archived, not quarantined) mail, this one
    /// included.
    pub(in super::super) recipient_open_mail: i64,
}

/// A minted item's escrow row, with `grant_item`'s instance defaults
/// (`inventory/grant/grant_item.rs`): durability 100, the template's
/// charges (also the loaded ammo), its default ammo type and ammo types,
/// not bound, no flags. `$1` mail, `$2` type, `$3` quantity, `$4` the
/// system source id, `$5` escrow time.
const MINT_ESCROW_SQL: &str = "INSERT INTO sgw_gate_mail_item \
     (mail_id, item_id, type_id, stack_size, charges, durability, flags, bound, \
      ammo, cur_ammo_type, ammo_type, ammo_types, source_character_id, escrowed_at) \
     SELECT $1, nextval('sgw_inventory_item_id_seq'), ri.item_id, $3, ri.charges, 100, 0, \
            false, ri.charges, 0, \
            COALESCE(ri.default_ammo_type, 'AMMO_NONE'::resources.\"EAmmoType\"), \
            ri.ammo_types, $4, $5 \
     FROM resources.items ri WHERE ri.item_id = $2 \
     RETURNING item_id";

/// An item the mail will carry, checked and (for an existing row) locked,
/// before any player row is locked.
enum Prepared {
    None,
    Minted { type_id: i32, qty: i32 },
    Existing { owner: i32, source: SourceItem },
}

/// Write one mail, and its escrow row if it carries an item, on `conn`
/// (the caller's transaction). Nothing is committed.
///
/// Order: the item (the owner's inventory advisory locks and the row
/// `FOR UPDATE`, or the template read), then the recipient's `sgw_player`
/// row `FOR UPDATE`, then the inserts: the shared inventory lock order,
/// the same as the player send path (`send/deliver.rs`).
pub(in super::super) async fn write_mail(
    conn: &mut PgConnection,
    header: &MailHeader<'_>,
    item: SystemItem,
    now: i32,
) -> Result<Written, SystemMailError> {
    let prepared = prepare_item(&mut *conn, item, header.recipient_player_id).await?;

    let recipient: Option<i32> =
        sqlx::query_scalar("SELECT player_id FROM sgw_player WHERE player_id = $1 FOR UPDATE")
            .bind(header.recipient_player_id)
            .fetch_optional(&mut *conn)
            .await?;
    if recipient.is_none() {
        return Err(SystemMailError::RecipientNotFound);
    }

    let mail_id: i32 = sqlx::query_scalar(
        "INSERT INTO sgw_gate_mail \
            (character_id, sender_id, sender_name, subject, message, cash, \
             sent_time, read_time, flags, item_id, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 0, $8, NULL, $9) \
         RETURNING mail_id",
    )
    .bind(header.recipient_player_id)
    .bind(header.sender_id)
    .bind(header.sender_name)
    .bind(header.subject)
    .bind(header.body)
    .bind(header.cash)
    .bind(now)
    .bind(header.flags)
    .bind(expires_at(now))
    .fetch_one(&mut *conn)
    .await?;

    let item = match prepared {
        Prepared::None => None,
        Prepared::Minted { type_id, qty } => {
            let item_id: i32 = sqlx::query_scalar(MINT_ESCROW_SQL)
                .bind(mail_id)
                .bind(type_id)
                .bind(qty)
                .bind(SYSTEM_SOURCE_CHARACTER_ID)
                .bind(now)
                .fetch_one(&mut *conn)
                .await?;
            Some(SystemEscrow {
                item_id,
                type_id,
                stack_size: qty,
                source_character_id: SYSTEM_SOURCE_CHARACTER_ID,
            })
        }
        Prepared::Existing { owner, source } => {
            // The whole row: the quantity is its stack, so `escrow_item`
            // moves it with its id and every instance column.
            let moved =
                escrow_item(&mut *conn, mail_id, owner, &source, source.stack_size, now).await?;
            Some(SystemEscrow {
                item_id: moved.escrow_item_id,
                type_id: moved.type_id,
                stack_size: moved.quantity,
                source_character_id: owner,
            })
        }
    };

    let recipient_open_mail: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sgw_gate_mail \
         WHERE character_id = $1 AND (flags & $2) = 0 AND NOT quarantined",
    )
    .bind(header.recipient_player_id)
    .bind(MAIL_ARCHIVE)
    .fetch_one(&mut *conn)
    .await?;

    Ok(Written {
        mail_id,
        item,
        recipient_open_mail,
    })
}

/// Check the item and, for an existing row, lock it.
async fn prepare_item(
    conn: &mut PgConnection,
    item: SystemItem,
    recipient_player_id: i32,
) -> Result<Prepared, SystemMailError> {
    match item {
        SystemItem::None => Ok(Prepared::None),
        SystemItem::Minted { type_id, qty } => {
            let max: Option<i32> =
                sqlx::query_scalar("SELECT max_stack_size FROM resources.items WHERE item_id = $1")
                    .bind(type_id)
                    .fetch_optional(&mut *conn)
                    .await?;
            let Some(max) = max else {
                return Err(SystemMailError::UnknownItemType);
            };
            // A template with a stack size below 1 still holds one item.
            let max_stack_size = max.max(1);
            if qty > max_stack_size {
                return Err(SystemMailError::QuantityExceedsStack { max_stack_size });
            }
            Ok(Prepared::Minted { type_id, qty })
        }
        SystemItem::ExistingInstance {
            item_id,
            owner_player_id,
        } => {
            // The owner is needed for the advisory lock, which must come
            // before the row lock; read it, lock, then re-read the row
            // under its lock and check it again, in case it moved between.
            let found: Option<(i32, i32)> = sqlx::query_as(
                "SELECT character_id, container_id FROM sgw_inventory WHERE item_id = $1",
            )
            .bind(item_id)
            .fetch_optional(&mut *conn)
            .await?;
            let Some((owner, container_id)) = found else {
                return Err(SystemMailError::ItemNotFound);
            };
            // A wrong id (an off-by-one listing, a stale cache) must not
            // take another seller's row.
            if owner != owner_player_id {
                return Err(SystemMailError::ItemOwnerMismatch { owner });
            }
            require_server_held(owner, container_id)?;
            take_inventory_locks(&mut *conn, owner, &[container_id]).await?;
            let source: Option<SourceItem> = sqlx::query_as(
                "SELECT item_id, type_id, stack_size, container_id, bound \
                 FROM sgw_inventory WHERE character_id = $1 AND item_id = $2 FOR UPDATE",
            )
            .bind(owner)
            .bind(item_id)
            .fetch_optional(&mut *conn)
            .await?;
            let Some(source) = source else {
                return Err(SystemMailError::ItemNotFound);
            };
            require_server_held(owner, source.container_id)?;
            // A bound item never changes hands, the rule every player path
            // enforces: it may only go back to its owner (a cancelled or
            // expired listing).
            if source.bound && recipient_player_id != owner {
                return Err(SystemMailError::ItemBound { owner });
            }
            Ok(Prepared::Existing { owner, source })
        }
    }
}

/// Refuse a row a player holds. The system is a trusted sender, so the
/// player send path's allowlist (main bag) does not apply; this is the
/// check that stands in for it.
fn require_server_held(owner: i32, container_id: i32) -> Result<(), SystemMailError> {
    if SERVER_HELD_CONTAINERS.contains(&container_id) {
        Ok(())
    } else {
        Err(SystemMailError::ItemNotServerHeld {
            container_id,
            owner,
        })
    }
}
