//! Black Market / Auction House (`SGWBlackMarket`) base-side support.
//!
//! `SGWBlackMarket` is a ServerOnly BASE entity. Client RPCs land at the cell
//! methods 61–66 (`cimmeria_cell_methods::cell::cell_methods::black_market`),
//! which decode them with the shared codec, check that 62-64 come from a
//! player at an auctioneer, and forward to the base via
//! `CellToBaseMsg::BlackMarket` (routed by `cimmeria-base-world-entry`'s
//! `cell_dispatch`). The base side (here) owns all DB, escrow, cash and mail
//! work and sends the `onBM*` (client indices 90–95) replies back to the
//! player's own client. `cimmeria-base` spawns the boot seed and the expiry
//! sweep at startup. The contract and its decisions are in
//! `docs/analysis/black-market/README.md`.
//!
//! Module map:
//! - [`types`]   — the `AuctionRow` model, status constants, the listing cap
//!   and listable bags.
//! - [`wire`]    — `onBM*` arguments through the shared codec, durations,
//!   the time-left bucket, the next minimum bid, and search paging.
//! - [`validate`] — pure accept/reject rules, each refusal a `BMError`.
//! - [`telemetry`] — the `bm.<transition>` events, refusal rows and the
//!   `bm_outcome_total` counter.
//! - [`helpers`] — the clock and `adjust_player_cash`.
//! - [`escrow`]  — moving the listed row into container 18 and back out.
//! - [`payout_mail`] — `send_mail_to_player` and the settlement mail texts.
//! - [`send`]    — `onBM*` and item-update sends.
//! - [`search`]  — the `BMSearch` handler and query.
//! - [`create`] / [`bid`] / [`cancel`] — the create/bid/cancel state machine.
//! - [`settle`]  — settling one auction (sweep and buyout).
//! - [`sweep`]   — the periodic expiry-settlement background task.
//! - [`seed`]    — the boot seed of system-seller listings.

use sqlx::PgPool;

pub mod bid;
pub mod cancel;
pub mod create;
pub mod escrow;
pub mod helpers;
pub mod payout_mail;
pub mod search;
pub mod seed;
pub mod send;
pub mod settle;
pub mod sweep;
pub mod telemetry;
pub mod types;
pub mod validate;
pub mod wire;

pub use types::BMSearchOptions;

#[cfg(test)]
mod tests;

/// A player's display name from `sgw_player` (S8: offline sellers too).
/// Empty when the row is gone or the read fails; it is a label only.
async fn player_name(pool: &PgPool, player_id: i32) -> String {
    sqlx::query_scalar::<_, String>("SELECT player_name FROM sgw_player WHERE player_id = $1")
        .bind(player_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
}
