//! The two reads and the one write behind a vault expansion.
//!
//! # The `bank_slots` grow-only invariant
//!
//! BV-03 reads `sgw_player.bank_slots` **without a lock**, in the move
//! transaction (`move_/bank_rules.rs`) and in `reservable_slots`
//! (`vendor/serializers.rs`). That is only safe while `bank_slots` never
//! decreases: a stale read can then only refuse a slot the player is about
//! to own, never accept one the player is about to lose. [`persist_expansion`]
//! is the only writer of `bank_slots` besides the column default, and it only
//! ever adds [`VAULT_EXPAND_STEP`], capped below the ceiling in the same
//! statement. Any future writer that could shrink it must take a lock where
//! BV-03 reads it (`FOR SHARE`, or the move lock) first.

use cimmeria_entity::cell_entity::ExpansionOffer;
use cimmeria_entity::inventory::{bag_max_slots, INV_BANK};
use cimmeria_wire::cell::vault::VAULT_EXPAND_STEP;
use sqlx::PgPool;

/// The vault's ceiling: container 17's capacity, 100 (D-BV02). The
/// `bank_slots_sanity` CHECK and the `bank_expansion_price` CHECK hold the
/// same bound in the schema.
pub const VAULT_CEILING: i16 = bag_max_slots(INV_BANK) as i16;

/// A player's vault size, cash, and the price of their next step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpansionState {
    pub bank_slots: i16,
    pub naquadah: i32,
    /// `bank_expansion_price.price_naquadah` for `bank_slots + 10`; `None`
    /// at the ceiling or when the seed has no row for that step.
    pub next_price: Option<i32>,
}

/// One purchase attempt's result. Every non-`Expanded` outcome changed
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpandOutcome {
    /// The step was bought.
    Expanded {
        bank_slots_after: i16,
        cash_after: i32,
        price: i32,
    },
    /// `bank_slots` is no longer the size the offer was made at: this
    /// offer already bought its step (a double send of one click).
    Replay { state: ExpansionState },
    /// The vault is already at the ceiling.
    AtCeiling { state: ExpansionState },
    /// No price row for the next step: a seed gap, not a player decision.
    PriceMissing { state: ExpansionState },
    /// The step's price is no longer the price the player was shown (the
    /// seed was retuned while the offer was open).
    PriceChanged { state: ExpansionState },
    /// Not enough naquadah for the next step.
    InsufficientCash { state: ExpansionState, price: i32 },
    /// Every condition holds on the follow-up read, so the row changed
    /// between the write and the read (cash arrived in between).
    RowChanged { state: ExpansionState },
    /// No `sgw_player` row for the character.
    PlayerMissing,
}

/// Read the player's vault size, cash and next price. A plain read with no
/// lock: it only decides what to show or log, never what to write.
pub async fn read_expansion_state(
    pool: &PgPool,
    player_id: i32,
) -> sqlx::Result<Option<ExpansionState>> {
    let row: Option<(i16, i32, Option<i32>)> = sqlx::query_as(
        "SELECT p.bank_slots, p.naquadah, x.price_naquadah \
           FROM sgw_player AS p \
           LEFT JOIN resources.bank_expansion_price AS x \
             ON x.to_slots = p.bank_slots + $2 \
          WHERE p.player_id = $1",
    )
    .bind(player_id)
    .bind(VAULT_EXPAND_STEP)
    .fetch_optional(pool)
    .await?;
    Ok(
        row.map(|(bank_slots, naquadah, next_price)| ExpansionState {
            bank_slots,
            naquadah,
            next_price,
        }),
    )
}

/// Buy the step `offer` showed, in **one statement**: `bank_slots += 10`
/// and `naquadah -= price` together, only while the row is still at the
/// offered size, below the ceiling, with a price row for the next step
/// still at the offered price, and the cash to pay it.
///
/// The `bank_slots = from_slots` guard is the replay key. Two sends for
/// one offer both name the same size; the second waits on the first's row
/// lock, re-evaluates its `WHERE` against the committed row (now 10
/// larger), and matches nothing. So one click is charged once, and the
/// grow-only invariant (module docs) holds by construction: the only
/// assignment is `bank_slots + 10`. The price guard means the player is
/// never charged a price they were not shown.
///
/// When the guards hold the row back, a follow-up read classifies why.
/// Nothing is written either way, so that read needs no lock.
pub async fn persist_expansion(
    pool: &PgPool,
    player_id: i32,
    offer: ExpansionOffer,
) -> sqlx::Result<ExpandOutcome> {
    let bought: Option<(i16, i32, i32)> = sqlx::query_as(
        "UPDATE sgw_player AS p \
            SET bank_slots = p.bank_slots + $3, \
                naquadah = p.naquadah - x.price_naquadah \
           FROM resources.bank_expansion_price AS x \
          WHERE p.player_id = $1 \
            AND p.bank_slots = $2 \
            AND p.bank_slots < $4 \
            AND x.to_slots = p.bank_slots + $3 \
            AND x.price_naquadah = $5 \
            AND p.naquadah >= x.price_naquadah \
        RETURNING p.bank_slots, p.naquadah, x.price_naquadah",
    )
    .bind(player_id)
    .bind(offer.from_slots)
    .bind(VAULT_EXPAND_STEP)
    .bind(VAULT_CEILING)
    .bind(offer.price)
    .fetch_optional(pool)
    .await?;
    if let Some((bank_slots_after, cash_after, price)) = bought {
        return Ok(ExpandOutcome::Expanded {
            bank_slots_after,
            cash_after,
            price,
        });
    }

    let Some(state) = read_expansion_state(pool, player_id).await? else {
        return Ok(ExpandOutcome::PlayerMissing);
    };
    Ok(classify_refusal(state, offer))
}

/// Why a purchase of `offer` matched no row, given the row as it is now.
/// The replay key is checked first: an offer for a size the vault has left
/// behind is a replay, whatever the cash.
pub fn classify_refusal(state: ExpansionState, offer: ExpansionOffer) -> ExpandOutcome {
    if state.bank_slots != offer.from_slots {
        return ExpandOutcome::Replay { state };
    }
    if state.bank_slots >= VAULT_CEILING {
        return ExpandOutcome::AtCeiling { state };
    }
    match state.next_price {
        None => ExpandOutcome::PriceMissing { state },
        Some(price) if price != offer.price => ExpandOutcome::PriceChanged { state },
        Some(price) if state.naquadah < price => ExpandOutcome::InsufficientCash { state, price },
        Some(_) => ExpandOutcome::RowChanged { state },
    }
}
